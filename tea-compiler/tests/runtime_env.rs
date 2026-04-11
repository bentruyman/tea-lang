use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

mod support;

fn unique_temp_dir() -> PathBuf {
    let mut base = std::env::temp_dir();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    base.push(format!("tea-env-test-{unique}"));
    base
}

#[test]
fn env_helpers_operate_through_runtime() -> anyhow::Result<()> {
    let cwd_path = unique_temp_dir();
    fs::create_dir_all(&cwd_path)?;
    let canonical_cwd = fs::canonicalize(&cwd_path)?;
    let cwd_str = canonical_cwd.to_string_lossy();
    let source = format!(
        r#"
use assert = "std.assert"
use env = "std.env"

assert.eq(env.get("TEA_LANG_TEST_VAR"), "")
assert.eq(env.get_or("TEA_LANG_TEST_FALLBACK", "fallback"), "fallback")
assert.ok(!env.has("TEA_LANG_TEST_VAR"))

env.set("TEA_LANG_TEST_VAR", "configured")
assert.eq(env.get("TEA_LANG_TEST_VAR"), "configured")
assert.ok(env.has("TEA_LANG_TEST_VAR"))
assert.eq(env.require("TEA_LANG_TEST_VAR"), "configured")

var vars = env.vars()
assert.ok(vars["TEA_LANG_TEST_VAR"] == "configured")

var temp_dir = env.temp_dir()
assert.ok(temp_dir != "")

var home_dir = env.home_dir()
var config_dir = env.config_dir()
assert.ok(@len(home_dir) >= 0)
assert.ok(@len(config_dir) >= 0)

env.set_cwd("{cwd}")
assert.eq(env.cwd(), "{cwd}")

env.unset("TEA_LANG_TEST_VAR")
assert.eq(env.get("TEA_LANG_TEST_VAR"), "")
@println("ok")
"#,
        cwd = cwd_str,
    );

    let stdout = support::build_and_run(&source, "env.tea", &[])?;
    fs::remove_dir_all(&cwd_path)?;
    assert_eq!(stdout, "ok\n");
    Ok(())
}
