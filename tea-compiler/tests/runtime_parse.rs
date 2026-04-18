mod support;

#[test]
fn parse_helpers_execute_with_runtime_errors() -> anyhow::Result<()> {
    let source = r#"
use assert from "std.assert"
use parse from "std.parse"

def invalid_int_input() -> String
  try parse.try_int("abc") catch err
  case is parse.ParseError.InvalidInt
    return err.input
  case _
    return "wrong"
  end

  @panic("expected parse.try_int to fail")
  return "wrong"
end

def invalid_bool_input() -> String
  try parse.try_bool("maybe") catch err
  case is parse.ParseError.InvalidBool
    return err.input
  case _
    return "wrong"
  end

  @panic("expected parse.try_bool to fail")
  return "wrong"
end

assert.eq(parse.int("42"), 42)
assert.eq(parse.int(" -17 "), -17)
assert.ok(parse.float("3.5") > 3.4)
assert.ok(parse.float("3.5") < 3.6)
assert.ok(parse.bool("yes"))
assert.ok(! parse.bool("off"))
assert.eq(@len(parse.words(" tea\tlang\ncli ")), 3)
assert.eq(invalid_int_input(), "abc")
assert.eq(invalid_bool_input(), "maybe")
@println("ok")
"#;

    let stdout = support::run_script(source, "parse.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}
