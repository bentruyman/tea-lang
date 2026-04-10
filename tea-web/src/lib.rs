use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tea_compiler::{
    CompileOptions, CompileTarget, Compiler, Diagnostic, InMemoryModuleLoader, SourceFile, SourceId,
};
use tea_eval::{evaluate, EvalOptions};
use wasm_bindgen::prelude::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunTeaRequest {
    entry_path: String,
    files: HashMap<String, String>,
    fuel: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunTeaResponse {
    diagnostics: Vec<WebDiagnostic>,
    stdout: Vec<String>,
    result: Option<String>,
    runtime_error: Option<String>,
    exit_code: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebDiagnostic {
    message: String,
    level: String,
    span: Option<WebSpan>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebSpan {
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
}

#[wasm_bindgen]
pub fn run_tea(value: JsValue) -> Result<JsValue, JsValue> {
    let request: RunTeaRequest = serde_wasm_bindgen::from_value(value)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let entry_path = PathBuf::from(&request.entry_path);
    let source_text = request
        .files
        .get(&request.entry_path)
        .cloned()
        .ok_or_else(|| JsValue::from_str("entryPath must exist in files"))?;

    let loader = InMemoryModuleLoader::new(
        request
            .files
            .into_iter()
            .map(|(path, contents)| (PathBuf::from(path), contents))
            .collect(),
    )
    .with_browser_stdlib();

    let source = SourceFile::new(SourceId(0), entry_path, source_text);
    let mut compiler = Compiler::new(CompileOptions {
        target: CompileTarget::Browser,
        module_loader: Some(Arc::new(loader)),
        ..CompileOptions::default()
    });

    let compilation = compiler.compile(&source);
    let diagnostics = compiler
        .diagnostics()
        .entries()
        .iter()
        .map(serialize_diagnostic)
        .collect::<Vec<_>>();

    let response = match compilation {
        Ok(compilation) => {
            let output = evaluate(
                &compilation,
                EvalOptions {
                    fuel: request.fuel.unwrap_or(EvalOptions::default().fuel),
                },
            );
            RunTeaResponse {
                diagnostics,
                stdout: output.stdout,
                result: output.result,
                runtime_error: output.runtime_error,
                exit_code: output.exit_code,
            }
        }
        Err(_) => RunTeaResponse {
            diagnostics,
            stdout: Vec::new(),
            result: None,
            runtime_error: None,
            exit_code: None,
        },
    };

    serde_wasm_bindgen::to_value(&response).map_err(|error| JsValue::from_str(&error.to_string()))
}

fn serialize_diagnostic(diagnostic: &Diagnostic) -> WebDiagnostic {
    WebDiagnostic {
        message: diagnostic.message.clone(),
        level: match diagnostic.level {
            tea_compiler::DiagnosticLevel::Error => "error".into(),
            tea_compiler::DiagnosticLevel::Warning => "warning".into(),
        },
        span: diagnostic.span.map(|span| WebSpan {
            line: span.line,
            column: span.column,
            end_line: span.end_line,
            end_column: span.end_column,
        }),
    }
}
