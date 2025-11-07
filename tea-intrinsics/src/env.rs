use anyhow::Result;
use std::collections::HashMap;
use std::env;
use tea_support::env_error;

/// Gets an environment variable value, returning empty string if not set
pub fn get(key: &str) -> String {
    env::var(key).unwrap_or_default()
}

/// Checks if an environment variable is set
pub fn has(key: &str) -> bool {
    env::var(key).is_ok()
}

/// Sets an environment variable
pub fn set(key: &str, value: &str) {
    env::set_var(key, value);
}

/// Unsets an environment variable
pub fn unset(key: &str) {
    env::remove_var(key);
}

/// Gets all environment variables as a map
pub fn vars() -> HashMap<String, String> {
    env::vars().collect()
}

/// Gets the current working directory
pub fn cwd() -> Result<String> {
    let current_dir =
        env::current_dir().map_err(|error| anyhow::anyhow!(env_error("cwd", None, error)))?;
    Ok(current_dir.to_string_lossy().into_owned())
}
