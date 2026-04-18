mod support;

#[test]
fn args_helpers_execute_with_cli_parser() -> anyhow::Result<()> {
    let source = r#"
use args from "std.args"
use assert from "std.assert"
use intrinsics from "std.intrinsics"

def verify() -> Void
  var spec = intrinsics.json_decode("{\"name\":\"todo\",\"description\":\"Task manager\",\"options\":[{\"name\":\"verbose\",\"aliases\":[\"-v\",\"--verbose\"],\"kind\":\"flag\"},{\"name\":\"count\",\"aliases\":[\"-c\",\"--count\"],\"kind\":\"option\",\"type\":\"int\"},{\"name\":\"tag\",\"aliases\":[\"-t\",\"--tag\"],\"kind\":\"option\",\"type\":\"string\",\"multiple\":true}],\"positionals\":[{\"name\":\"file\",\"type\":\"string\"}]}")
  var parsed = args.parse_with(spec, ["-v", "-c", "3", "--tag", "work", "--tag", "urgent", "tasks.txt"])
  assert.ok(parsed.ok)
  assert.eq(parsed.options["verbose"], true)
  assert.eq(parsed.options["count"], 3)
  assert.eq(@len(parsed.options["tag"]), 2)
  assert.eq(parsed.positionals["file"], "tasks.txt")
  assert.eq(@len(parsed.path), 1)
end

verify()
@println("ok")
"#;

    let stdout = support::build_and_run(source, "args.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}

#[test]
fn args_helpers_execute_with_subcommands() -> anyhow::Result<()> {
    let source = r#"
use args from "std.args"
use assert from "std.assert"
use intrinsics from "std.intrinsics"

def verify() -> Void
  var spec = intrinsics.json_decode("{\"name\":\"todo\",\"subcommands\":[{\"name\":\"done\",\"positionals\":[{\"name\":\"id\",\"type\":\"string\"}]}]}")
  var parsed = args.parse_with(spec, ["done", "task-7"])
  assert.ok(parsed.ok)
  assert.eq(parsed.command, "done")
  assert.eq(@len(parsed.path), 2)
  assert.eq(parsed.positionals["id"], "task-7")
end

verify()
@println("ok")
"#;

    let stdout = support::build_and_run(source, "args-subcommands.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}
