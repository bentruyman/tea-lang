mod support;

#[test]
fn time_helpers_execute_through_runtime() -> anyhow::Result<()> {
    let source = r#"
use assert from "std.assert"
use time from "std.time"

def invalid_timestamp_input() -> String
  try time.try_parse_rfc3339("not-a-timestamp") catch err
  case is time.TimeError.InvalidRfc3339
    return err.input
  case _
    return "wrong"
  end

  @panic("expected time.try_parse_rfc3339 to fail")
  return "wrong"
end

const epoch = time.from_unix_millis(0)
assert.eq(epoch.unix_seconds, 0)
assert.eq(epoch.unix_millis, 0)
assert.eq(time.unix_seconds(epoch), 0)
assert.eq(time.unix_millis(epoch), 0)
assert.eq(time.format_rfc3339(epoch), "1970-01-01T00:00:00Z")

const parsed = time.parse_rfc3339("2026-04-18T12:34:56.789Z")
assert.eq(time.format_rfc3339(parsed), "2026-04-18T12:34:56.789Z")
assert.eq(parsed.unix_seconds, time.unix_seconds(parsed))
assert.eq(parsed.unix_millis, time.unix_millis(parsed))

assert.eq(time.milliseconds(250).milliseconds, 250)
assert.eq(time.seconds(3).milliseconds, 3000)
assert.eq(time.minutes(2).milliseconds, 120000)
assert.eq(time.hours(1).milliseconds, 3600000)
assert.eq(time.days(1).milliseconds, 86400000)
assert.eq(time.add(time.seconds(3), time.milliseconds(250)).milliseconds, 3250)
assert.eq(time.subtract(time.seconds(3), time.milliseconds(250)).milliseconds, 2750)
assert.eq(time.multiply(time.seconds(2), 4).milliseconds, 8000)
assert.eq(
  time.between(time.from_unix_seconds(10), time.from_unix_millis(12500)).milliseconds,
  2500
)
assert.eq(
  time.add_to(time.from_unix_seconds(10), time.seconds(5)).unix_millis,
  15000
)
assert.eq(
  time.subtract_from(time.from_unix_millis(15000), time.seconds(5)).unix_millis,
  10000
)

assert.eq(invalid_timestamp_input(), "not-a-timestamp")

const before = time.now()
assert.ok(before.unix_millis > 0)
assert.eq(before.unix_seconds, time.unix_seconds(before))
assert.ok(time.now_unix_seconds() > 0)
assert.ok(time.now_unix_millis() > 0)

time.sleep(5)
time.sleep_for(time.milliseconds(1))

const after = time.now()
assert.ok(after.unix_millis >= before.unix_millis)

@println("ok")
"#;

    let stdout = support::run_script(source, "time.tea", &[])?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}
