mod support;

#[test]
fn process_helpers_execute_with_checked_wrappers() -> anyhow::Result<()> {
    let source = r#"
use process = "std.process"
use assert = "std.assert"

var result = process.run_checked("echo", ["hello"])
assert.ok(result.success)
assert.eq(result.stdout, "hello\n")

var streaming = process.spawn("cat", [])
process.write_stdin(streaming, "bye\n")
process.close_stdin(streaming)
var streamed = process.wait(streaming)
assert.eq(streamed.stdout, "bye\n")

var handle = process.spawn("cat", [])
process.close(handle)
@println("ok")
"#;

    let stdout = support::build_and_run(source, "process.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}
