use std::borrow::Cow;

/// Format tea-lang source code according to the canonical style guide.
///
/// The formatter normalises indentation to two spaces, trims trailing
/// whitespace, collapses multiple blank lines, and keeps inline comments in
/// place. It avoids altering the substantive contents of a line so that
/// constructs such as lambda literals or inline assertions remain unchanged.
pub fn format_source(input: &str) -> String {
    let mut output = String::new();
    let mut block_indent: usize = 0;
    let mut expr_indent: usize = 0;
    let mut continuation_indent: usize = 0;
    let mut pending_blank = false;
    let mut saw_any_line = false;

    for raw_line in input.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim_start();

        if trimmed.is_empty() {
            pending_blank = saw_any_line;
            continue;
        }

        if pending_blank && !output.is_empty() {
            output.push('\n');
        }
        pending_blank = false;
        saw_any_line = true;

        let (code_part, _comment) = split_code_and_comment(trimmed);
        let code_trimmed = code_part.trim_start();

        if is_block_closer(code_trimmed) {
            block_indent = block_indent.saturating_sub(1);
        }

        let metrics = analyze_brackets(code_trimmed);
        let previous_expr_indent = expr_indent;
        let effective_expr_indent = previous_expr_indent.saturating_sub(metrics.leading_closers);
        let line_extra_indent = hanging_operator_indent(code_trimmed);
        let applied_continuation = continuation_indent;
        let indent_level =
            block_indent + effective_expr_indent + applied_continuation + line_extra_indent;

        output.push_str(&indent_string(indent_level));
        let normalized_line = normalize_def_signature_spacing(trimmed);
        output.push_str(&normalized_line);
        output.push('\n');

        let next_expr_indent =
            (previous_expr_indent as isize + metrics.net_bracket_change).max(0) as usize;
        expr_indent = next_expr_indent;

        let mut next_continuation = 0usize;
        if line_continues_expression(code_trimmed) {
            next_continuation = 1;
        }
        continuation_indent = next_continuation;

        if opens_block(code_trimmed) {
            block_indent += 1;
        }
    }

    if pending_blank && !output.is_empty() {
        output.push('\n');
    }

    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    output
}

fn normalize_def_signature_spacing(line: &str) -> Cow<'_, str> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("def") {
        return Cow::Borrowed(line);
    }

    if let Some(next) = trimmed.chars().nth(3) {
        if !next.is_whitespace() {
            return Cow::Borrowed(line);
        }
    } else {
        return Cow::Borrowed(line);
    }

    let comment_idx = line.find('#');
    let code_end = comment_idx.unwrap_or_else(|| line.len());
    let code_slice = &line[..code_end];

    let paren_idx = match code_slice.find('(') {
        Some(idx) => idx,
        None => return Cow::Borrowed(line),
    };

    let mut space_start = paren_idx;
    let bytes = code_slice.as_bytes();
    while space_start > 0 && matches!(bytes[space_start - 1], b' ' | b'\t') {
        space_start -= 1;
    }

    if space_start == paren_idx {
        return Cow::Borrowed(line);
    }

    let spaces = paren_idx - space_start;
    let mut normalized = String::with_capacity(line.len() - spaces);
    normalized.push_str(&code_slice[..space_start]);
    normalized.push_str(&code_slice[paren_idx..]);
    if let Some(idx) = comment_idx {
        normalized.push_str(&line[idx..]);
    }

    Cow::Owned(normalized)
}

struct LineMetrics {
    leading_closers: usize,
    net_bracket_change: isize,
}

fn split_code_and_comment(line: &str) -> (&str, Option<&str>) {
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in line.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '#' => {
                let code = line[..idx].trim_end();
                return (code, Some(&line[idx..]));
            }
            _ => {}
        }
    }

    (line.trim_end(), None)
}

fn is_block_closer(code: &str) -> bool {
    line_starts_with_keyword(code, "end") || line_starts_with_keyword(code, "else")
}

fn opens_block(code: &str) -> bool {
    const BLOCK_KEYWORDS: &[&str] = &[
        "def", "if", "unless", "for", "while", "until", "struct", "else", "match",
    ];
    BLOCK_KEYWORDS
        .iter()
        .any(|kw| line_starts_with_keyword(code, kw))
}

fn line_starts_with_keyword(code: &str, keyword: &str) -> bool {
    if code.len() < keyword.len() || !code.starts_with(keyword) {
        return false;
    }

    match code[keyword.len()..].chars().next() {
        None => true,
        Some(ch) => matches!(ch, ' ' | '\t' | '#' | '('),
    }
}

fn analyze_brackets(code: &str) -> LineMetrics {
    let mut leading_closers = 0usize;
    let mut net_change: isize = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut leading_phase = true;

    for ch in code.chars() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        if leading_phase {
            match ch {
                ' ' | '\t' => continue,
                '#' => break,
                ')' | ']' | '}' => {
                    leading_closers += 1;
                    net_change -= 1;
                    continue;
                }
                '"' => {
                    in_string = true;
                    leading_phase = false;
                    continue;
                }
                _ => leading_phase = false,
            }
        }

        match ch {
            '"' => in_string = true,
            '#' => break,
            '(' | '[' | '{' => net_change += 1,
            ')' | ']' | '}' => net_change -= 1,
            _ => {}
        }
    }

    LineMetrics {
        leading_closers,
        net_bracket_change: net_change,
    }
}

fn indent_string(level: usize) -> String {
    const INDENT: &str = "  ";
    let mut buf = String::new();
    for _ in 0..level {
        buf.push_str(INDENT);
    }
    buf
}

fn line_continues_expression(code: &str) -> bool {
    let trimmed = code.trim_end();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.ends_with("->") || trimmed.ends_with("=>") {
        return true;
    }

    let mut chars = trimmed.chars().rev();
    let last = chars.find(|ch| !ch.is_whitespace());
    match last {
        Some(ch) if matches!(ch, '=' | '+' | '-' | '*' | '/' | '%') => true,
        _ => false,
    }
}

fn hanging_operator_indent(code: &str) -> usize {
    let trimmed = code.trim_start();
    if trimmed.is_empty() {
        return 0;
    }

    const LEADING_OPS: &[&str] = &["+", "-", "*", "/", "%", "&&", "||"];
    for op in LEADING_OPS {
        if trimmed.starts_with(op) {
            return 1;
        }
    }

    const LEADING_KW: &[&str] = &["and", "or"];
    for kw in LEADING_KW {
        if trimmed.starts_with(kw)
            && trimmed
                .chars()
                .nth(kw.len())
                .map(|ch| ch.is_whitespace())
                .unwrap_or(true)
        {
            return 1;
        }
    }

    0
}
