use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::ast::Statement;
use crate::compiler::{CompileOptions, Compiler};
use crate::source::{SourceFile, SourceId};
use crate::stdlib::{self, StdFunction, StdModule, StdType, BUILTINS};

pub const BUILTIN_REFERENCE_SUMMARY: &str =
    "Tea exposes a small set of global `@` intrinsics for output, script control, introspection, and math.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferenceManifest {
    pub manifest_version: u32,
    pub generated_at: String,
    pub entries: Vec<ReferenceEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceEntryKind {
    Builtins,
    Module,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferenceEntry {
    pub slug: String,
    pub kind: ReferenceEntryKind,
    pub title: String,
    pub eyebrow: String,
    pub summary: String,
    pub module_path: Option<String>,
    pub source_path: String,
    pub functions: Vec<ReferenceFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferenceFunction {
    pub name: String,
    pub signature_display: String,
    pub summary: String,
}

pub fn build_reference_manifest(
    repo_root: &Path,
    generated_at: String,
) -> Result<ReferenceManifest> {
    let mut entries = vec![build_builtins_entry()];
    entries.extend(build_stdlib_entries(repo_root)?);

    Ok(ReferenceManifest {
        manifest_version: 1,
        generated_at,
        entries,
    })
}

fn build_builtins_entry() -> ReferenceEntry {
    let functions = BUILTINS
        .iter()
        .map(|builtin| ReferenceFunction {
            name: builtin.name.to_string(),
            signature_display: builtin_signature(builtin),
            summary: summarize_docstring(builtin.docstring),
        })
        .collect();

    ReferenceEntry {
        slug: "builtins".to_string(),
        kind: ReferenceEntryKind::Builtins,
        title: "Built-ins".to_string(),
        eyebrow: "Language Runtime".to_string(),
        summary: BUILTIN_REFERENCE_SUMMARY.to_string(),
        module_path: None,
        source_path: "tea-compiler/src/stdlib/builtins.rs".to_string(),
        functions,
    }
}

fn build_stdlib_entries(repo_root: &Path) -> Result<Vec<ReferenceEntry>> {
    stdlib::REFERENCE_STDLIB_MODULES
        .into_iter()
        .map(|module_path| {
            let slug = module_path
                .strip_prefix("std.")
                .expect("stdlib module paths should use std. prefix");
            build_tea_module_entry(repo_root, slug)
        })
        .collect()
}

fn build_tea_module_entry(repo_root: &Path, slug: &str) -> Result<ReferenceEntry> {
    let module_path = format!("std.{slug}");
    let source_path = format!("stdlib/{slug}/mod.tea");
    let absolute_path = repo_root.join(&source_path);
    let source = fs::read_to_string(&absolute_path)
        .with_context(|| format!("failed to read stdlib source {}", absolute_path.display()))?;
    let fallback_module = stdlib::find_module(&module_path);

    let summary = extract_leading_doc_comment(&source)
        .map(|doc| summarize_docstring(&doc))
        .filter(|doc| !doc.is_empty())
        .or_else(|| {
            fallback_module
                .map(|module| summarize_docstring(module.docstring))
                .filter(|doc| !doc.is_empty())
        })
        .unwrap_or_default();

    let functions = extract_public_tea_functions(&absolute_path, &source, fallback_module)?;

    Ok(ReferenceEntry {
        slug: slug.to_string(),
        kind: ReferenceEntryKind::Module,
        title: module_path.clone(),
        eyebrow: "Standard Library".to_string(),
        summary,
        module_path: Some(module_path),
        source_path,
        functions,
    })
}

fn extract_public_tea_functions(
    path: &Path,
    source: &str,
    fallback_module: Option<&StdModule>,
) -> Result<Vec<ReferenceFunction>> {
    let source_file = SourceFile::new(SourceId(0), path.to_path_buf(), source.to_string());
    let mut compiler = Compiler::new(CompileOptions::default());
    let parsed = compiler.parse_source(&source_file)?;
    let lines: Vec<&str> = source.lines().collect();
    let fallback_docs = fallback_module
        .map(|module| {
            module
                .functions
                .iter()
                .map(|function| (function.name, function.docstring))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    let mut functions = Vec::new();
    for statement in parsed.into_module().statements {
        let Statement::Function(function) = statement else {
            continue;
        };
        if !function.is_public {
            continue;
        }

        let signature_display = signature_from_source_line(&lines, function.name_span.line)
            .with_context(|| format!("failed to read signature for '{}'", function.name))?;
        let summary = function
            .docstring
            .as_deref()
            .map(summarize_docstring)
            .filter(|doc| !doc.is_empty())
            .or_else(|| {
                fallback_docs
                    .get(function.name.as_str())
                    .map(|doc| summarize_docstring(doc))
                    .filter(|doc| !doc.is_empty())
            })
            .unwrap_or_default();

        functions.push(ReferenceFunction {
            name: function.name,
            signature_display,
            summary,
        });
    }

    Ok(functions)
}

fn signature_from_source_line(lines: &[&str], line_number: usize) -> Result<String> {
    let signature = lines
        .get(line_number.saturating_sub(1))
        .map(|line| line.trim().to_string())
        .filter(|line| line.starts_with("pub def "))
        .ok_or_else(|| anyhow!("expected `pub def` on line {}", line_number))?;
    Ok(signature)
}

fn extract_leading_doc_comment(source: &str) -> Option<String> {
    let mut lines = Vec::new();
    let mut saw_comment = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            saw_comment = true;
            lines.push(trimmed.trim_start_matches('#').trim_start().to_string());
            continue;
        }

        if trimmed.is_empty() {
            if saw_comment {
                break;
            }
            continue;
        }

        break;
    }

    if !saw_comment {
        return None;
    }

    Some(lines.join("\n").trim().to_string())
}

fn summarize_docstring(doc: &str) -> String {
    let mut summary_lines = Vec::new();
    let mut saw_content = false;

    for line in doc.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Examples:") {
            break;
        }

        if trimmed.is_empty() {
            if saw_content {
                break;
            }
            continue;
        }

        saw_content = true;
        summary_lines.push(trimmed);
    }

    if !saw_content {
        return String::new();
    }

    summary_lines.join(" ")
}

fn builtin_signature(function: &StdFunction) -> String {
    if function.name == "append" {
        return "@append(list: List[T], value: T) -> Void".to_string();
    }

    let param_names = builtin_param_names(function.name);
    let params = function
        .params
        .iter()
        .enumerate()
        .map(|(idx, ty)| {
            let name = param_names
                .get(idx)
                .copied()
                .unwrap_or_else(|| fallback_param_name(idx));
            format!("{name}: {}", std_type_name(*ty))
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "@{}({}) -> {}",
        function.name,
        params,
        std_type_name(function.return_type)
    )
}

fn builtin_param_names(name: &str) -> &'static [&'static str] {
    match name {
        "print" | "println" | "to_string" | "type_of" | "eprint" | "eprintln" | "len" => &["value"],
        "panic" => &["message"],
        "exit" => &["code"],
        "floor" | "ceil" | "round" | "abs" | "sqrt" => &["value"],
        "max" | "min" => &["left", "right"],
        _ => &[],
    }
}

fn fallback_param_name(idx: usize) -> &'static str {
    match idx {
        0 => "arg1",
        1 => "arg2",
        2 => "arg3",
        3 => "arg4",
        _ => "arg",
    }
}

fn std_type_name(ty: StdType) -> &'static str {
    match ty {
        StdType::Any => "Any",
        StdType::Bool => "Bool",
        StdType::Int => "Int",
        StdType::Float => "Float",
        StdType::String => "String",
        StdType::List => "List",
        StdType::Dict => "Dict",
        StdType::Struct => "Struct",
        StdType::Nil => "Nil",
        StdType::Void => "Void",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use anyhow::Result;
    use tempfile::tempdir;

    use super::{
        build_builtins_entry, build_reference_manifest, build_tea_module_entry,
        summarize_docstring, ReferenceEntryKind, BUILTIN_REFERENCE_SUMMARY,
    };
    use crate::stdlib::BUILTINS;

    #[test]
    fn builtins_entry_matches_builtin_registry_order() {
        let entry = build_builtins_entry();
        let names = entry
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>();
        let builtin_names = BUILTINS
            .iter()
            .map(|function| function.name)
            .collect::<Vec<_>>();

        assert_eq!(entry.kind, ReferenceEntryKind::Builtins);
        assert_eq!(entry.summary, BUILTIN_REFERENCE_SUMMARY);
        assert_eq!(names, builtin_names);
        assert_eq!(
            entry.functions[0].signature_display,
            "@print(value: Any) -> Void"
        );
        assert_eq!(
            entry
                .functions
                .iter()
                .find(|function| function.name == "append")
                .expect("append builtin")
                .signature_display,
            "@append(list: List[T], value: T) -> Void"
        );
    }

    #[test]
    fn tea_module_entry_uses_only_public_functions_and_trims_examples() -> Result<()> {
        let repo = tempdir()?;
        let module_dir = repo.path().join("stdlib").join("sample");
        fs::create_dir_all(&module_dir)?;
        fs::write(
            module_dir.join("mod.tea"),
            r#"# Sample module summary.
#
# Additional context that should not be included.

pub def public_fn(name: String) -> String
  name
end

def private_fn() -> String
  ""
end
"#,
        )?;

        let entry = build_tea_module_entry(repo.path(), "sample")?;

        assert_eq!(entry.summary, "Sample module summary.");
        assert_eq!(entry.functions.len(), 1);
        assert_eq!(entry.functions[0].name, "public_fn");
        assert_eq!(
            entry.functions[0].signature_display,
            "pub def public_fn(name: String) -> String"
        );

        Ok(())
    }

    #[test]
    fn summarize_docstring_stops_before_examples_and_second_paragraph() {
        let summary = summarize_docstring(
            "First sentence.\nSecond sentence.\n\nExtra detail.\nExamples:\n  demo()",
        );

        assert_eq!(summary, "First sentence. Second sentence.");
    }

    #[test]
    fn manifest_includes_generated_stdlib_entries() -> Result<()> {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf();
        let manifest = build_reference_manifest(&repo_root, "1970-01-01T00:00:00Z".to_string())?;

        assert_eq!(manifest.manifest_version, 1);
        assert!(manifest
            .entries
            .iter()
            .any(|entry| entry.slug == "builtins"));
        assert!(manifest.entries.iter().any(|entry| entry.slug == "args"));
        assert!(manifest.entries.iter().any(|entry| entry.slug == "assert"));
        assert!(manifest.entries.iter().any(|entry| entry.slug == "json"));
        assert!(manifest.entries.iter().any(|entry| entry.slug == "string"));

        Ok(())
    }
}
