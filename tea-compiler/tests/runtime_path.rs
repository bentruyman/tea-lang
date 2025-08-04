use std::env;
use std::path::{Path, PathBuf};

use path_clean::PathClean;
use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId, Vm};

fn escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

#[test]
fn path_builtins_roundtrip_through_vm() -> anyhow::Result<()> {
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
    let replaced_ext = PathBuf::from("report.md").to_string_lossy().into_owned();
    let stripped_ext = PathBuf::from("report").to_string_lossy().into_owned();

    let normalize_input = PathBuf::from_iter(["logs", ".", "day", "..", "current"]);
    let normalize_expected = normalize_input.clean().to_string_lossy().into_owned();

    let base_path = PathBuf::from_iter(["workspace", "dist"]);
    let relative_target = PathBuf::from_iter(["workspace", "dist", "bin", "tea"]);
    let workspace_root = env::current_dir()?;
    let absolute_result = workspace_root.join(&base_path).join("bin/tea").clean();
    let absolute_str = absolute_result.to_string_lossy().into_owned();

    let separator = std::path::MAIN_SEPARATOR.to_string();

    let source = format!(
        r#"
use assert = "std.assert"
use path = "std.path"
use util = "std.util"

var joined = path.join(["foo", "bar", "baz"])
assert.assert_eq(joined, "{joined}")

var parts = path.components(joined)
assert.assert_eq(util.len(parts), {component_count})
assert.assert_eq(parts[0], "{part0}")
assert.assert_eq(parts[1], "{part1}")
assert.assert_eq(parts[2], "{part2}")

assert.assert_eq(path.dirname(joined), "{dirname}")
assert.assert_eq(path.basename(joined), "{basename}")

assert.assert_eq(path.extension("{with_ext}"), "{extension}")
assert.assert_eq(path.set_extension("{with_ext}", "md"), "{replaced_ext}")
assert.assert_eq(path.strip_extension("{with_ext}"), "{stripped_ext}")

assert.assert_eq(path.normalize("logs/./day/../current"), "{normalized}")

var abs_path = path.absolute("bin/tea", "{base}")
assert.assert_eq(abs_path, "{absolute}")
assert.assert(path.is_absolute(abs_path))
assert.assert(path.is_absolute(joined) == false)

var relative_value = path.relative("{target}", "{base}")
assert.assert(relative_value != "")
assert.assert_eq(path.separator(), "{separator}")
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
        replaced_ext = escape(&replaced_ext),
        stripped_ext = escape(&stripped_ext),
        normalized = escape(&normalize_expected),
        base = escape(&base_path.to_string_lossy()),
        absolute = escape(&absolute_str),
        target = escape(&relative_target.to_string_lossy()),
        separator = escape(&separator),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("path.tea"), source);
    let compilation = compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    let mut vm = Vm::new(&compilation.program);
    vm.run()?;

    Ok(())
}
