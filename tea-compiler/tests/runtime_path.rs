use std::path::{Path, PathBuf};

mod support;

fn escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

#[test]
fn path_builtins_roundtrip_through_runtime() -> anyhow::Result<()> {
    let joined = PathBuf::from_iter(["foo", "bar", "baz"]);
    let joined_str = joined.to_string_lossy().to_string();
    let absolute_base = PathBuf::from("/tmp/tea-path-base");
    let absolute_expected = absolute_base.join("src");
    let absolute_expected_str = absolute_expected.to_string_lossy().to_string();
    let relative_target = absolute_base.join("src").join("main.tea");
    let relative_expected = PathBuf::from_iter(["src", "main.tea"])
        .to_string_lossy()
        .to_string();

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
use assert from "std.assert"
use path from "std.path"

var joined = path.join(["foo", "bar", "baz"])
assert.eq(joined, "{joined}")

var parts = path.split(joined)
assert.eq(@len(parts), {component_count})

assert.eq(path.dirname(joined), "{dirname}")
assert.eq(path.basename(joined), "{basename}")

assert.eq(path.extension("{with_ext}"), "{extension}")
assert.eq(path.stem("{with_ext}"), "report")
assert.eq(path.parent(joined), "{dirname}")
assert.eq(path.absolute_from("src", "{absolute_base}"), "{absolute_expected}")
assert.eq(path.relative("{relative_target}", "{absolute_base}"), "{relative_expected}")
assert.ok(path.is_absolute(path.absolute("src")))
assert.eq(path.normalize("foo/./bar/../baz"), "foo/baz")
assert.ok(path.separator() != "")
@println("ok")
"#,
        joined = escape(&joined_str),
        component_count = components.len(),
        dirname = escape(&dirname),
        basename = escape(&basename),
        with_ext = escape(&with_ext.to_string_lossy()),
        extension = escape(&extension),
        absolute_base = escape(&absolute_base.to_string_lossy()),
        absolute_expected = escape(&absolute_expected_str),
        relative_target = escape(&relative_target.to_string_lossy()),
        relative_expected = escape(&relative_expected),
    );

    let stdout = support::run_script(&source, "path.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}
