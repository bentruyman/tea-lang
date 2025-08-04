use std::borrow::Cow;
use std::fmt;

fn escape_single_quotes(input: &str) -> Cow<'_, str> {
    if input.contains('\'') {
        Cow::Owned(input.replace('\'', "\\'"))
    } else {
        Cow::Borrowed(input)
    }
}

fn format_operation_error(
    module: &str,
    operation: &str,
    target: Option<&str>,
    error: impl fmt::Display,
) -> String {
    match target {
        Some(target) => {
            let escaped = escape_single_quotes(target);
            format!("{module}.{operation}('{}') failed: {error}", escaped)
        }
        None => format!("{module}.{operation} failed: {error}"),
    }
}

pub fn fs_error(operation: &str, path: &str, error: impl fmt::Display) -> String {
    format_operation_error("std.fs", operation, Some(path), error)
}

pub fn io_error(operation: &str, error: impl fmt::Display) -> String {
    format_operation_error("std.io", operation, None, error)
}

pub fn cli_error(operation: &str, error: impl fmt::Display) -> String {
    format_operation_error("support.cli", operation, None, error)
}

pub fn cli_target_error(operation: &str, target: &str, error: impl fmt::Display) -> String {
    format_operation_error("support.cli", operation, Some(target), error)
}

pub fn process_error(operation: &str, command: &str, error: impl fmt::Display) -> String {
    format_operation_error("std.process", operation, Some(command), error)
}

pub fn env_error(operation: &str, target: Option<&str>, error: impl fmt::Display) -> String {
    format_operation_error("std.env", operation, target, error)
}
