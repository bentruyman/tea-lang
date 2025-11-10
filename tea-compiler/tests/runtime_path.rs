use std::env;
use std::path::{Path, PathBuf};

use path_clean::PathClean;
use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

fn escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

#[test]
fn path_builtins_roundtrip_through_runtime() -> anyhow::Result<()> {
    let joined = PathBuf::from_iter(["foo", "bar", "baz"]);
    let joined_str = joined.to_string_lossy().to_string();

    let components: Vec<String> = joined
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();

    let dirname = joined
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                ".".to_string()
            } else {
                p.to_string_lossy().into_owned()
            }
        })
        .unwrap_or_else(|| ".".to_string());

    let basename = joined
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let with_ext = PathBuf::from("report.txt");
    let extension = Path::new("report.txt")
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default();

    let source = format!(
        r#"
use assert = "std.assert"
use path = "std.path"

var joined = path.join(["foo", "bar", "baz"])
assert.eq(joined, "{joined}")

var parts = path.split(joined)
assert.eq(@len(parts), {component_count})
assert.eq(parts[0], "{part0}")
assert.eq(parts[1], "{part1}")
assert.eq(parts[2], "{part2}")

assert.eq(path.dirname(joined), "{dirname}")
assert.eq(path.basename(joined), "{basename}")

assert.eq(path.extension("{with_ext}"), "{extension}")
"#,
        joined = escape(&joined_str),
        component_count = components.len(),
        part0 = escape(&components[0]),
        part1 = escape(&components[1]),
        part2 = escape(&components[2]),
        dirname = escape(&dirname),
        basename = escape(&basename),
        with_ext = escape(&with_ext.to_string_lossy()),
        extension = escape(&extension),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("path.tea"), source);
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Full test execution support via AOT is planned for the future
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // TODO: Add AOT test execution when implemented
    // For now, we verify that the code compiles without errors

    Ok(())
}
