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

#[test]
fn args_wrappers_execute_with_required_accessors() -> anyhow::Result<()> {
    let source = r#"
use args from "std.args"
use assert from "std.assert"
use intrinsics from "std.intrinsics"

def usage_message() -> String
  var spec = intrinsics.json_decode("{\"name\":\"todo\",\"options\":[{\"name\":\"verbose\",\"aliases\":[\"-v\",\"--verbose\"],\"kind\":\"flag\"},{\"name\":\"count\",\"aliases\":[\"-c\",\"--count\"],\"kind\":\"option\",\"type\":\"int\"}],\"positionals\":[{\"name\":\"file\",\"type\":\"string\"}]}")
  var parsed = args.parse_with(spec, [])
  try args.require(parsed) catch err
  case is args.ArgsError.Usage
    return err.message
  case _
    return "wrong"
  end

  return "wrong"
end

def missing_option_name() -> String
  var spec = intrinsics.json_decode("{\"name\":\"todo\",\"options\":[{\"name\":\"count\",\"aliases\":[\"-c\",\"--count\"],\"kind\":\"option\",\"type\":\"int\"}]}")
  var parsed = args.parse_with(spec, [])
  try args.require_option_int(parsed, "count") catch err
  case is args.ArgsError.MissingOption
    return err.name
  case _
    return "wrong"
  end

  return "wrong"
end

var spec = intrinsics.json_decode("{\"name\":\"todo\",\"options\":[{\"name\":\"verbose\",\"aliases\":[\"-v\",\"--verbose\"],\"kind\":\"flag\"},{\"name\":\"count\",\"aliases\":[\"-c\",\"--count\"],\"kind\":\"option\",\"type\":\"int\"}],\"positionals\":[{\"name\":\"file\",\"type\":\"string\"}]}")
var parsed = args.parse_with(spec, ["-v", "-c", "3", "tasks.txt"])
assert.ok(args.flag(parsed, "verbose"))
assert.eq(args.option_int_or(parsed, "count", 0), 3)
assert.eq(args.require_option_int(parsed, "count"), 3)
assert.eq(args.require_positional(parsed, "file"), "tasks.txt")
assert.eq(missing_option_name(), "count")
assert.ok(@len(usage_message()) > 0)

var sub_spec = intrinsics.json_decode("{\"name\":\"todo\",\"subcommands\":[{\"name\":\"done\",\"positionals\":[{\"name\":\"id\",\"type\":\"string\"}]}]}")
var sub = args.parse_with(sub_spec, ["done", "task-7"])
assert.eq(args.require_subcommand(sub), "done")
assert.ok(args.command_is(sub, "done"))
@println("ok")
"#;

    let stdout = support::build_and_run(source, "args-wrappers.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}
