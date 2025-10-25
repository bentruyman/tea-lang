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
    let mut block_stack: Vec<BlockState> = Vec::new();
    let mut pending_blank_after_block: Option<BlockKind> = None;
    let mut last_significant_was_comment = false;

    for raw_line in input.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim_start();

        if trimmed.is_empty() {
            let skip_for_function_edge = block_stack
                .last()
                .map(|state| {
                    matches!(state.kind, BlockKind::Function | BlockKind::Match)
                        && !state.saw_content
                })
                .unwrap_or(false);

            if skip_for_function_edge
                || matches!(pending_blank_after_block, Some(BlockKind::Conditional))
            {
                continue;
            }

            pending_blank = saw_any_line;
            pending_blank_after_block = None;
            continue;
        }

        if pending_blank && !output.is_empty() {
            output.push('\n');
        }
        pending_blank = false;
        saw_any_line = true;

        let (code_part, _comment) = split_code_and_comment(trimmed);
        let code_trimmed = code_part.trim_start();
        let inline_match_line = is_inline_match_expression(code_trimmed);

        if let Some(_) = pending_blank_after_block {
            if !is_block_closer(code_trimmed) {
                if !output.is_empty() && !output.ends_with("\n\n") {
                    output.push('\n');
                }
                pending_blank_after_block = None;
            }
        }

        let is_function_line = is_function_header(code_trimmed);
        let is_test_line = is_test_header(code_trimmed);
        let is_conditional_line = is_conditional_header(code_trimmed);
        let parent_has_content = block_stack
            .last()
            .map(|state| state.saw_content)
            .unwrap_or(false);

        let should_insert_blank_before = if is_function_line || is_test_line {
            true
        } else if is_conditional_line {
            parent_has_content
                || (block_stack.is_empty() && saw_any_line && !last_significant_was_comment)
        } else {
            false
        };

        if should_insert_blank_before
            && !output.is_empty()
            && !output.ends_with("\n\n")
            && !last_significant_was_comment
        {
            output.push('\n');
        }

        let mut closed_block: Option<BlockKind> = None;
        let mut reopened_from_else: Option<BlockKind> = None;
        if is_block_closer(code_trimmed) {
            if line_starts_with_keyword(code_trimmed, "end") {
                if let Some(state) = block_stack.pop() {
                    closed_block = Some(state.kind);
                }
            } else if line_starts_with_keyword(code_trimmed, "else") {
                reopened_from_else = block_stack.pop().map(|state| state.kind);
            }
            block_indent = block_indent.saturating_sub(1);
        }

        if !trimmed.is_empty()
            && !line_starts_with_keyword(code_trimmed, "else")
            && !line_starts_with_keyword(code_trimmed, "end")
        {
            if let Some(state) = block_stack.last_mut() {
                state.saw_content = true;
            }
        } else if trimmed.starts_with('#') {
            if let Some(state) = block_stack.last_mut() {
                state.saw_content = true;
            }
        }

        let metrics = analyze_brackets(code_trimmed);
        let previous_expr_indent = expr_indent;
        let effective_expr_indent = previous_expr_indent.saturating_sub(metrics.leading_closers);
        let line_extra_indent = hanging_operator_indent(code_trimmed);
        let applied_continuation = continuation_indent;
        let indent_level =
            block_indent + effective_expr_indent + applied_continuation + line_extra_indent;

        output.push_str(&indent_string(indent_level));
        let normalized_line = normalize_line(trimmed);
        output.push_str(normalized_line.as_ref());
        output.push('\n');

        last_significant_was_comment = trimmed.starts_with('#');

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
            let kind = if is_function_line {
                BlockKind::Function
            } else if is_test_line {
                BlockKind::Test
            } else if let Some(kind) = reopened_from_else {
                kind
            } else if is_conditional_line {
                BlockKind::Conditional
            } else {
                BlockKind::Other
            };
            block_stack.push(BlockState {
                kind,
                saw_content: false,
            });
        }

        if inline_match_line {
            block_indent += 1;
            block_stack.push(BlockState {
                kind: BlockKind::Match,
                saw_content: false,
            });
        }

        if let Some(kind) = closed_block {
            if matches!(
                kind,
                BlockKind::Function | BlockKind::Conditional | BlockKind::Test | BlockKind::Match
            ) {
                pending_blank_after_block = Some(kind);
            } else {
                pending_blank_after_block = None;
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Function,
    Conditional,
    Test,
    Match,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BlockState {
    kind: BlockKind,
    saw_content: bool,
}

fn normalize_line(line: &str) -> Cow<'_, str> {
    let comment_idx = find_comment_index(line);
    let (code_with_ws, comment) = match comment_idx {
        Some(idx) => (&line[..idx], Some(&line[idx..])),
        None => (line, None),
    };

    let trimmed_code = code_with_ws.trim_end();
    let trailing_ws = &code_with_ws[trimmed_code.len()..];

    let mut normalized_code: Cow<'_, str> = Cow::Borrowed(trimmed_code);
    macro_rules! apply_normalizer {
        ($func:expr) => {{
            let current = normalized_code.as_ref();
            if let Cow::Owned(new_value) = $func(current) {
                if new_value != current {
                    normalized_code = Cow::Owned(new_value);
                }
            }
        }};
    }

    apply_normalizer!(normalize_def_signature_spacing);
    apply_normalizer!(normalize_control_keyword_spacing);
    apply_normalizer!(normalize_expression_spacing);
    apply_normalizer!(normalize_case_arrow_spacing);
    apply_normalizer!(normalize_struct_declaration_spacing);
    apply_normalizer!(normalize_enum_declaration_spacing);
    apply_normalizer!(normalize_comparison_operator_spacing);

    let mut result = String::with_capacity(line.len());
    result.push_str(normalized_code.as_ref());
    result.push_str(trailing_ws);
    if let Some(comment) = comment {
        result.push_str(comment);
    }

    if result == line {
        Cow::Borrowed(line)
    } else {
        Cow::Owned(result)
    }
}

fn normalize_def_signature_spacing(line: &str) -> Cow<'_, str> {
    let trimmed = line.trim_start();
    let leading_len = line.len() - trimmed.len();
    let leading_ws = &line[..leading_len];
    let mut rest = trimmed;

    let mut normalized = String::new();
    normalized.push_str(leading_ws);

    if let Some(after_pub) = rest.strip_prefix("pub") {
        if after_pub
            .chars()
            .next()
            .map(|c| c.is_whitespace())
            .unwrap_or(false)
        {
            normalized.push_str("pub ");
            rest = after_pub.trim_start();
        } else {
            return Cow::Borrowed(line);
        }
    }

    if !rest.starts_with("def") {
        return Cow::Borrowed(line);
    }

    let mut after_def = &rest["def".len()..];
    if !after_def
        .chars()
        .next()
        .map(|c| c.is_whitespace())
        .unwrap_or(false)
    {
        // require a delimiter between `def` and name
        return Cow::Borrowed(line);
    }

    normalized.push_str("def ");
    after_def = after_def.trim_start();
    if after_def.is_empty() {
        return Cow::Borrowed(line);
    }

    // Extract function name
    let mut name_start = None;
    let mut name_end = after_def.len();
    for (idx, ch) in after_def.char_indices() {
        if name_start.is_none() {
            if ch.is_whitespace() {
                continue;
            } else {
                name_start = Some(idx);
            }
        } else if ch.is_whitespace() || matches!(ch, '[' | '(') {
            name_end = idx;
            break;
        }
    }

    let name_start = match name_start {
        Some(idx) => idx,
        None => return Cow::Borrowed(line),
    };
    let name = &after_def[name_start..name_end];
    if name.is_empty() {
        return Cow::Borrowed(line);
    }
    normalized.push_str(name);

    // Advance index past the name and any trailing whitespace
    let mut idx = name_end;
    while idx < after_def.len() {
        let ch = after_def[idx..].chars().next().unwrap();
        if ch.is_whitespace() {
            idx += ch.len_utf8();
        } else {
            break;
        }
    }

    // Optional generic parameters
    if idx < after_def.len() && after_def[idx..].starts_with('[') {
        let mut depth = 0i32;
        let mut closing = None;
        for (offset, ch) in after_def[idx..].char_indices() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        closing = Some(idx + offset);
                        break;
                    }
                }
                _ => {}
            }
        }

        let closing = match closing {
            Some(c) => c,
            None => return Cow::Borrowed(line),
        };

        let content = &after_def[idx + 1..closing];
        let generics = content
            .split(',')
            .map(|part| part.trim())
            .collect::<Vec<_>>()
            .join(", ");
        normalized.push('[');
        normalized.push_str(&generics);
        normalized.push(']');

        idx = closing + 1;
        while idx < after_def.len() {
            let ch = after_def[idx..].chars().next().unwrap();
            if ch.is_whitespace() {
                idx += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    // Parameters
    if idx >= after_def.len() || !after_def[idx..].starts_with('(') {
        return Cow::Borrowed(line);
    }

    let mut depth = 0i32;
    let mut close_idx = None;
    for (offset, ch) in after_def[idx..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close_idx = Some(idx + offset);
                    break;
                }
            }
            _ => {}
        }
    }

    let close_idx = match close_idx {
        Some(c) => c,
        None => return Cow::Borrowed(line),
    };

    let params_content = &after_def[idx + 1..close_idx];
    let formatted_params = if params_content.trim().is_empty() {
        String::new()
    } else {
        params_content
            .split(',')
            .map(|arg| arg.trim())
            .collect::<Vec<_>>()
            .join(", ")
    };
    normalized.push('(');
    normalized.push_str(&formatted_params);
    normalized.push(')');

    idx = close_idx + 1;
    let mut rest_after = after_def[idx..].trim_start();

    if rest_after.starts_with("->") {
        rest_after = rest_after[2..].trim_start();
        normalized.push_str(" -> ");
        normalized.push_str(rest_after);
    } else if !rest_after.is_empty() {
        normalized.push(' ');
        normalized.push_str(rest_after);
    }

    if normalized == line {
        Cow::Borrowed(line)
    } else {
        Cow::Owned(normalized)
    }
}

fn normalize_control_keyword_spacing(line: &str) -> Cow<'_, str> {
    let first_non_ws = match line.find(|ch: char| !ch.is_whitespace()) {
        Some(idx) => idx,
        None => return Cow::Borrowed(line),
    };

    const KEYWORDS: &[&str] = &["if", "unless"];
    for kw in KEYWORDS {
        if line[first_non_ws..].starts_with(kw) {
            let mut idx = first_non_ws + kw.len();
            let mut consumed = 0usize;
            let bytes = line.as_bytes();
            while idx < bytes.len() && matches!(bytes[idx], b' ' | b'\t') {
                idx += 1;
                consumed += 1;
            }

            if idx == line.len() {
                return Cow::Borrowed(line);
            }

            if consumed == 1 && matches!(bytes[idx - 1], b' ') {
                return Cow::Borrowed(line);
            }

            let mut normalized = String::with_capacity(line.len() - consumed + 1);
            normalized.push_str(&line[..first_non_ws + kw.len()]);
            normalized.push(' ');
            normalized.push_str(&line[idx..]);
            return Cow::Owned(normalized);
        }
    }

    Cow::Borrowed(line)
}

fn normalize_expression_spacing(input: &str) -> Cow<'_, str> {
    let mut result = String::with_capacity(input.len());
    let mut changed = false;
    let mut chars = input.chars().peekable();
    let mut in_string: Option<char> = None;
    let mut escape = false;
    let mut in_lambda = false;
    let mut bracket_stack: Vec<char> = Vec::new();

    while let Some(ch) = chars.next() {
        if let Some(delim) = in_string {
            result.push(ch);
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                _ if ch == delim => in_string = None,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' | '\'' | '`' => {
                result.push(ch);
                in_string = Some(ch);
            }
            '.' => {
                if trim_trailing_spaces(&mut result) {
                    changed = true;
                }
                let mut removed = false;
                while matches!(chars.peek(), Some(' ' | '\t')) {
                    chars.next();
                    removed = true;
                }
                if removed {
                    changed = true;
                }
                result.push('.');
            }
            '(' => {
                if trim_trailing_spaces(&mut result) {
                    changed = true;
                }
                result.push('(');
                let mut removed = false;
                while matches!(chars.peek(), Some(' ' | '\t')) {
                    chars.next();
                    removed = true;
                }
                if removed {
                    changed = true;
                }
                bracket_stack.push('(');
            }
            ')' => {
                if trim_trailing_spaces(&mut result) {
                    changed = true;
                }
                result.push(')');
                if matches!(bracket_stack.pop(), Some('(')) {}
            }
            '{' => {
                if trim_trailing_spaces(&mut result) {
                    changed = true;
                }
                let prev_non_space = last_non_space(&result).map(|(_, ch)| ch);
                if prev_non_space
                    .map(|ch| matches!(ch, '=' | ':'))
                    .unwrap_or(false)
                    && !result.ends_with(' ')
                {
                    result.push(' ');
                    changed = true;
                }
                if prev_non_space
                    .map(|ch| !matches!(ch, ' ' | '\n' | '\t' | '\r'))
                    .unwrap_or(false)
                {
                    if !result.ends_with(' ') {
                        result.push(' ');
                        changed = true;
                    }
                }
                result.push('{');
                let mut removed = false;
                while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
                    chars.next();
                    removed = true;
                }
                if removed {
                    changed = true;
                }
                if let Some(next) = chars.peek() {
                    if !matches!(next, '}' | '\n' | '\r') && !result.ends_with(' ') {
                        result.push(' ');
                        changed = true;
                    }
                }
                bracket_stack.push('{');
            }
            '}' => {
                if trim_trailing_spaces(&mut result) {
                    changed = true;
                }
                if result
                    .chars()
                    .rev()
                    .find(|ch| !ch.is_whitespace())
                    .map(|ch| ch != '{')
                    .unwrap_or(false)
                    && !result.ends_with(' ')
                {
                    result.push(' ');
                    changed = true;
                }
                result.push('}');
                if matches!(bracket_stack.pop(), Some('{')) {}
            }
            '[' => {
                let prev_non_space = last_non_space(&result).map(|(_, ch)| ch);
                if !matches!(prev_non_space, Some(',')) && trim_trailing_spaces(&mut result) {
                    changed = true;
                }
                let needs_space_before = last_non_space(&result)
                    .map(|(_, ch)| matches!(ch, '=' | ':'))
                    .unwrap_or(false);
                if needs_space_before {
                    result.push(' ');
                    changed = true;
                }
                result.push('[');
                let mut removed = false;
                while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
                    chars.next();
                    removed = true;
                }
                if removed {
                    changed = true;
                }
                bracket_stack.push('[');
            }
            ']' => {
                if trim_trailing_spaces(&mut result) {
                    changed = true;
                }
                result.push(']');
                if matches!(bracket_stack.pop(), Some('[')) {}
            }
            '|' => {
                let was_in_lambda = in_lambda;
                let lambda_bar = detect_lambda_bar(&result, &chars, in_lambda);

                if lambda_bar {
                    if was_in_lambda {
                        if trim_trailing_spaces(&mut result) {
                            changed = true;
                        }
                    } else if let Some((_, prev)) = last_non_space(&result) {
                        if !matches!(prev, ' ' | '\n' | '|' | '(' | '[' | '{') {
                            if !result.ends_with(' ') {
                                result.push(' ');
                                changed = true;
                            }
                        }
                    }
                    while result.ends_with('\t') {
                        result.pop();
                        changed = true;
                    }
                    result.push('|');
                    in_lambda = !in_lambda;
                    let mut removed = false;
                    while matches!(chars.peek(), Some(c) if c.is_whitespace() && *c != '\n') {
                        chars.next();
                        removed = true;
                    }
                    if removed {
                        changed = true;
                    }

                    if !in_lambda {
                        if let Some(&next) = chars.peek() {
                            if next == '=' {
                                result.push(' ');
                                changed = true;
                            } else if !matches!(next, ' ' | '\n' | ')' | '}' | ']' | ',' | ';') {
                                result.push(' ');
                                changed = true;
                            }
                        }
                    }

                    continue;
                }

                if ensure_space_before_binary(&mut result) {
                    changed = true;
                }
                result.push('|');
                if ensure_space_after_binary(&mut result, &mut chars) {
                    changed = true;
                }
                continue;
            }
            '+' | '-' | '*' | '/' | '%' | '=' | '!' | '<' | '>' => {
                if ch == '-' {
                    if let Some('>') = chars.peek() {
                        if trim_trailing_spaces(&mut result) {
                            changed = true;
                        }
                        if !result.is_empty() && !result.ends_with(' ') {
                            result.push(' ');
                            changed = true;
                        }

                        result.push_str("->");
                        chars.next();

                        let mut removed_ws = false;
                        while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
                            chars.next();
                            removed_ws = true;
                        }
                        if removed_ws {
                            changed = true;
                        }

                        if let Some(&next) = chars.peek() {
                            if !matches!(next, ')' | ']' | '}' | ',' | ';') {
                                if !result.ends_with(' ') {
                                    result.push(' ');
                                    changed = true;
                                }
                            }
                        }

                        continue;
                    }
                }

                if ch == '=' {
                    match chars.peek().copied() {
                        Some('=') => {
                            if ensure_space_before_binary(&mut result) {
                                changed = true;
                            }
                            result.push('=');
                            chars.next();
                            result.push('=');
                            if ensure_space_after_binary(&mut result, &mut chars) {
                                changed = true;
                            }
                            continue;
                        }
                        Some('>') => {
                            result.push('=');
                            continue;
                        }
                        _ => {
                            if matches!(
                                last_non_space(&result).map(|(_, c)| c),
                                Some('!' | '<' | '>' | '=')
                            ) {
                                result.push('=');
                                continue;
                            }
                        }
                    }
                }

                if ch == '!' && !matches!(chars.peek(), Some('=')) {
                    if trim_trailing_spaces(&mut result) {
                        changed = true;
                    }
                    result.push('!');
                    continue;
                }

                if finalize_comparison_operator(ch, &mut result, &mut chars) {
                    changed = true;
                    continue;
                }

                let prev_info = last_non_space(&result);
                let mut lookahead = chars.clone();
                let mut next_char = None;
                while let Some(next) = lookahead.next() {
                    if next.is_whitespace() {
                        continue;
                    }
                    next_char = Some(next);
                    break;
                }

                if is_binary_operator_context(prev_info, next_char, ch, &result) {
                    if ensure_space_before_binary(&mut result) {
                        changed = true;
                    }
                    result.push(ch);
                    if ensure_space_after_binary(&mut result, &mut chars) {
                        changed = true;
                    }
                } else {
                    result.push(ch);
                }
            }
            ':' => {
                if trim_trailing_spaces(&mut result) {
                    changed = true;
                }
                result.push(':');
                let mut consumed_ws = false;
                while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
                    chars.next();
                    consumed_ws = true;
                }
                if consumed_ws {
                    changed = true;
                }
                if let Some(&next) = chars.peek() {
                    let is_closing = matches!(next, ',' | ')' | ']' | '}' | '\n' | '\r');
                    if !is_closing {
                        if !result.ends_with(' ') {
                            result.push(' ');
                            changed = true;
                        }
                    }
                }
            }
            ',' => {
                if trim_trailing_spaces(&mut result) {
                    changed = true;
                }
                result.push(',');
                let mut consumed_ws = false;
                while matches!(chars.peek(), Some(' ' | '\t')) {
                    chars.next();
                    consumed_ws = true;
                }
                if consumed_ws {
                    changed = true;
                }

                let next_char = chars.peek().copied();
                if let Some(next) = next_char {
                    if matches!(next, '\n' | '\r' | ']' | '}' | ')') {
                        // do not inject a trailing space before structural closures or newlines
                    } else if !result.ends_with(' ') {
                        result.push(' ');
                        changed = true;
                    }
                }
            }
            _ => result.push(ch),
        }
    }

    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(input)
    }
}

fn find_comment_index(line: &str) -> Option<usize> {
    let mut in_string: Option<char> = None;
    let mut escape = false;

    for (idx, ch) in line.char_indices() {
        if let Some(delim) = in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                _ if ch == delim => in_string = None,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' | '\'' | '`' => in_string = Some(ch),
            '#' => return Some(idx),
            _ => {}
        }
    }

    None
}

fn last_non_space(text: &str) -> Option<(usize, char)> {
    text.char_indices()
        .rev()
        .find(|&(_, ch)| !ch.is_whitespace())
}

fn trim_trailing_spaces(buffer: &mut String) -> bool {
    let mut changed = false;
    while buffer
        .as_bytes()
        .last()
        .map(|b| matches!(b, b' ' | b'\t'))
        .unwrap_or(false)
    {
        buffer.pop();
        changed = true;
    }
    changed
}

fn ensure_space_before_binary(buffer: &mut String) -> bool {
    let mut changed = trim_trailing_spaces(buffer);
    if let Some(last) = buffer.chars().last() {
        if last != ' ' && last != '\n' {
            buffer.push(' ');
            changed = true;
        }
    }
    changed
}

fn ensure_space_after_binary(
    buffer: &mut String,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> bool {
    let mut changed = false;
    let mut consumed_ws = false;
    while matches!(chars.peek(), Some(c) if matches!(c, ' ' | '\t')) {
        chars.next();
        consumed_ws = true;
    }
    if consumed_ws {
        changed = true;
    }

    let upcoming = chars.peek().copied();
    if matches!(upcoming, Some(c) if !matches!(c, ')' | ']' | '}' | ',' | ';' | '\n' | '\r')) {
        if !buffer.ends_with(' ') {
            buffer.push(' ');
            changed = true;
        }
    }

    changed
}

fn detect_lambda_bar(
    _buffer: &str,
    chars: &std::iter::Peekable<std::str::Chars<'_>>,
    in_lambda: bool,
) -> bool {
    if in_lambda {
        return true;
    }

    let mut iter = chars.clone();
    while let Some(next) = iter.next() {
        match next {
            '|' => {
                let mut arrow_iter = iter.clone();
                while let Some(ch) = arrow_iter.next() {
                    if ch.is_whitespace() {
                        continue;
                    }
                    if ch == '=' {
                        if let Some('>') = arrow_iter.next() {
                            return true;
                        }
                    }
                    break;
                }
                return true;
            }
            '\n' | '\r' => break,
            ')' | ']' | '}' => break,
            _ => {}
        }
    }

    false
}

fn normalize_comparison_operator_spacing(line: &str) -> Cow<'_, str> {
    if !(line.contains("==") || line.contains("!=") || line.contains("<=") || line.contains(">=")) {
        return Cow::Borrowed(line);
    }

    let mut result = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string: Option<char> = None;
    let mut escape = false;
    let mut changed = false;

    while let Some(ch) = chars.next() {
        if let Some(delim) = in_string {
            result.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == delim {
                in_string = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => {
                result.push(ch);
                in_string = Some(ch);
            }
            '=' => {
                if matches!(chars.peek(), Some('=')) {
                    changed = normalize_operator_pair(&mut result, &mut chars, "==") || changed;
                } else {
                    result.push('=');
                }
            }
            '!' => {
                if matches!(chars.peek(), Some('=')) {
                    changed = normalize_operator_pair(&mut result, &mut chars, "!=") || changed;
                } else {
                    result.push('!');
                }
            }
            '<' => {
                if matches!(chars.peek(), Some('=')) {
                    changed = normalize_operator_pair(&mut result, &mut chars, "<=") || changed;
                } else {
                    result.push('<');
                }
            }
            '>' => {
                if matches!(chars.peek(), Some('=')) {
                    changed = normalize_operator_pair(&mut result, &mut chars, ">=") || changed;
                } else {
                    result.push('>');
                }
            }
            _ => result.push(ch),
        }
    }

    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(line)
    }
}

fn normalize_operator_pair(
    buffer: &mut String,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    op: &str,
) -> bool {
    let mut changed = trim_trailing_spaces(buffer);
    if let Some(last) = buffer.chars().last() {
        if last != ' ' && last != '\n' {
            buffer.push(' ');
            changed = true;
        }
    }

    buffer.push_str(op);
    chars.next();

    let mut consumed_ws = false;
    while matches!(chars.peek(), Some(c) if matches!(c, ' ' | '\t')) {
        chars.next();
        consumed_ws = true;
        changed = true;
    }

    if let Some(next) = chars.peek().copied() {
        if !matches!(next, ')' | ']' | '}' | ',' | ';' | '\n' | '\r') {
            if !buffer.ends_with(' ') {
                buffer.push(' ');
                changed = true;
            }
        }
    }

    if !consumed_ws && !buffer.ends_with(' ') {
        buffer.push(' ');
        changed = true;
    }

    changed
}

fn normalize_case_arrow_spacing(line: &str) -> Cow<'_, str> {
    if !line.contains("=>") {
        return Cow::Borrowed(line);
    }

    let mut result = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string: Option<char> = None;
    let mut escape = false;
    let mut changed = false;

    while let Some(ch) = chars.next() {
        if let Some(delim) = in_string {
            result.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == delim {
                in_string = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => {
                result.push(ch);
                in_string = Some(ch);
            }
            '=' => {
                // look ahead ignoring whitespace
                let mut iter = chars.clone();
                while let Some(next) = iter.peek() {
                    if next.is_whitespace() {
                        iter.next();
                        continue;
                    }
                    break;
                }

                let mut iter2 = iter.clone();
                if let Some('>') = iter2.next() {
                    // consume trailing spaces in result
                    while result.ends_with(' ') {
                        result.pop();
                        changed = true;
                    }

                    // emit normalized arrow
                    result.push(' ');
                    result.push('=');
                    result.push('>');

                    // consume characters from original iterator up to and including '>'
                    while let Some(next) = chars.peek() {
                        if next.is_whitespace() {
                            chars.next();
                            changed = true;
                            continue;
                        }
                        if *next == '>' {
                            chars.next();
                            break;
                        }
                        break;
                    }

                    // consume whitespace after arrow
                    let mut consumed_ws = false;
                    while let Some(next) = chars.peek() {
                        if next.is_whitespace() {
                            chars.next();
                            consumed_ws = true;
                            changed = true;
                        } else {
                            break;
                        }
                    }

                    if !consumed_ws {
                        if !matches!(chars.peek(), Some(c) if matches!(c, '>' | ')' | ']' | '}' | ',' | ';' | '\n' | '\r'))
                        {
                            result.push(' ');
                            changed = true;
                        }
                    } else {
                        if !result.ends_with(' ') {
                            result.push(' ');
                        }
                    }

                    continue;
                } else {
                    result.push('=');
                }
            }
            _ => result.push(ch),
        }
    }

    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(line)
    }
}

fn normalize_struct_declaration_spacing(line: &str) -> Cow<'_, str> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("struct") {
        return Cow::Borrowed(line);
    }

    let leading_len = line.len() - trimmed.len();
    let leading_ws = &line[..leading_len];

    let mut rest = &trimmed["struct".len()..];
    if !rest
        .chars()
        .next()
        .map(|c| c.is_whitespace())
        .unwrap_or(false)
    {
        return Cow::Borrowed(line);
    }
    rest = rest.trim_start();
    if rest.is_empty() {
        return Cow::Borrowed(line);
    }

    // Parse identifier
    let mut name_end = rest.len();
    for (idx, ch) in rest.char_indices() {
        if ch.is_whitespace() || matches!(ch, '[' | '{') {
            name_end = idx;
            break;
        }
    }
    let name = &rest[..name_end];
    if name.is_empty() {
        return Cow::Borrowed(line);
    }
    let mut idx = name_end;
    while idx < rest.len() {
        let ch = rest[idx..].chars().next().unwrap();
        if ch.is_whitespace() {
            idx += ch.len_utf8();
        } else {
            break;
        }
    }

    // Optional generics
    let mut generics = String::new();
    if idx < rest.len() && rest[idx..].starts_with('[') {
        let mut depth = 0i32;
        let mut closing = None;
        for (offset, ch) in rest[idx..].char_indices() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        closing = Some(idx + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let closing = match closing {
            Some(c) => c,
            None => return Cow::Borrowed(line),
        };
        let content = &rest[idx + 1..closing];
        generics.push('[');
        generics.push_str(
            &content
                .split(',')
                .map(|part| part.trim())
                .collect::<Vec<_>>()
                .join(", "),
        );
        generics.push(']');
        idx = closing + 1;
        while idx < rest.len() {
            let ch = rest[idx..].chars().next().unwrap();
            if ch.is_whitespace() {
                idx += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    let mut normalized = String::with_capacity(line.len());
    normalized.push_str(leading_ws);
    normalized.push_str("struct ");
    normalized.push_str(name);
    normalized.push_str(&generics);

    let remainder = &rest[idx..];
    let mut did_change = false;

    if let Some(after_brace) = remainder.strip_prefix('{') {
        normalized.push_str(" {");
        let trimmed_after = after_brace.trim_start_matches([' ', '\t']);
        if trimmed_after.len() != after_brace.len() {
            did_change = true;
        }
        if !trimmed_after.is_empty()
            && !matches!(trimmed_after.chars().next(), Some('\n' | '\r' | '}'))
        {
            normalized.push(' ');
        }
        normalized.push_str(trimmed_after);
    } else if !remainder.is_empty() {
        if !matches!(remainder.chars().next(), Some('\n' | '\r')) {
            normalized.push(' ');
        }
        normalized.push_str(remainder);
    }

    if did_change || normalized != line {
        Cow::Owned(normalized)
    } else {
        Cow::Borrowed(line)
    }
}

fn normalize_enum_declaration_spacing(line: &str) -> Cow<'_, str> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("enum") {
        return Cow::Borrowed(line);
    }

    let leading_len = line.len() - trimmed.len();
    let leading_ws = &line[..leading_len];

    let mut rest = &trimmed["enum".len()..];
    if !rest
        .chars()
        .next()
        .map(|c| c.is_whitespace())
        .unwrap_or(false)
    {
        return Cow::Borrowed(line);
    }
    rest = rest.trim_start();
    if rest.is_empty() {
        return Cow::Borrowed(line);
    }

    let mut name_end = rest.len();
    for (idx, ch) in rest.char_indices() {
        if ch.is_whitespace() || ch == '{' {
            name_end = idx;
            break;
        }
    }
    let name = &rest[..name_end];
    if name.is_empty() {
        return Cow::Borrowed(line);
    }
    let mut idx = name_end;
    while idx < rest.len() {
        let ch = rest[idx..].chars().next().unwrap();
        if ch.is_whitespace() {
            idx += ch.len_utf8();
        } else {
            break;
        }
    }

    let mut normalized = String::with_capacity(line.len());
    normalized.push_str(leading_ws);
    normalized.push_str("enum ");
    normalized.push_str(name);

    let remainder = &rest[idx..];
    let mut did_change = false;

    if let Some(after_brace) = remainder.strip_prefix('{') {
        normalized.push_str(" {");
        let trimmed_after = after_brace.trim_start_matches([' ', '\t']);
        if trimmed_after.len() != after_brace.len() {
            did_change = true;
        }
        if !trimmed_after.is_empty()
            && !matches!(trimmed_after.chars().next(), Some('\n' | '\r' | '}'))
        {
            normalized.push(' ');
        }
        normalized.push_str(trimmed_after);
    } else if !remainder.is_empty() {
        if !matches!(remainder.chars().next(), Some('\n' | '\r')) {
            normalized.push(' ');
        }
        normalized.push_str(remainder);
    }

    if did_change || normalized != line {
        Cow::Owned(normalized)
    } else {
        Cow::Borrowed(line)
    }
}

fn finalize_comparison_operator(
    ch: char,
    buffer: &mut String,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> bool {
    match ch {
        '!' => {
            if matches!(chars.peek(), Some('=')) {
                if ensure_space_before_binary(buffer) {
                    buffer.push(' ');
                }
                buffer.push('!');
                chars.next();
                buffer.push('=');
                ensure_space_after_binary(buffer, chars)
            } else {
                false
            }
        }
        '<' => {
            if matches!(chars.peek(), Some('=')) {
                let mut changed = ensure_space_before_binary(buffer);
                buffer.push('<');
                chars.next();
                buffer.push('=');
                if ensure_space_after_binary(buffer, chars) {
                    changed = true;
                }
                changed
            } else {
                false
            }
        }
        '>' => match chars.peek().copied() {
            Some('=') => {
                let mut changed = ensure_space_before_binary(buffer);
                buffer.push('>');
                chars.next();
                buffer.push('=');
                if ensure_space_after_binary(buffer, chars) {
                    changed = true;
                }
                changed
            }
            _ => false,
        },
        _ => false,
    }
}

fn is_binary_operator_context(
    prev: Option<(usize, char)>,
    next: Option<char>,
    op: char,
    before: &str,
) -> bool {
    if op == '-' {
        if matches!(next, Some('>')) {
            return false;
        }
        if is_exponent_notation(prev, before) {
            return false;
        }
    }

    if op == '=' {
        if matches!(next, Some('=' | '>')) {
            return false;
        }
        if prev
            .map(|(_, ch)| matches!(ch, '!' | '<' | '>' | '='))
            .unwrap_or(false)
        {
            return false;
        }
    }

    let prev_ok = prev.map(|(_, ch)| is_binary_prev_char(ch)).unwrap_or(false);
    let next_ok = next.map(is_binary_next_char).unwrap_or(false);

    prev_ok && next_ok
}

fn is_binary_prev_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || matches!(ch, ')' | ']' | '}' | '"' | '\'' | '`')
}

fn is_binary_next_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || ch == '_'
        || matches!(ch, '(' | '[' | '{' | '"' | '\'' | '`')
        || matches!(ch, ')' | ']' | '}')
}

fn is_exponent_notation(prev: Option<(usize, char)>, before: &str) -> bool {
    let (idx, _) = match prev {
        Some(data) if matches!(data.1, 'e' | 'E') => data,
        _ => return false,
    };

    let mut saw_digit = false;
    for prior in before[..idx].chars().rev() {
        if prior.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        if matches!(prior, '_' | '.') {
            continue;
        }
        break;
    }

    saw_digit
}

struct LineMetrics {
    leading_closers: usize,
    net_bracket_change: isize,
}

fn split_code_and_comment(line: &str) -> (&str, Option<&str>) {
    let mut in_string: Option<char> = None;
    let mut escape = false;

    for (idx, ch) in line.char_indices() {
        if let Some(delim) = in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                _ if ch == delim => in_string = None,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' | '\'' | '`' => in_string = Some(ch),
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
        "def", "if", "unless", "for", "while", "until", "test", "else", "match",
    ];

    if BLOCK_KEYWORDS
        .iter()
        .any(|kw| line_starts_with_keyword(code, kw))
    {
        return true;
    }

    if line_starts_with_keyword(code, "pub") {
        let rest = code["pub".len()..].trim_start();
        const PUB_KEYWORDS: &[&str] = &["def", "struct"];
        return PUB_KEYWORDS
            .iter()
            .any(|kw| line_starts_with_keyword(rest, kw));
    }

    false
}

fn is_function_header(code: &str) -> bool {
    if line_starts_with_keyword(code, "def") {
        return true;
    }

    if line_starts_with_keyword(code, "pub") {
        let rest = code["pub".len()..].trim_start();
        return line_starts_with_keyword(rest, "def");
    }

    false
}

fn is_conditional_header(code: &str) -> bool {
    if line_starts_with_keyword(code, "if") || line_starts_with_keyword(code, "unless") {
        return true;
    }

    false
}

fn is_test_header(code: &str) -> bool {
    line_starts_with_keyword(code, "test")
}

fn is_inline_match_expression(code: &str) -> bool {
    let trimmed = code.trim_start();
    if trimmed.starts_with("match") {
        return false;
    }

    let mut in_string: Option<char> = None;
    let mut escape = false;
    let mut iter = code.char_indices().peekable();

    while let Some((idx, ch)) = iter.next() {
        if let Some(delim) = in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == delim {
                in_string = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' | '`' => {
                in_string = Some(ch);
                continue;
            }
            _ => {}
        }

        if code[idx..].starts_with("match") {
            let after_idx = idx + "match".len();
            let after_char = code[after_idx..].chars().next();
            if after_char
                .map(|c| c.is_alphanumeric() || c == '_')
                .unwrap_or(false)
            {
                continue;
            }

            let prev_non_ws = code[..idx].chars().rev().find(|c| !c.is_whitespace());
            if let Some(prev) = prev_non_ws {
                if matches!(prev, '=' | '(' | '[' | '{' | ',' | '>') {
                    return true;
                }
            }
        }
    }

    false
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

    if trimmed.ends_with("&&") || trimmed.ends_with("||") {
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

    0
}
