mod support;

#[test]
fn process_helpers_execute_with_checked_wrappers() -> anyhow::Result<()> {
    let source = r#"
use process from "std.process"
use assert from "std.assert"

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

#[test]
fn process_byte_helpers_execute_through_runtime() -> anyhow::Result<()> {
    let source = r#"
use process from "std.process"
use assert from "std.assert"

var stdout_handle = process.spawn("cat", [])
process.write_stdin_bytes(stdout_handle, [0, 255, 10, 65])
process.close_stdin(stdout_handle)
var stdout_bytes = process.read_stdout_bytes(stdout_handle, 4)
assert.eq(@len(stdout_bytes), 4)
assert.eq(stdout_bytes[0], 0)
assert.eq(stdout_bytes[1], 255)
assert.eq(stdout_bytes[2], 10)
assert.eq(stdout_bytes[3], 65)
var stdout_result = process.wait(stdout_handle)
assert.ok(stdout_result.success)
assert.eq(stdout_result.stdout, "")

var stderr_handle = process.spawn("sh", ["-c", "printf 'oops' >&2"])
var stderr_bytes = process.read_stderr_bytes(stderr_handle, 4)
assert.eq(@len(stderr_bytes), 4)
assert.eq(stderr_bytes[0], 111)
assert.eq(stderr_bytes[1], 111)
assert.eq(stderr_bytes[2], 112)
assert.eq(stderr_bytes[3], 115)
var stderr_result = process.wait(stderr_handle)
assert.ok(stderr_result.success)
assert.eq(stderr_result.stderr, "")

@println("ok")
"#;

    let stdout = support::build_and_run(source, "process-bytes.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}
