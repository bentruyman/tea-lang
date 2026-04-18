use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Result};

use crate::analysis::SemanticAnalysis;
use crate::ast::{Module, SourceSpan};
use crate::browser::validate_browser_target;
use crate::diagnostics::Diagnostics;
use crate::expansion::{ExpandedModule, ModuleExpander};
use crate::lexer::{Lexer, LexerError};
use crate::loader::ModuleLoader;
use crate::parser::Parser;
use crate::resolver::{ModuleAliasBinding, Resolver, ResolverOutput};
use crate::source::SourceFile;
use crate::typechecker::TypeChecker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileTarget {
    Native,
    Browser,
}

#[derive(Clone)]
pub struct CompileOptions {
    pub dump_tokens: bool,
    pub module_overrides: HashMap<PathBuf, String>,
    pub target: CompileTarget,
    pub module_loader: Option<Arc<dyn ModuleLoader>>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            dump_tokens: false,
            module_overrides: HashMap::new(),
            target: CompileTarget::Native,
            module_loader: None,
        }
    }
}

pub struct Compilation {
    pub module: Module,
    pub analysis: SemanticAnalysis,
}

pub struct ParsedModule {
    pub module: Module,
}

impl ParsedModule {
    pub fn module(&self) -> &Module {
        &self.module
    }

    pub fn into_module(self) -> Module {
        self.module
    }
}

pub struct ResolvedModule {
    pub module: Module,
    lambda_captures: HashMap<usize, Vec<String>>,
    module_aliases: HashMap<String, ModuleAliasBinding>,
    alias_exports: HashMap<String, Vec<String>>,
    alias_export_renames: HashMap<String, HashMap<String, String>>,
    alias_export_docstrings: HashMap<String, HashMap<String, String>>,
}

impl ResolvedModule {
    pub fn module(&self) -> &Module {
        &self.module
    }
}

pub struct Compiler {
    diagnostics: Diagnostics,
    options: CompileOptions,
}

impl Compiler {
    pub fn new(options: CompileOptions) -> Self {
        Self {
            diagnostics: Diagnostics::new(),
            options,
        }
    }

    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    pub fn parse_source(&mut self, source: &SourceFile) -> Result<ParsedModule> {
        let mut lexer = Lexer::new(source)?;
        let tokens = match lexer.tokenize() {
            Ok(tokens) => tokens,
            Err(err) => {
                if let Some(lexer_error) = err.downcast_ref::<LexerError>() {
                    let line = lexer_error.line();
                    let column = lexer_error.column();
                    self.diagnostics.push_error_with_span(
                        lexer_error.to_string(),
                        Some(SourceSpan::new(line, column, line, column)),
                    );
                } else {
                    self.diagnostics.push_error_with_span(err.to_string(), None);
                }
                bail!("Lexing failed");
            }
        };

        if self.options.dump_tokens {
            for token in &tokens {
                println!("{token:?}");
            }
        }

        let mut parser = Parser::new(source, tokens);
        let module = match parser.parse() {
            Ok(module) => {
                let diagnostics = parser.into_diagnostics();
                self.diagnostics.extend(diagnostics);
                Ok(module)
            }
            Err(err) => {
                let diagnostics = parser.into_diagnostics();
                self.diagnostics.extend(diagnostics);
                Err(err)
            }
        }?;

        Ok(ParsedModule { module })
    }

    pub fn expand_modules(
        &mut self,
        source: &SourceFile,
        parsed: ParsedModule,
    ) -> Result<ExpandedModule> {
        let entry_path = source.path.clone();
        let mut expander = ModuleExpander::new(
            self.options.module_overrides.clone(),
            self.options.module_loader.clone(),
        )?;
        let expanded_module = match expander.expand(&parsed.module, &entry_path) {
            Ok(module) => module,
            Err(err) => {
                let diagnostics = expander.into_diagnostics();
                self.diagnostics.extend(diagnostics);
                return Err(err);
            }
        };
        let diagnostics = expander.into_diagnostics();
        let has_errors = diagnostics.has_errors();
        self.diagnostics.extend(diagnostics);
        if has_errors {
            bail!("Module expansion failed");
        }

        Ok(expanded_module)
    }

    pub fn resolve_module(&mut self, expanded: ExpandedModule) -> Result<ResolvedModule> {
        let (module, alias_exports, alias_export_renames, alias_export_docstrings) =
            expanded.into_parts();

        let mut resolver = Resolver::new();
        resolver.resolve_module(&module);
        let ResolverOutput {
            diagnostics: resolve_diagnostics,
            lambda_captures,
            module_aliases,
        } = resolver.into_parts();
        let resolve_errors = resolve_diagnostics.has_errors();
        self.diagnostics.extend(resolve_diagnostics);
        if resolve_errors {
            bail!("Name resolution failed");
        }

        Ok(ResolvedModule {
            module,
            lambda_captures,
            module_aliases,
            alias_exports,
            alias_export_renames,
            alias_export_docstrings,
        })
    }

    pub fn typecheck_module(&mut self, resolved: ResolvedModule) -> Result<Compilation> {
        let ResolvedModule {
            module,
            lambda_captures,
            module_aliases,
            alias_exports,
            alias_export_renames,
            alias_export_docstrings,
        } = resolved;

        let mut type_checker = TypeChecker::new();
        type_checker.check_module(&module);
        let analysis = SemanticAnalysis::from_parts(
            lambda_captures,
            module_aliases,
            &type_checker,
            &alias_exports,
            &alias_export_renames,
            &alias_export_docstrings,
        );
        let mut type_diagnostics = type_checker.into_diagnostics();
        let type_errors = type_diagnostics.has_errors();
        if !alias_export_renames.is_empty() {
            let mut reverse_names: HashMap<String, String> = HashMap::new();
            for (alias, renames) in &alias_export_renames {
                for (original, renamed) in renames {
                    reverse_names.insert(renamed.clone(), format!("{}.{}", alias, original));
                }
            }
            if !reverse_names.is_empty() {
                for diagnostic in type_diagnostics.entries_mut() {
                    let mut message = diagnostic.message.clone();
                    for (internal, display) in &reverse_names {
                        if message.contains(internal) {
                            message = message.replace(internal, display);
                        }
                    }
                    diagnostic.message = message;
                }
            }
        }
        self.diagnostics.extend(type_diagnostics);
        if type_errors {
            bail!("Type checking failed");
        }

        let compilation = Compilation { module, analysis };
        if self.options.target == CompileTarget::Browser {
            let diagnostics = validate_browser_target(&compilation.module, &compilation.analysis);
            let has_errors = diagnostics.has_errors();
            self.diagnostics.extend(diagnostics);
            if has_errors {
                bail!("Browser target validation failed");
            }
        }

        Ok(compilation)
    }

    pub fn compile(&mut self, source: &SourceFile) -> Result<Compilation> {
        let parsed = self.parse_source(source)?;
        let expanded = self.expand_modules(source, parsed)?;
        let resolved = self.resolve_module(expanded)?;
        self.typecheck_module(resolved)
    }
}
