use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tea_compiler::{
    CompileOptions, Compiler, Diagnostic as CompilerDiagnostic, DiagnosticLevel,
    InterpolatedStringPart, Keyword, Lexer, Module, ModuleAliasBinding, SourceFile, SourceId,
    Statement, TokenKind,
};
use tokio::{
    sync::Mutex,
    task,
    time::{sleep, Duration},
};
use tokio_util::sync::CancellationToken;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionResponse,
    Diagnostic as LspDiagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, HoverProviderCapability,
    InitializeParams, InitializeResult, InitializedParams, Location, MarkupContent, MarkupKind,
    MessageType, OneOf, Position, Range, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url, WorkDoneProgressOptions,
};
use tower_lsp::{async_trait, Client, LanguageServer, LspService, Server};

macro_rules! range_from_span {
    ($span:expr) => {{
        let span = $span;
        let start_line = span.line.saturating_sub(1) as u32;
        let start_col = span.column.saturating_sub(1) as u32;
        let mut end_line = span.end_line.saturating_sub(1) as u32;
        let mut end_col = span.end_column.saturating_sub(1) as u32;

        if end_line < start_line || (end_line == start_line && end_col < start_col) {
            end_line = start_line;
            end_col = start_col;
        }
        if end_line == start_line && end_col == start_col {
            end_col = end_col.saturating_add(1);
        }

        Range {
            start: Position {
                line: start_line,
                character: start_col,
            },
            end: Position {
                line: end_line,
                character: end_col,
            },
        }
    }};
}

#[derive(Debug, Clone)]
struct DocumentState {
    source_id: SourceId,
    path: PathBuf,
    text: String,
    version: i32,
    analysis: Option<DocumentAnalysis>,
    pending: Option<PendingCompile>,
    dependencies: HashSet<PathBuf>,
}

#[derive(Debug)]
struct ServerState {
    next_source_id: u32,
    documents: HashMap<Url, DocumentState>,
    next_task_id: u64,
    dependents: HashMap<PathBuf, HashSet<Url>>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            next_source_id: 0,
            documents: HashMap::new(),
            next_task_id: 0,
            dependents: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct DocumentAnalysis {
    symbols: Vec<SymbolInfo>,
    module_aliases: HashMap<String, tea_compiler::ModuleAliasBinding>,
    argument_expectations: Vec<ArgumentExpectation>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tea_compiler::{Compilation, CompileOptions, Compiler, SourceFile, SourceId};

    fn compile_source(contents: &str) -> Compilation {
        let mut compiler = Compiler::new(CompileOptions::default());
        let source = SourceFile::new(SourceId(0), "test.tea".into(), contents.to_string());
        compiler.compile(&source).expect("compilation to succeed")
    }

    #[test]
    fn std_module_member_contains_docstring() {
        let compilation = compile_source(
            r#"use debug = "std.debug"

def main() -> Nil
  debug.print("hello")
end
"#,
        );

        let analysis = collect_symbols(
            &compilation.module,
            &compilation.module_aliases,
            &compilation.binding_types,
            &compilation.argument_types,
        );

        let debug_binding = analysis
            .module_aliases
            .get("debug")
            .expect("debug alias to be present");

        assert_eq!(
            debug_binding.docstring.as_deref(),
            Some("Debug utilities such as printing."),
        );
        assert_eq!(
            debug_binding.export_docs.get("print"),
            Some(&"Write the string representation of a value to stderr.".to_string()),
        );

        let symbol_doc = analysis
            .symbols
            .iter()
            .find(|symbol| symbol.name == "print")
            .and_then(|symbol| symbol.docstring.clone());
        assert_eq!(
            symbol_doc.as_deref(),
            Some("Write the string representation of a value to stderr."),
        );
    }

    #[test]
    fn variable_docstring_appears_in_symbols() {
        let compilation = compile_source(
            r#"## Enable this to see the flag
var flag = false

def main() -> Bool
  flag
end
"#,
        );

        let analysis = collect_symbols(
            &compilation.module,
            &compilation.module_aliases,
            &compilation.binding_types,
            &compilation.argument_types,
        );

        let flag_symbol = analysis
            .symbols
            .iter()
            .find(|symbol| symbol.name == "flag")
            .expect("flag symbol to be collected");

        assert_eq!(
            flag_symbol.docstring.as_deref(),
            Some("Enable this to see the flag"),
        );
    }
}

#[derive(Debug, Clone)]
struct SymbolInfo {
    name: String,
    range: Range,
    kind: SymbolKind,
    type_desc: Option<String>,
    docstring: Option<String>,
}

#[derive(Debug, Clone)]
struct ArgumentExpectation {
    range: Range,
    type_desc: String,
}

#[derive(Debug, Clone)]
struct PendingCompile {
    id: u64,
    token: CancellationToken,
}

struct BlockingCompileOutput {
    analysis: Option<DocumentAnalysis>,
    dependencies: HashSet<PathBuf>,
    diagnostics: Vec<CompilerDiagnostic>,
    compile_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymbolKind {
    ModuleAlias,
    Struct,
    Enum,
    Function,
    Const,
    Variable,
    Parameter,
    Field,
    EnumVariant,
}

impl SymbolKind {
    fn label(self) -> &'static str {
        match self {
            SymbolKind::ModuleAlias => "module",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Function => "function",
            SymbolKind::Const => "const",
            SymbolKind::Variable => "variable",
            SymbolKind::Parameter => "parameter",
            SymbolKind::Field => "field",
            SymbolKind::EnumVariant => "enum variant",
        }
    }

    fn completion_kind(self) -> CompletionItemKind {
        match self {
            SymbolKind::ModuleAlias => CompletionItemKind::MODULE,
            SymbolKind::Struct => CompletionItemKind::STRUCT,
            SymbolKind::Enum => CompletionItemKind::ENUM,
            SymbolKind::Function => CompletionItemKind::FUNCTION,
            SymbolKind::Const => CompletionItemKind::CONSTANT,
            SymbolKind::Variable => CompletionItemKind::VARIABLE,
            SymbolKind::Parameter => CompletionItemKind::VARIABLE,
            SymbolKind::Field => CompletionItemKind::FIELD,
            SymbolKind::EnumVariant => CompletionItemKind::ENUM_MEMBER,
        }
    }
}

impl ServerState {
    fn allocate_source_id(&mut self) -> SourceId {
        let id = self.next_source_id;
        self.next_source_id = self.next_source_id.saturating_add(1);
        SourceId(id)
    }
}

struct TeaLanguageServer {
    client: Client,
    state: Arc<Mutex<ServerState>>,
}

impl Clone for TeaLanguageServer {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            state: self.state.clone(),
        }
    }
}

impl TeaLanguageServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(Mutex::new(ServerState::default())),
        }
    }

    async fn document_snapshot(&self, uri: &Url) -> Option<DocumentState> {
        let state = self.state.lock().await;
        state.documents.get(uri).cloned()
    }

    async fn collect_module_overrides(&self) -> HashMap<PathBuf, String> {
        let state = self.state.lock().await;
        state
            .documents
            .values()
            .map(|doc| (doc.path.clone(), doc.text.clone()))
            .collect()
    }

    async fn schedule_compile(&self, uri: Url, version: i32) {
        let (token, task_id) = {
            let mut state = self.state.lock().await;
            let task_id = state.next_task_id;
            state.next_task_id = state.next_task_id.saturating_add(1);
            let Some(doc) = state.documents.get_mut(&uri) else {
                return;
            };
            if let Some(pending) = doc.pending.take() {
                pending.token.cancel();
            }
            let token = CancellationToken::new();
            doc.pending = Some(PendingCompile {
                id: task_id,
                token: token.clone(),
            });
            (token, task_id)
        };

        let server = self.clone();
        task::spawn_local(async move {
            sleep(Duration::from_millis(150)).await;
            if token.is_cancelled() {
                return;
            }
            server
                .client
                .log_message(
                    MessageType::LOG,
                    format!("compile:start uri={uri} version={version} task={task_id}"),
                )
                .await;
            server
                .run_compile_task(uri, version, task_id, token.clone())
                .await;
        });
    }

    async fn run_compile_task(
        &self,
        uri: Url,
        version: i32,
        task_id: u64,
        token: CancellationToken,
    ) {
        {
            let state = self.state.lock().await;
            match state.documents.get(&uri) {
                Some(doc)
                    if doc.version == version
                        && doc
                            .pending
                            .as_ref()
                            .map(|pending| pending.id == task_id)
                            .unwrap_or(false) => {}
                _ => return,
            }
        }

        if token.is_cancelled() {
            return;
        }

        let _ = self.compile_and_publish(&uri, Some(&token)).await;

        self.client
            .log_message(
                MessageType::LOG,
                format!(
                    "compile:finish uri={} version={version} task={task_id}",
                    uri
                ),
            )
            .await;

        let mut state = self.state.lock().await;
        if let Some(doc) = state.documents.get_mut(&uri) {
            if doc.version == version {
                if let Some(pending) = doc.pending.as_ref() {
                    if pending.id == task_id {
                        doc.pending = None;
                    }
                }
            }
        }
    }

    async fn compile_and_publish(
        &self,
        uri: &Url,
        cancel: Option<&CancellationToken>,
    ) -> Result<()> {
        let (path, text, version, source_id) = {
            let state = self.state.lock().await;
            let Some(doc) = state.documents.get(uri) else {
                return Ok(());
            };
            (
                doc.path.clone(),
                doc.text.clone(),
                doc.version,
                doc.source_id,
            )
        };

        if Self::cancellation_requested(cancel) {
            return Ok(());
        }

        let source = SourceFile::new(source_id, path, text);
        let module_overrides = Arc::new(self.collect_module_overrides().await);
        let compile_output = self
            .blocking_compile(source, module_overrides.clone(), cancel)
            .await?;
        if Self::cancellation_requested(cancel) {
            self.client
                .log_message(MessageType::LOG, format!("compile:cancelled uri={uri}"))
                .await;
            return Ok(());
        }

        if Self::cancellation_requested(cancel) {
            return Ok(());
        }

        let Some(output) = compile_output else {
            return Ok(());
        };

        let BlockingCompileOutput {
            analysis,
            dependencies,
            diagnostics,
            compile_error,
        } = output;

        let mut diagnostics = diagnostics
            .iter()
            .map(convert_diagnostic)
            .collect::<Vec<_>>();

        let mut dependents_to_recompile: Vec<(Url, i32)> = Vec::new();
        {
            let mut state = self.state.lock().await;
            let doc_state = if let Some(doc) = state.documents.get_mut(uri) {
                doc.analysis = analysis;
                let doc_path = doc.path.clone();
                let old_deps = std::mem::take(&mut doc.dependencies);
                let new_deps = dependencies.clone();
                doc.dependencies = new_deps.clone();
                Some((doc_path, old_deps, new_deps))
            } else {
                None
            };

            if let Some((doc_path, old_deps, new_deps)) = doc_state {
                for dep in old_deps {
                    if let Some(set) = state.dependents.get_mut(&dep) {
                        set.remove(uri);
                        if set.is_empty() {
                            state.dependents.remove(&dep);
                        }
                    }
                }
                for dep in &new_deps {
                    state
                        .dependents
                        .entry(dep.clone())
                        .or_default()
                        .insert(uri.clone());
                }
                if let Some(set) = state.dependents.get(&doc_path) {
                    dependents_to_recompile = set
                        .iter()
                        .filter(|dependent_uri| *dependent_uri != uri)
                        .filter_map(|dependent_uri| {
                            state
                                .documents
                                .get(dependent_uri)
                                .map(|doc| (dependent_uri.clone(), doc.version))
                        })
                        .collect();
                }
            }
        }

        if let Some(message) = compile_error {
            if diagnostics.is_empty() {
                let range = extract_range_from_message(&message).unwrap_or(Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                });
                diagnostics.push(LspDiagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("tea-compiler".into()),
                    message,
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(version))
            .await;

        for (dependent_uri, dependent_version) in dependents_to_recompile {
            self.schedule_compile(dependent_uri, dependent_version)
                .await;
        }

        let mut processed = HashSet::new();
        for dependency in dependencies {
            if !processed.insert(dependency.clone()) {
                continue;
            }
            if Self::cancellation_requested(cancel) {
                return Ok(());
            }
            let is_open = {
                let state = self.state.lock().await;
                state.documents.values().any(|doc| doc.path == dependency)
            };
            if is_open {
                continue;
            }
            if let Err(err) = self
                .compile_dependency_and_publish(
                    dependency.clone(),
                    module_overrides.clone(),
                    cancel,
                )
                .await
            {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!(
                            "failed to compile dependency {}: {err}",
                            dependency.display()
                        ),
                    )
                    .await;
            }
        }

        Ok(())
    }

    async fn upsert_document(&self, uri: &Url, text: String, version: i32) -> Result<()> {
        let raw_path = uri
            .to_file_path()
            .map_err(|_| anyhow!("unsupported URI scheme for document: {uri}"))?;
        let path = raw_path.canonicalize().unwrap_or(raw_path.clone());

        let mut state = self.state.lock().await;
        if let Some(doc) = state.documents.get_mut(uri) {
            doc.path = path;
            doc.text = text;
            doc.version = version;
            if let Some(pending) = doc.pending.take() {
                pending.token.cancel();
            }
            return Ok(());
        }

        let source_id = state.allocate_source_id();
        state.documents.insert(
            uri.clone(),
            DocumentState {
                source_id,
                path,
                text,
                version,
                analysis: None,
                pending: None,
                dependencies: HashSet::new(),
            },
        );

        Ok(())
    }

    async fn remove_document(&self, uri: &Url) {
        let mut state = self.state.lock().await;
        if let Some(doc) = state.documents.remove(uri) {
            if let Some(pending) = doc.pending {
                pending.token.cancel();
            }
            for dep in doc.dependencies {
                if let Some(set) = state.dependents.get_mut(&dep) {
                    set.remove(uri);
                    if set.is_empty() {
                        state.dependents.remove(&dep);
                    }
                }
            }
        }
    }
}

fn collect_symbols(
    module: &Module,
    module_aliases: &HashMap<String, ModuleAliasBinding>,
    binding_types: &HashMap<tea_compiler::SourceSpan, String>,
    argument_types: &HashMap<tea_compiler::SourceSpan, String>,
) -> DocumentAnalysis {
    let mut alias_bindings = module_aliases.clone();
    for binding in alias_bindings.values_mut() {
        if binding.module_path.starts_with("std.") {
            if let Some(module) = tea_compiler::stdlib_find_module(&binding.module_path) {
                if binding.docstring.is_none() && !module.docstring.is_empty() {
                    binding.docstring = Some(module.docstring.to_string());
                }
                for function in module.functions {
                    if !function.docstring.is_empty() {
                        binding
                            .export_docs
                            .entry(function.name.to_string())
                            .or_insert_with(|| function.docstring.to_string());
                    }
                }
            }
        }
    }

    struct Collector<'a> {
        symbols: Vec<SymbolInfo>,
        binding_types: &'a HashMap<tea_compiler::SourceSpan, String>,
        module_aliases: &'a HashMap<String, ModuleAliasBinding>,
    }

    impl<'a> Collector<'a> {
        fn collect(mut self, module: &Module) -> Vec<SymbolInfo> {
            self.visit_statements(&module.statements);
            self.symbols
        }

        fn visit_statements(&mut self, statements: &[Statement]) {
            for statement in statements {
                self.visit_statement(statement);
            }
        }

        fn visit_statement(&mut self, statement: &Statement) {
            match statement {
                Statement::Use(use_stmt) => {
                    let range = range_from_span!(&use_stmt.alias.span);
                    let docstring = self
                        .module_aliases
                        .get(&use_stmt.alias.name)
                        .and_then(|binding| binding.docstring.clone());
                    self.symbols.push(SymbolInfo {
                        name: use_stmt.alias.name.clone(),
                        range,
                        kind: SymbolKind::ModuleAlias,
                        type_desc: None,
                        docstring,
                    });
                }
                Statement::Var(var_stmt) => {
                    let kind = if var_stmt.is_const {
                        SymbolKind::Const
                    } else {
                        SymbolKind::Variable
                    };
                    let binding_doc = var_stmt.docstring.clone();
                    for binding in &var_stmt.bindings {
                        let range = range_from_span!(&binding.span);
                        let type_desc = self.binding_types.get(&binding.span).cloned();
                        self.symbols.push(SymbolInfo {
                            name: binding.name.clone(),
                            range,
                            kind,
                            type_desc,
                            docstring: binding_doc.clone(),
                        });
                    }
                    for binding in &var_stmt.bindings {
                        if let Some(initializer) = &binding.initializer {
                            self.visit_expression(initializer);
                        }
                    }
                }
                Statement::Function(function_stmt) => {
                    let range = range_from_span!(&function_stmt.name_span);
                    let type_desc = self.binding_types.get(&function_stmt.name_span).cloned();
                    self.symbols.push(SymbolInfo {
                        name: function_stmt.name.clone(),
                        range,
                        kind: SymbolKind::Function,
                        type_desc,
                        docstring: function_stmt.docstring.clone(),
                    });
                    for parameter in &function_stmt.parameters {
                        let range = range_from_span!(&parameter.span);
                        let type_desc = self.binding_types.get(&parameter.span).cloned();
                        self.symbols.push(SymbolInfo {
                            name: parameter.name.clone(),
                            range,
                            kind: SymbolKind::Parameter,
                            type_desc,
                            docstring: None,
                        });
                    }
                    self.visit_statements(&function_stmt.body.statements);
                }
                Statement::Test(test_stmt) => {
                    self.visit_statements(&test_stmt.body.statements);
                }
                Statement::Struct(struct_stmt) => {
                    let range = range_from_span!(&struct_stmt.name_span);
                    self.symbols.push(SymbolInfo {
                        name: struct_stmt.name.clone(),
                        range,
                        kind: SymbolKind::Struct,
                        type_desc: None,
                        docstring: struct_stmt.docstring.clone(),
                    });
                    for field in &struct_stmt.fields {
                        let range = range_from_span!(&field.span);
                        self.symbols.push(SymbolInfo {
                            name: field.name.clone(),
                            range,
                            kind: SymbolKind::Field,
                            type_desc: None,
                            docstring: None,
                        });
                    }
                }
                Statement::Enum(enum_stmt) => {
                    let range = range_from_span!(&enum_stmt.name_span);
                    self.symbols.push(SymbolInfo {
                        name: enum_stmt.name.clone(),
                        range,
                        kind: SymbolKind::Enum,
                        type_desc: None,
                        docstring: enum_stmt.docstring.clone(),
                    });
                    for variant in &enum_stmt.variants {
                        let range = range_from_span!(&variant.span);
                        self.symbols.push(SymbolInfo {
                            name: variant.name.clone(),
                            range,
                            kind: SymbolKind::EnumVariant,
                            type_desc: None,
                            docstring: variant.docstring.clone(),
                        });
                    }
                }
                Statement::Conditional(cond_stmt) => {
                    self.visit_expression(&cond_stmt.condition);
                    self.visit_statements(&cond_stmt.consequent.statements);
                    if let Some(alternative) = &cond_stmt.alternative {
                        self.visit_statements(&alternative.statements);
                    }
                }
                Statement::Loop(loop_stmt) => {
                    match &loop_stmt.header {
                        tea_compiler::LoopHeader::For { pattern, iterator } => {
                            self.visit_expression(pattern);
                            self.visit_expression(iterator);
                        }
                        tea_compiler::LoopHeader::Condition(expr) => {
                            self.visit_expression(expr);
                        }
                    }
                    self.visit_statements(&loop_stmt.body.statements);
                }
                Statement::Return(ret_stmt) => {
                    if let Some(expr) = &ret_stmt.expression {
                        self.visit_expression(expr);
                    }
                }
                Statement::Expression(expr_stmt) => {
                    self.visit_expression(&expr_stmt.expression);
                }
            }
        }

        fn visit_expression(&mut self, expression: &tea_compiler::Expression) {
            use tea_compiler::ExpressionKind;

            match &expression.kind {
                ExpressionKind::Identifier(_) | ExpressionKind::Literal(_) => {}
                ExpressionKind::InterpolatedString(template) => {
                    for part in &template.parts {
                        if let InterpolatedStringPart::Expression(expr) = part {
                            self.visit_expression(expr);
                        }
                    }
                }
                ExpressionKind::List(expr) => {
                    for element in &expr.elements {
                        self.visit_expression(element);
                    }
                }
                ExpressionKind::Dict(expr) => {
                    for entry in &expr.entries {
                        self.visit_expression(&entry.value);
                    }
                }
                ExpressionKind::Unary(expr) => {
                    self.visit_expression(&expr.operand);
                }
                ExpressionKind::Binary(expr) => {
                    self.visit_expression(&expr.left);
                    self.visit_expression(&expr.right);
                }
                ExpressionKind::Call(expr) => {
                    self.visit_expression(&expr.callee);
                    for argument in &expr.arguments {
                        self.visit_expression(&argument.expression);
                    }
                }
                ExpressionKind::Member(expr) => {
                    if let ExpressionKind::Identifier(identifier) = &expr.object.kind {
                        if let Some(binding) = self.module_aliases.get(&identifier.name) {
                            let range = range_from_span!(&expr.property_span);
                            let type_desc = binding.export_types.get(&expr.property).cloned();
                            let mut docstring = binding.export_docs.get(&expr.property).cloned();
                            if docstring.as_deref().map(str::is_empty).unwrap_or(false) {
                                docstring = None;
                            }
                            self.symbols.push(SymbolInfo {
                                name: expr.property.clone(),
                                range,
                                kind: SymbolKind::Function,
                                type_desc,
                                docstring,
                            });
                        }
                    }
                    self.visit_expression(&expr.object);
                }
                ExpressionKind::Index(expr) => {
                    self.visit_expression(&expr.object);
                    self.visit_expression(&expr.index);
                }
                ExpressionKind::Range(expr) => {
                    self.visit_expression(&expr.start);
                    self.visit_expression(&expr.end);
                }
                ExpressionKind::Lambda(expr) => {
                    for parameter in &expr.parameters {
                        let range = range_from_span!(&parameter.span);
                        let type_desc = self.binding_types.get(&parameter.span).cloned();
                        self.symbols.push(SymbolInfo {
                            name: parameter.name.clone(),
                            range,
                            kind: SymbolKind::Parameter,
                            type_desc,
                            docstring: None,
                        });
                    }
                    self.visit_lambda_body(&expr.body);
                }
                ExpressionKind::Assignment(expr) => {
                    self.visit_expression(&expr.target);
                    self.visit_expression(&expr.value);
                }
                ExpressionKind::Grouping(inner) => {
                    self.visit_expression(inner);
                }
            }
        }

        fn visit_lambda_body(&mut self, body: &tea_compiler::LambdaBody) {
            match body {
                tea_compiler::LambdaBody::Expression(expr) => self.visit_expression(expr),
                tea_compiler::LambdaBody::Block(block) => self.visit_block(block),
            }
        }

        fn visit_block(&mut self, block: &tea_compiler::Block) {
            self.visit_statements(&block.statements);
        }
    }

    let collector = Collector {
        symbols: Vec::new(),
        binding_types,
        module_aliases: &alias_bindings,
    };

    let symbols = collector.collect(module);
    let argument_expectations = argument_types
        .iter()
        .map(|(span, ty)| ArgumentExpectation {
            range: adjust_suggestion_range(range_from_span!(span)),
            type_desc: ty.clone(),
        })
        .collect();

    DocumentAnalysis {
        symbols,
        module_aliases: alias_bindings,
        argument_expectations,
    }
}

fn symbol_at_position<'a>(
    analysis: &'a DocumentAnalysis,
    position: &Position,
) -> Option<&'a SymbolInfo> {
    analysis
        .symbols
        .iter()
        .find(|symbol| range_contains(&symbol.range, position))
}

fn symbol_by_name<'a>(analysis: &'a DocumentAnalysis, name: &str) -> Option<&'a SymbolInfo> {
    analysis.symbols.iter().find(|symbol| symbol.name == name)
}

fn adjust_suggestion_range(mut range: Range) -> Range {
    if range.start.line == range.end.line && range.start.character == range.end.character {
        range.end.character = range.end.character.saturating_add(1);
    }
    range
}

fn collect_dependencies(doc_path: &Path, module: &Module) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();
    for statement in &module.statements {
        if let Statement::Use(use_stmt) = statement {
            let module_path = use_stmt.module_path.as_str();
            if module_path.starts_with("std.") || module_path.starts_with("support.") {
                continue;
            }
            if let Some(resolved) = resolve_import_path(doc_path, module_path) {
                deps.insert(resolved);
            }
        }
    }
    deps
}

fn collect_dependencies_from_source(source: &SourceFile) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();
    let mut lexer = match Lexer::new(source) {
        Ok(lexer) => lexer,
        Err(_) => return deps,
    };
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(_) => return deps,
    };

    let mut idx = 0;
    while idx < tokens.len() {
        if matches!(tokens[idx].kind, TokenKind::Keyword(Keyword::Use)) {
            let mut j = idx + 1;
            while j < tokens.len() && matches!(tokens[j].kind, TokenKind::Newline) {
                j += 1;
            }
            if j >= tokens.len() || !matches!(tokens[j].kind, TokenKind::Identifier) {
                idx += 1;
                continue;
            }
            j += 1;
            while j < tokens.len() && matches!(tokens[j].kind, TokenKind::Newline) {
                j += 1;
            }
            if j >= tokens.len() || !matches!(tokens[j].kind, TokenKind::Equal) {
                idx += 1;
                continue;
            }
            j += 1;
            while j < tokens.len() && matches!(tokens[j].kind, TokenKind::Newline) {
                j += 1;
            }
            if j >= tokens.len() {
                idx += 1;
                continue;
            }
            if let TokenKind::StringLiteral(ref module_path) = tokens[j].kind {
                if module_path.starts_with("std.") || module_path.starts_with("support.") {
                    idx = j + 1;
                    continue;
                }
                if let Some(resolved) = resolve_import_path(source.path.as_path(), module_path) {
                    deps.insert(resolved);
                }
            }
            idx = j + 1;
        } else {
            idx += 1;
        }
    }

    deps
}

fn resolve_import_path(doc_path: &Path, import: &str) -> Option<PathBuf> {
    let base_dir = doc_path.parent().unwrap_or_else(|| Path::new("."));
    let mut path = if Path::new(import).is_absolute() {
        PathBuf::from(import)
    } else {
        base_dir.join(import)
    };
    if path.extension().is_none() {
        path.set_extension("tea");
    }
    Some(path.canonicalize().unwrap_or(path))
}

impl DocumentAnalysis {
    fn expected_type_at(&self, position: &Position) -> Option<&str> {
        self.argument_expectations
            .iter()
            .find(|expectation| range_contains(&expectation.range, position))
            .map(|expectation| expectation.type_desc.as_str())
    }
}

fn apply_content_change(text: &mut String, range: Option<Range>, new_text: &str) -> Result<()> {
    match range {
        None => {
            text.clear();
            text.push_str(new_text);
            Ok(())
        }
        Some(range) => {
            let start = position_to_offset(text, &range.start)
                .ok_or_else(|| anyhow!("invalid start position in change range"))?;
            let end = position_to_offset(text, &range.end)
                .ok_or_else(|| anyhow!("invalid end position in change range"))?;

            if end < start {
                anyhow::bail!("change range end precedes start");
            }

            text.replace_range(start..end, new_text);
            Ok(())
        }
    }
}

#[async_trait]
impl LanguageServer for TeaLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec![".".into()]),
                all_commit_characters: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
                completion_item: None,
            }),
            definition_provider: Some(OneOf::Left(true)),
            ..Default::default()
        };

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "tea-lsp".into(),
                version: None,
            }),
            capabilities,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "tea language server initialized")
            .await;
        self.client
            .log_message(
                MessageType::INFO,
                "Use `tea-lsp: set tracing true` in settings to increase logging verbosity.",
            )
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let text = params.text_document.text;

        if let Err(err) = (|| async {
            self.upsert_document(&uri, text, version).await?;
            self.compile_and_publish(&uri, None).await
        })()
        .await
        {
            self.client
                .log_message(MessageType::ERROR, format!("failed to open {uri}: {err}"))
                .await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let DidChangeTextDocumentParams {
            text_document,
            content_changes,
        } = params;
        let uri = text_document.uri;
        let version = text_document.version;

        let mut error_message = None;
        {
            let mut state = self.state.lock().await;
            if let Some(doc) = state.documents.get_mut(&uri) {
                for change in content_changes {
                    if let Err(err) =
                        apply_content_change(&mut doc.text, change.range, &change.text)
                    {
                        error_message = Some(format!("failed to apply change for {uri}: {err}"));
                        break;
                    }
                }

                if error_message.is_none() {
                    doc.version = version;
                    doc.analysis = None;
                }
            } else {
                error_message = Some(format!("received change for unknown document {}", uri));
            }
        }

        if let Some(message) = error_message {
            self.client.log_message(MessageType::ERROR, message).await;
            return;
        }

        self.schedule_compile(uri, version).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.remove_document(&uri).await;
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let mut doc = match self.document_snapshot(&uri).await {
            Some(doc) => doc,
            None => return Ok(None),
        };

        if doc.analysis.is_none() {
            let _ = self.compile_and_publish(&uri, None).await;
            doc = match self.document_snapshot(&uri).await {
                Some(doc) => doc,
                None => return Ok(None),
            };
        }

        let Some(ref analysis) = doc.analysis else {
            return Ok(None);
        };

        if let Some(symbol) = symbol_at_position(analysis, &position).or_else(|| {
            identifier_at_position(&doc.text, &position)
                .and_then(|name| symbol_by_name(analysis, &name))
        }) {
            let mut value = format!("{} `{}`", symbol.kind.label(), symbol.name);
            if let Some(ref ty) = symbol.type_desc {
                value.push_str(&format!(" : {}", ty));
            }
            if let Some(ref doc) = symbol.docstring {
                if !doc.is_empty() {
                    value.push_str("\n\n");
                    value.push_str(doc);
                }
            }
            let contents = HoverContents::Markup(MarkupContent {
                kind: MarkupKind::PlainText,
                value,
            });

            return Ok(Some(Hover {
                contents,
                range: Some(symbol.range.clone()),
            }));
        }

        if let Some((alias, member)) = member_at_position(&doc.text, &position) {
            if let Some(binding) = analysis.module_aliases.get(&alias) {
                let mut label = "symbol";
                if let Some(ty) = binding.export_types.get(&member) {
                    if ty.starts_with("Func") {
                        label = "function";
                    } else if ty == "Struct" {
                        label = "struct";
                    }
                }

                let mut value = format!("{} `{}.{}`", label, alias, member);
                if let Some(ty) = binding.export_types.get(&member) {
                    value.push_str(&format!(" : {}", ty));
                }
                let mut doc = binding.export_docs.get(&member).cloned();
                if doc.is_none() && binding.module_path.starts_with("std.") {
                    if let Some(module) = tea_compiler::stdlib_find_module(&binding.module_path) {
                        doc = module
                            .functions
                            .iter()
                            .find(|function| function.name == member)
                            .map(|function| function.docstring.to_string());
                    }
                }
                if let Some(doc) = doc {
                    if !doc.is_empty() {
                        value.push_str("\n\n");
                        value.push_str(&doc);
                    }
                }

                let contents = HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::PlainText,
                    value,
                });

                return Ok(Some(Hover {
                    contents,
                    range: None,
                }));
            }
        }

        Ok(None)
    }

    async fn completion(
        &self,
        params: tower_lsp::lsp_types::CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;

        let mut doc = match self.document_snapshot(&uri).await {
            Some(doc) => doc,
            None => return Ok(None),
        };

        if doc.analysis.is_none() {
            let _ = self.compile_and_publish(&uri, None).await;
            doc = match self.document_snapshot(&uri).await {
                Some(doc) => doc,
                None => return Ok(None),
            };
        }

        let Some(ref analysis) = doc.analysis else {
            return Ok(None);
        };

        let position = params.text_document_position.position;
        let expected_type = analysis
            .expected_type_at(&position)
            .filter(|ty| *ty != "Unknown");

        if let Some((alias, partial)) = member_completion_context(&doc.text, &position) {
            if let Some(binding) = analysis.module_aliases.get(&alias) {
                if !binding.exports.is_empty() {
                    let mut items = Vec::new();
                    let push_item = |export: &String, items: &mut Vec<CompletionItem>| {
                        let type_detail = binding.export_types.get(export);
                        let mut item =
                            CompletionItem::new_simple(export.clone(), format!("module {alias}"));
                        item.kind = Some(match type_detail {
                            Some(ty) if ty.starts_with("Func") => CompletionItemKind::FUNCTION,
                            Some(ty) if ty == "Struct" => CompletionItemKind::STRUCT,
                            Some(ty) if ty == "Enum" => CompletionItemKind::ENUM,
                            _ => CompletionItemKind::VARIABLE,
                        });
                        if let Some(ty) = type_detail {
                            item.detail = Some(ty.clone());
                        }
                        items.push(item);
                    };

                    for export in &binding.exports {
                        if !(partial.is_empty() || export.starts_with(&partial)) {
                            continue;
                        }
                        if let Some(expected) = expected_type {
                            if let Some(ty) = binding.export_types.get(export) {
                                if ty != expected {
                                    continue;
                                }
                            }
                        }
                        push_item(export, &mut items);
                    }

                    if items.is_empty() && expected_type.is_some() {
                        for export in &binding.exports {
                            if partial.is_empty() || export.starts_with(&partial) {
                                push_item(export, &mut items);
                            }
                        }
                    }

                    if !items.is_empty() {
                        return Ok(Some(CompletionResponse::Array(items)));
                    }
                }
            }
        }

        let mut seen = HashSet::new();
        let mut items = Vec::new();

        if let Some(expected) = expected_type {
            for symbol in &analysis.symbols {
                if symbol.type_desc.as_deref() != Some(expected) {
                    continue;
                }
                if seen.insert(symbol.name.clone()) {
                    let mut item = CompletionItem::new_simple(
                        symbol.name.clone(),
                        symbol.kind.label().to_string(),
                    );
                    item.kind = Some(symbol.kind.completion_kind());
                    if let Some(ref ty) = symbol.type_desc {
                        item.detail = Some(ty.clone());
                    }
                    items.push(item);
                }
            }
            if items.is_empty() {
                seen.clear();
                for symbol in &analysis.symbols {
                    if seen.insert(symbol.name.clone()) {
                        let mut item = CompletionItem::new_simple(
                            symbol.name.clone(),
                            symbol.kind.label().to_string(),
                        );
                        item.kind = Some(symbol.kind.completion_kind());
                        if let Some(ref ty) = symbol.type_desc {
                            item.detail = Some(ty.clone());
                        }
                        items.push(item);
                    }
                }
            }
        } else {
            for symbol in &analysis.symbols {
                if seen.insert(symbol.name.clone()) {
                    let mut item = CompletionItem::new_simple(
                        symbol.name.clone(),
                        symbol.kind.label().to_string(),
                    );
                    item.kind = Some(symbol.kind.completion_kind());
                    if let Some(ref ty) = symbol.type_desc {
                        item.detail = Some(ty.clone());
                    }
                    items.push(item);
                }
            }
        }

        if items.is_empty() {
            return Ok(None);
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let mut doc = match self.document_snapshot(&uri).await {
            Some(doc) => doc,
            None => return Ok(None),
        };

        if doc.analysis.is_none() {
            let _ = self.compile_and_publish(&uri, None).await;
            doc = match self.document_snapshot(&uri).await {
                Some(doc) => doc,
                None => return Ok(None),
            };
        }

        let Some(ref analysis) = doc.analysis else {
            return Ok(None);
        };

        let symbol = symbol_at_position(analysis, &position).or_else(|| {
            identifier_at_position(&doc.text, &position)
                .and_then(|name| symbol_by_name(analysis, &name))
        });

        let Some(symbol) = symbol else {
            return Ok(None);
        };

        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: symbol.range.clone(),
        })))
    }
}

impl TeaLanguageServer {
    fn run_compile_sync(
        source: SourceFile,
        module_overrides: &HashMap<PathBuf, String>,
        cancel: Option<CancellationToken>,
    ) -> Result<Option<BlockingCompileOutput>> {
        if Self::cancellation_requested(cancel.as_ref()) {
            return Ok(None);
        }

        let mut options = CompileOptions::default();
        options.module_overrides = module_overrides.clone();
        let mut compiler = Compiler::new(options);
        let compile_result = compiler.compile(&source);
        let diagnostics = compiler
            .diagnostics()
            .entries()
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        let output = match compile_result {
            Ok(compilation) => {
                let analysis = collect_symbols(
                    &compilation.module,
                    &compilation.module_aliases,
                    &compilation.binding_types,
                    &compilation.argument_types,
                );
                BlockingCompileOutput {
                    analysis: Some(analysis),
                    dependencies: collect_dependencies(&source.path, &compilation.module),
                    diagnostics,
                    compile_error: None,
                }
            }
            Err(err) => BlockingCompileOutput {
                analysis: None,
                dependencies: collect_dependencies_from_source(&source),
                diagnostics,
                compile_error: Some(err.to_string()),
            },
        };

        if Self::cancellation_requested(cancel.as_ref()) {
            return Ok(None);
        }

        Ok(Some(output))
    }

    async fn blocking_compile(
        &self,
        source: SourceFile,
        module_overrides: Arc<HashMap<PathBuf, String>>,
        cancel: Option<&CancellationToken>,
    ) -> Result<Option<BlockingCompileOutput>> {
        let cancel_clone = cancel.cloned();
        task::spawn_blocking(move || -> Result<Option<BlockingCompileOutput>> {
            TeaLanguageServer::run_compile_sync(source, &module_overrides, cancel_clone)
        })
        .await?
    }

    async fn compile_dependency_and_publish(
        &self,
        path: PathBuf,
        module_overrides: Arc<HashMap<PathBuf, String>>,
        cancel: Option<&CancellationToken>,
    ) -> Result<()> {
        let canonical = path.canonicalize().unwrap_or(path.clone());
        let url = match Url::from_file_path(&canonical) {
            Ok(url) => url,
            Err(_) => return Ok(()),
        };

        if Self::cancellation_requested(cancel) {
            return Ok(());
        }

        let cancel_clone = cancel.cloned();
        let module_overrides_clone = module_overrides.clone();
        let canonical_for_compile = canonical.clone();
        let compile_output =
            task::spawn_blocking(move || -> Result<Option<BlockingCompileOutput>> {
                if TeaLanguageServer::cancellation_requested(cancel_clone.as_ref()) {
                    return Ok(None);
                }
                let contents = std::fs::read_to_string(&canonical_for_compile)?;
                let source = SourceFile::new(SourceId(0), canonical_for_compile, contents);
                TeaLanguageServer::run_compile_sync(source, &module_overrides_clone, cancel_clone)
            })
            .await??;

        let Some(output) = compile_output else {
            self.client
                .log_message(
                    MessageType::LOG,
                    format!("compile:skip uri={} (dependency)", url),
                )
                .await;
            return Ok(());
        };

        if Self::cancellation_requested(cancel) {
            return Ok(());
        }

        let BlockingCompileOutput {
            diagnostics: compiler_diags,
            compile_error,
            ..
        } = output;

        let mut diagnostics = compiler_diags
            .iter()
            .map(convert_diagnostic)
            .collect::<Vec<_>>();

        if let Some(message) = compile_error {
            if diagnostics.is_empty() {
                let range = extract_range_from_message(&message).unwrap_or(Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                });
                diagnostics.push(LspDiagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("tea-compiler".into()),
                    message,
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }

        self.client
            .publish_diagnostics(url, diagnostics, None)
            .await;
        Ok(())
    }

    #[inline]
    fn cancellation_requested(token: Option<&CancellationToken>) -> bool {
        token.map_or(false, |t| t.is_cancelled())
    }
}

fn position_to_offset(text: &str, position: &Position) -> Option<usize> {
    let mut line_offsets = Vec::new();
    line_offsets.push(0usize);
    for (idx, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            line_offsets.push(idx + 1);
        }
    }
    line_offsets.push(text.len());

    let line = position.line as usize;
    let line_count = text.split_inclusive('\n').count();
    if line > line_count {
        return None;
    }
    if line == line_count {
        return if position.character == 0 {
            Some(text.len())
        } else {
            None
        };
    }

    let line_start = line_offsets[line];
    let line_end = line_offsets[line + 1];
    let line_str = &text[line_start..line_end];
    let character = position.character as usize;

    let mut char_count = 0usize;
    for (byte_idx, _) in line_str.char_indices() {
        if char_count == character {
            return Some(line_start + byte_idx);
        }
        char_count += 1;
    }

    if char_count == character {
        return Some(line_start + line_str.len());
    }

    None
}

fn identifier_at_position(text: &str, position: &Position) -> Option<String> {
    let offset = position_to_offset(text, position)?;
    let (line_start, line_end) = {
        let start = text[..offset].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let end = text[offset..]
            .find('\n')
            .map(|idx| offset + idx)
            .unwrap_or_else(|| text.len());
        (start, end)
    };

    let line_slice = &text[line_start..line_end];
    let cursor = offset.saturating_sub(line_start);
    let bytes = line_slice.as_bytes();

    if bytes.is_empty() {
        return None;
    }

    let mut idx = cursor.min(bytes.len().saturating_sub(1));
    if !is_identifier_byte(bytes[idx]) {
        while idx > 0 && !is_identifier_byte(bytes[idx]) {
            idx -= 1;
        }
        if !is_identifier_byte(bytes[idx]) {
            return None;
        }
    }

    let mut start = idx;
    while start > 0 && is_identifier_byte(bytes[start - 1]) {
        start -= 1;
    }

    let mut end = idx + 1;
    while end < bytes.len() && is_identifier_byte(bytes[end]) {
        end += 1;
    }

    std::str::from_utf8(&bytes[start..end])
        .ok()
        .map(|s| s.to_string())
}

fn is_identifier_byte(b: u8) -> bool {
    matches!(b,
        b'a'..=b'z'
        | b'A'..=b'Z'
        | b'0'..=b'9'
        | b'_'
    )
}

fn is_identifier_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn previous_char(text: &str, idx: usize) -> Option<(usize, char)> {
    if idx == 0 {
        return None;
    }
    text[..idx].char_indices().next_back()
}

fn member_completion_context(text: &str, position: &Position) -> Option<(String, String)> {
    let offset = position_to_offset(text, position)?;

    let mut partial_start = offset;
    while let Some((idx, ch)) = previous_char(text, partial_start) {
        if is_identifier_char(ch) {
            partial_start = idx;
        } else {
            break;
        }
    }

    let partial = text[partial_start..offset].to_string();

    let dot_index = match previous_char(text, partial_start) {
        Some((idx, '.')) => idx,
        Some((_, ch)) if ch.is_whitespace() => return None,
        Some(_) => return None,
        None => return None,
    };

    let mut alias_start = dot_index;
    while let Some((idx, ch)) = previous_char(text, alias_start) {
        if is_identifier_char(ch) {
            alias_start = idx;
        } else {
            break;
        }
    }

    let alias = text[alias_start..dot_index].trim();
    if alias.is_empty() {
        return None;
    }

    Some((alias.to_string(), partial))
}

fn member_at_position(text: &str, position: &Position) -> Option<(String, String)> {
    let mut offset = position_to_offset(text, position)?;
    if text.is_empty() {
        return None;
    }
    if offset >= text.len() {
        offset = text.len().saturating_sub(1);
    }

    let bytes = text.as_bytes();
    let mut idx = offset.min(bytes.len().saturating_sub(1));
    if !is_identifier_byte(bytes[idx]) {
        while idx > 0 && !is_identifier_byte(bytes[idx]) {
            idx -= 1;
        }
        if !is_identifier_byte(bytes[idx]) {
            return None;
        }
    }

    let mut member_start = idx;
    while member_start > 0 && is_identifier_byte(bytes[member_start - 1]) {
        member_start -= 1;
    }
    let mut member_end = idx + 1;
    while member_end < bytes.len() && is_identifier_byte(bytes[member_end]) {
        member_end += 1;
    }
    if member_start >= member_end {
        return None;
    }
    let member = text[member_start..member_end].to_string();

    let mut cursor = member_start;
    let mut dot_idx = None;
    while cursor > 0 {
        if let Some((prev_idx, ch)) = previous_char(text, cursor) {
            if ch.is_whitespace() {
                cursor = prev_idx;
                continue;
            }
            if ch == '.' {
                dot_idx = Some(prev_idx);
                break;
            }
            return None;
        } else {
            break;
        }
    }
    let dot_idx = dot_idx?;
    if dot_idx == 0 {
        return None;
    }

    let alias_end = dot_idx;
    let mut alias_start = alias_end;
    let mut found_alias_char = false;
    while alias_start > 0 {
        if let Some((prev_idx, ch)) = previous_char(text, alias_start) {
            if is_identifier_char(ch) {
                alias_start = prev_idx;
                found_alias_char = true;
                continue;
            }
            if ch.is_whitespace() {
                return None;
            }
            break;
        } else {
            alias_start = 0;
            found_alias_char = true;
            break;
        }
    }

    if !found_alias_char && alias_start == alias_end {
        return None;
    }

    let alias = text[alias_start..alias_end].trim();
    if alias.is_empty() {
        return None;
    }

    Some((alias.to_string(), member))
}

fn extract_range_from_message(message: &str) -> Option<Range> {
    fn parse_number(slice: &str) -> Option<(u32, usize)> {
        let mut value = 0u32;
        let mut consumed = 0usize;
        for ch in slice.chars() {
            if let Some(digit) = ch.to_digit(10) {
                value = value.saturating_mul(10).saturating_add(digit);
                consumed += ch.len_utf8();
            } else {
                break;
            }
        }
        if consumed == 0 {
            None
        } else {
            Some((value, consumed))
        }
    }

    let line_keyword = "line ";
    let column_keyword = "column ";

    let line_pos = message.find(line_keyword)?;
    let line_slice = &message[line_pos + line_keyword.len()..];
    let (line_number, line_consumed) = parse_number(line_slice)?;

    let column_search_start = line_pos + line_keyword.len() + line_consumed;
    let after_line = &message[column_search_start..];
    let column_pos = after_line.find(column_keyword)?;
    let column_slice = &after_line[column_keyword.len() + column_pos..];
    let (column_number, _) = parse_number(column_slice)?;

    let line = line_number.saturating_sub(1);
    let character = column_number.saturating_sub(1);

    Some(Range {
        start: Position { line, character },
        end: Position { line, character },
    })
}

fn range_contains(range: &Range, position: &Position) -> bool {
    if position.line < range.start.line || position.line > range.end.line {
        return false;
    }

    if range.start.line == range.end.line {
        return position.character >= range.start.character
            && position.character < range.end.character;
    }

    if position.line == range.start.line {
        return position.character >= range.start.character;
    }

    if position.line == range.end.line {
        return position.character < range.end.character;
    }

    true
}

fn convert_diagnostic(diagnostic: &tea_compiler::Diagnostic) -> LspDiagnostic {
    let range = if let Some(span) = diagnostic.span.as_ref() {
        let start_line = span.line.saturating_sub(1) as u32;
        let start_col = span.column.saturating_sub(1) as u32;
        let mut end_line = span.end_line.saturating_sub(1) as u32;
        let mut end_col = span.end_column.saturating_sub(1) as u32;

        if end_line < start_line || (end_line == start_line && end_col < start_col) {
            end_line = start_line;
            end_col = start_col;
        }

        Range {
            start: Position {
                line: start_line,
                character: start_col,
            },
            end: Position {
                line: end_line,
                character: end_col,
            },
        }
    } else {
        Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        }
    };

    let severity = match diagnostic.level {
        DiagnosticLevel::Error => Some(DiagnosticSeverity::ERROR),
        DiagnosticLevel::Warning => Some(DiagnosticSeverity::WARNING),
    };

    LspDiagnostic {
        range,
        severity,
        code: None,
        code_description: None,
        source: Some("tea-compiler".into()),
        message: diagnostic.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| TeaLanguageServer::new(client));
    let server = Server::new(stdin, stdout, socket);
    let local = task::LocalSet::new();
    local
        .run_until(async move {
            server.serve(service).await;
        })
        .await;

    Ok(())
}
