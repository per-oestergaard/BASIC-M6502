/// Stage 1 parser — converts raw MACRO-10 source text into Vec<SourceNode>.
///
/// Responsibilities:
///   1. Strip COMMENT block spans from the raw source (whole-source pass).
///   2. Strip inline `;` comments from each line.
///   3. Parse each line into a SourceNode using a recursive-descent approach.
///   4. Collect multi-line bodies for DEFINE, IFE/IFN/IFNDEF/IF1/REPEAT using
///      a depth-counter state machine — *never* regex.
///
/// The parser is purely structural: it does NOT evaluate any expression,
/// look up any symbol value, or make decisions based on symbol state.
use crate::ast::{CondKind, SourceNode};
use anyhow::{bail, Result};
use tracing::trace;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse raw MACRO-10 source into an AST.
pub fn parse(source: &str) -> Result<Vec<SourceNode>> {
    let cleaned = strip_block_comments(source);
    let lines: Vec<String> = cleaned.lines().map(|l| l.to_string()).collect();
    let mut pos = 0usize;
    parse_block(&lines, &mut pos, None)
}

// ---------------------------------------------------------------------------
// Block comment removal (whole-source, pre-pass)
// ---------------------------------------------------------------------------

/// Strip all COMMENT spans from the source.
///
/// Grammar: `COMMENT X … X`
///   - X is the first non-space character after the keyword `COMMENT`.
///   - Everything from X up to (and including) the next line that consists
///     solely of X (possibly surrounded by whitespace) is discarded.
fn strip_block_comments(src: &str) -> String {
    let mut result = String::with_capacity(src.len());
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(delim) = detect_comment_start(line) {
            trace!(target: "assembler6502::parser", line = i, ?delim, "block comment start");
            // Skip everything until we find a line that is exactly `delim`
            i += 1;
            while i < lines.len() {
                let trimmed = lines[i].trim();
                if trimmed == delim.to_string().as_str() {
                    trace!(target: "assembler6502::parser", line = i, "block comment end");
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else {
            result.push_str(line);
            result.push('\n');
            i += 1;
        }
    }
    result
}

/// If `line` is a COMMENT directive, return the delimiter character.
fn detect_comment_start(line: &str) -> Option<char> {
    let trimmed = line.trim();
    // Case-insensitive match of the word "COMMENT"
    if trimmed.len() < 7 {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("COMMENT") {
        return None;
    }
    let rest = &trimmed[7..];
    // Must be followed by whitespace then the delimiter
    let delim_char = rest.chars().find(|c| !c.is_whitespace())?;
    Some(delim_char)
}

// ---------------------------------------------------------------------------
// Inline comment stripping
// ---------------------------------------------------------------------------

/// Strip the inline comment (first `;` and everything after).
fn strip_inline_comment(line: &str) -> &str {
    // Walk character by character so we can skip over quoted strings.
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b';' {
            return &line[..i];
        }
        // Skip over double-quoted strings (DC / DCI directives)
        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
        }
        i += 1;
    }
    line
}

// ---------------------------------------------------------------------------
// Body collection using depth-counter state machine
// ---------------------------------------------------------------------------

/// Collect the text of an inline body that starts with `<` and ends at the
/// matching `>`.  The opening `<` has already been consumed (or the caller
/// found it at `start_pos` inside `text`).
///
/// Returns `(body_content, chars_consumed_including_closing_angle)`.
fn collect_angle_body(text: &str) -> String {
    let mut depth = 1usize;
    let mut body = String::new();
    for ch in text.chars() {
        match ch {
            '<' => {
                depth += 1;
                body.push(ch);
            }
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return body;
                }
                body.push(ch);
            }
            _ => body.push(ch),
        }
    }
    // Unclosed — return what we have
    body
}

/// Collect a multi-line body from `lines[pos..]` where the opening `<` was
/// the last non-whitespace character on the previous line.
///
/// Reads lines until we encounter one that is exactly `>` (possibly
/// surrounded by whitespace).  Returns the accumulated non-brace lines as a
/// `Vec<String>` and advances `pos` past the closing `>` line.
fn collect_multiline_body(lines: &[String], pos: &mut usize) -> Vec<String> {
    let mut body_lines: Vec<String> = Vec::new();
    // Track depth in case there are nested angle brackets in sub-macros.
    let mut depth = 1usize;
    while *pos < lines.len() {
        let line = lines[*pos].clone();
        *pos += 1;
        let trimmed = line.trim();
        // Count angle brackets on this line for depth tracking
        for ch in trimmed.chars() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        return body_lines;
                    }
                }
                _ => {}
            }
        }
        // If this line is just `>` at depth-0 we return above.
        // Otherwise record it.
        body_lines.push(line);
    }
    body_lines
}

// ---------------------------------------------------------------------------
// Recursive block parser
// ---------------------------------------------------------------------------

/// Parse a sequence of lines starting at `pos`.
///
/// `stop_keyword` — when set, stop and return when we see a bare line that
/// is that keyword (used by ELSE/ENDIF handling).
fn parse_block(
    lines: &[String],
    pos: &mut usize,
    stop_keyword: Option<&str>,
) -> Result<Vec<SourceNode>> {
    let mut nodes: Vec<SourceNode> = Vec::new();

    while *pos < lines.len() {
        let raw = lines[*pos].clone();
        let line = strip_inline_comment(&raw).trim().to_string();

        // Check for stop keywords (ELSE / ENDIF)
        if let Some(kw) = stop_keyword {
            let upper = line.to_ascii_uppercase();
            let first_token = upper.split_whitespace().next().unwrap_or("");
            if first_token == kw {
                return Ok(nodes);
            }
            // Also stop on ELSE when parsing the then-block
            if kw == "ELSE" && first_token == "ELSE" {
                return Ok(nodes);
            }
        }

        *pos += 1;

        if line.is_empty() {
            continue;
        }

        if let Some(node) = parse_one_line(&line, lines, pos)? {
            nodes.push(node);
        }
    }

    Ok(nodes)
}

// ---------------------------------------------------------------------------
// Single-line parser
// ---------------------------------------------------------------------------

fn parse_one_line(
    line: &str,
    lines: &[String],
    pos: &mut usize,
) -> Result<Option<SourceNode>> {
    // Split off optional label (token ending in `:` or `::`)
    let (label, rest) = split_label(line);
    let rest = rest.trim();

    if rest.is_empty() {
        // Line is just a label
        if let Some(lbl) = label {
            return Ok(Some(SourceNode::Label { name: lbl }));
        }
        return Ok(None);
    }

    // Get first token (mnemonic / directive)
    let (mnemonic_raw, after_mnem) = split_first_token(rest);
    let mnemonic = mnemonic_raw.to_ascii_uppercase();
    let after_mnem = after_mnem.trim();

    // -----------------------------------------------------------------------
    // Skip-only directives (produce no nodes)
    // -----------------------------------------------------------------------
    match mnemonic.as_str() {
        "TITLE" | "SUBTTL" | "SEARCH" | "SALL" | "RADIX" | "PAGE" | "PRINTX"
        | "XLIST" | "LIST" | "PURGE" | "IF2" | ".XCREF" | ".CREF" => {
            return Ok(None);
        }
        "COMMENT" => {
            // Inline COMMENT on a single line — we already stripped block
            // comments in the pre-pass, so anything left here is safe to skip.
            return Ok(None);
        }
        "ELSE" | "ENDIF" => {
            // These are handled by the recursive parse_block caller.
            return Ok(None);
        }
        _ => {}
    }

    // -----------------------------------------------------------------------
    // DEFINE name [(params)],<body>
    // -----------------------------------------------------------------------
    if mnemonic == "DEFINE" {
        return Ok(parse_define(label, after_mnem, lines, pos)?);
    }

    // -----------------------------------------------------------------------
    // Equates: `REST = expr` or `REST == expr`
    // But `REST` might be the label we already stripped, so we handle the
    // case where rest is `sym = expr` or just `= expr` (label already taken).
    // -----------------------------------------------------------------------
    if let Some(node) = try_parse_equate(label.as_deref(), rest)? {
        return Ok(Some(node));
    }

    // -----------------------------------------------------------------------
    // ORG
    // -----------------------------------------------------------------------
    if mnemonic == "ORG" {
        return Ok(Some(SourceNode::Org {
            expr: after_mnem.to_string(),
        }));
    }

    // -----------------------------------------------------------------------
    // IFE / IFN / IFNDEF / IF1 / IFDEF
    // -----------------------------------------------------------------------
    if matches!(
        mnemonic.as_str(),
        "IFE" | "IFN" | "IFNDEF" | "IFDEF" | "IF1" | "IF2"
    ) {
        return Ok(Some(parse_conditional(
            label,
            &mnemonic,
            after_mnem,
            lines,
            pos,
        )?));
    }

    // -----------------------------------------------------------------------
    // REPEAT n,<body>
    // -----------------------------------------------------------------------
    if mnemonic == "REPEAT" {
        return Ok(Some(parse_repeat(label, after_mnem, lines, pos)?));
    }

    // -----------------------------------------------------------------------
    // IRPC sym,<chars>  — ignore (not used by BASIC source)
    // -----------------------------------------------------------------------
    if mnemonic == "IRPC" {
        // Collect and discard the body
        let (_sym, after_sym) = split_first_token(after_mnem);
        let after_sym = after_sym.trim().trim_start_matches(',').trim();
        if after_sym.starts_with('<') {
            collect_angle_body(&after_sym[1..]);
        } else if after_sym.is_empty() || after_sym == "<" {
            collect_multiline_body(lines, pos);
        }
        return Ok(None);
    }

    // -----------------------------------------------------------------------
    // .byte / .BYTE
    // -----------------------------------------------------------------------
    if mnemonic == ".BYTE" {
        let args = split_comma_args(after_mnem);
        return Ok(Some(SourceNode::Bytes { label, args }));
    }

    // -----------------------------------------------------------------------
    // .word / .WORD
    // -----------------------------------------------------------------------
    if mnemonic == ".WORD" {
        let args = split_comma_args(after_mnem);
        return Ok(Some(SourceNode::Word { label, args }));
    }

    // -----------------------------------------------------------------------
    // .res / .RES / BLOCK
    // -----------------------------------------------------------------------
    if mnemonic == ".RES" || mnemonic == "BLOCK" {
        return Ok(Some(SourceNode::Res {
            label,
            count_expr: after_mnem.to_string(),
        }));
    }

    // -----------------------------------------------------------------------
    // ADR(sym) — synonym for .word sym  (may appear as `ADR(FOO)`)
    // -----------------------------------------------------------------------
    if mnemonic.starts_with("ADR(") || mnemonic == "ADR" {
        let sym = if mnemonic.starts_with("ADR(") {
            // ADR(sym) on the same token
            mnemonic
                .trim_start_matches("ADR(")
                .trim_end_matches(')')
                .to_string()
        } else {
            // ADR sym
            after_mnem.trim_matches(|c| c == '(' || c == ')').to_string()
        };
        return Ok(Some(SourceNode::Word {
            label,
            args: vec![sym],
        }));
    }

    // -----------------------------------------------------------------------
    // DCI "text" — high-bit-last ASCII
    // DC  "text" — plain ASCII
    // -----------------------------------------------------------------------
    if mnemonic == "DCI" || mnemonic == "DC" {
        let high_bit = mnemonic == "DCI";
        let text = after_mnem.trim().trim_matches('"');
        let mut args: Vec<String> = text
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let mut byte = c as u8;
                if high_bit && i == text.len() - 1 {
                    byte |= 0x80;
                }
                format!("${:02X}", byte)
            })
            .collect();
        // DCI: last char already has high bit set above; DC: all plain
        if args.is_empty() {
            args.push("$00".to_string());
        }
        return Ok(Some(SourceNode::Bytes { label, args }));
    }

    // -----------------------------------------------------------------------
    // XWD hi,lo — emit a 16-bit word from two halves
    // -----------------------------------------------------------------------
    if mnemonic == "XWD" {
        let parts: Vec<&str> = after_mnem.splitn(2, ',').collect();
        let hi = parts.first().copied().unwrap_or("0").trim().to_string();
        let lo = parts.get(1).copied().unwrap_or("0").trim().to_string();
        // Emit as a word: (hi << 8) | lo — store both as args
        return Ok(Some(SourceNode::Word {
            label,
            args: vec![format!("(({})&$FF)|(({})<<8)", lo, hi)],
        }));
    }

    // -----------------------------------------------------------------------
    // EXP val — emit a word/byte value
    // -----------------------------------------------------------------------
    if mnemonic == "EXP" {
        return Ok(Some(SourceNode::Word {
            label,
            args: vec![after_mnem.to_string()],
        }));
    }

    // -----------------------------------------------------------------------
    // Everything else is an instruction / macro call.
    // We store it as Instr and let the expander decide if it's a known
    // pseudo-op, a user-defined macro, or a real 6502 opcode.
    // -----------------------------------------------------------------------
    let operand = if after_mnem.is_empty() {
        None
    } else {
        Some(after_mnem.to_string())
    };

    Ok(Some(SourceNode::Instr {
        label,
        mnemonic,
        operand,
    }))
}

// ---------------------------------------------------------------------------
// DEFINE parser
// ---------------------------------------------------------------------------

fn parse_define(
    _label: Option<String>,
    after_mnem: &str,
    lines: &[String],
    pos: &mut usize,
) -> Result<Option<SourceNode>> {
    // Syntax: `DEFINE name [(param,...)],<body>`
    // or multi-line: `DEFINE name [(param,...)],<` then lines then `>`
    let body_lines;

    // Find name and params
    let (name_raw, after_name) = split_first_token(after_mnem);
    let name = name_raw.to_ascii_uppercase();

    // Parse optional parameter list
    let (params, after_params) = parse_macro_params(after_name.trim());

    // Find the `<` that starts the body
    let after_params = after_params.trim().trim_start_matches(',').trim();

    if after_params.starts_with('<') {
        let body_text = after_params[1..].trim().to_string();
        if body_text.is_empty() || !body_text.contains('>') {
            // Multi-line body
            body_lines = collect_multiline_body(lines, pos);
        } else {
            // Inline body
            let body = collect_angle_body(&after_params[1..]);
            body_lines = body.lines().map(|l| l.to_string()).collect();
        }
    } else if after_params.is_empty() || after_params == "<" {
        body_lines = collect_multiline_body(lines, pos);
    } else {
        body_lines = Vec::new();
    }

    trace!(target: "assembler6502::parser", name = %name, lines = body_lines.len(), "DEFINE");

    Ok(Some(SourceNode::MacroDef {
        name,
        params,
        body: body_lines,
    }))
}

/// Parse optional `(param, ...)` after a macro name.
/// Returns `(params, remaining_text)`.
fn parse_macro_params(text: &str) -> (Vec<String>, &str) {
    if !text.starts_with('(') {
        return (Vec::new(), text);
    }
    // Find matching ')'
    let close = text.find(')').unwrap_or(text.len() - 1);
    let inner = &text[1..close];
    let params: Vec<String> = inner
        .split(',')
        .map(|p| p.trim().to_ascii_uppercase())
        .filter(|p| !p.is_empty())
        .collect();
    (&text[close + 1..], &text[close + 1..]);
    let remaining = if close + 1 <= text.len() {
        &text[close + 1..]
    } else {
        ""
    };
    (params, remaining)
}

// ---------------------------------------------------------------------------
// Equate parser
// ---------------------------------------------------------------------------

fn try_parse_equate(label: Option<&str>, rest: &str) -> Result<Option<SourceNode>> {
    // Patterns:
    //   SYM == expr    (strong equate — double equals)
    //   SYM =  expr    (soft equate — single equals)
    // The whole of `rest` might be `SYM = expr` (when label was not on this
    // line) or just `= expr` if the label came from the label field.

    // Double-equals first (to avoid matching single-equals inside `==`)
    if let Some(pos) = rest.find("==") {
        let name = rest[..pos].trim();
        let expr = rest[pos + 2..].trim().to_string();
        let sym = if name.is_empty() {
            label.unwrap_or("").trim_end_matches(':').trim().to_ascii_uppercase()
        } else {
            name.trim_end_matches(':').trim().to_ascii_uppercase()
        };
        if !sym.is_empty() {
            return Ok(Some(SourceNode::Equate { name: sym, expr }));
        }
    }

    // Single equals — must not be part of `==`, `>=`, `<=`, `!=`
    if let Some(eq_pos) = find_single_equals(rest) {
        let name = rest[..eq_pos].trim();
        let expr = rest[eq_pos + 1..].trim().to_string();
        let sym = if name.is_empty() {
            label.unwrap_or("").trim_end_matches(':').trim().to_ascii_uppercase()
        } else {
            name.trim_end_matches(':').trim().to_ascii_uppercase()
        };
        if !sym.is_empty() && !expr.is_empty() {
            return Ok(Some(SourceNode::Equate { name: sym, expr }));
        }
    }

    Ok(None)
}

/// Find the position of a single `=` that is not part of `==`, `<=`, `>=`, `!=`.
fn find_single_equals(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            let prev = if i > 0 { bytes[i - 1] } else { 0 };
            let next = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
            if next != b'=' && prev != b'=' && prev != b'!' && prev != b'<' && prev != b'>' {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// Conditional parser
// ---------------------------------------------------------------------------

fn parse_conditional(
    _label: Option<String>,
    mnemonic: &str,
    after_mnem: &str,
    lines: &[String],
    pos: &mut usize,
) -> Result<SourceNode> {
    let kind = match mnemonic {
        "IFE" => CondKind::IfEq,
        "IFN" => CondKind::IfNe,
        "IFNDEF" | "IFDEF" => CondKind::IfNotDef,
        "IF1" => CondKind::If1,
        "IF2" => CondKind::If2,
        _ => bail!("Unknown conditional: {}", mnemonic),
    };

    // after_mnem is: `expr,<body>` or `expr,<` (multi-line) or just `expr`
    // For IFNDEF the expr is a symbol name.

    // Split condition expression from body
    let (cond_expr, body_text) = split_cond_and_body(after_mnem);

    let (then_body, else_body) = if let Some(bt) = body_text {
        if bt.trim().is_empty() || bt.trim() == "<" {
            // Multi-line body
            let then_lines = collect_multiline_body(lines, pos);
            // Check for ELSE / ENDIF
            let (tb, eb) = parse_if_else_endif(&then_lines, lines, pos)?;
            (tb, eb)
        } else {
            // Inline single-line body
            let body_content = if bt.starts_with('<') {
                collect_angle_body(&bt[1..])
            } else {
                bt.to_string()
            };
            let bl: Vec<String> = body_content.lines().map(|l| l.to_string()).collect();
            let tb = parse_block(&bl, &mut 0, None)?;
            (tb, Vec::new())
        }
    } else {
        // No body on this line — multi-line up to ENDIF
        let tb = parse_until_else_or_endif(lines, pos, false)?;
        let ub = parse_until_else_or_endif(lines, pos, true)?;
        (tb, ub)
    };

    trace!(
        target: "assembler6502::parser",
        ?kind,
        cond = %cond_expr,
        then_nodes = then_body.len(),
        else_nodes = else_body.len(),
        "conditional"
    );

    Ok(SourceNode::Conditional {
        kind,
        expr: cond_expr,
        then_body,
        else_body,
    })
}

/// Parse lines from `pos` until ELSE or ENDIF, returning the then-block.
/// If `up_to_endif` is true, parse until ENDIF (for the else-block).
fn parse_until_else_or_endif(
    lines: &[String],
    pos: &mut usize,
    up_to_endif: bool,
) -> Result<Vec<SourceNode>> {
    let mut nodes = Vec::new();
    while *pos < lines.len() {
        let raw = lines[*pos].clone();
        let line = strip_inline_comment(&raw).trim().to_ascii_uppercase();
        let first = line.split_whitespace().next().unwrap_or("");
        if first == "ENDIF" {
            *pos += 1;
            return Ok(nodes);
        }
        if !up_to_endif && first == "ELSE" {
            *pos += 1;
            return Ok(nodes);
        }
        *pos += 1;
        let stripped = strip_inline_comment(&raw).trim().to_string();
        if stripped.is_empty() {
            continue;
        }
        if let Some(node) = parse_one_line(&stripped, lines, pos)? {
            nodes.push(node);
        }
    }
    Ok(nodes)
}

/// Given the "then" body lines (already stripped), check if there's an
/// ELSE in the `lines` stream and split accordingly.
fn parse_if_else_endif(
    then_lines: &[String],
    lines: &[String],
    pos: &mut usize,
) -> Result<(Vec<SourceNode>, Vec<SourceNode>)> {
    // Parse the then_lines as a block
    let mut p = 0usize;
    let tb = parse_block(then_lines, &mut p, None)?;

    // Now check if the next real line in `lines` is ELSE or ENDIF
    let mut else_body = Vec::new();
    while *pos < lines.len() {
        let raw = lines[*pos].clone();
        let trimmed = strip_inline_comment(&raw).trim().to_ascii_uppercase();
        let first = trimmed.split_whitespace().next().unwrap_or("");
        match first {
            "ELSE" => {
                *pos += 1;
                let else_lines = collect_multiline_body(lines, pos);
                let mut ep = 0usize;
                else_body = parse_block(&else_lines, &mut ep, None)?;
                break;
            }
            "ENDIF" => {
                *pos += 1;
                break;
            }
            "" => {
                *pos += 1;
            }
            _ => break,
        }
    }

    Ok((tb, else_body))
}

/// Split `expr,<body>` into `(expr_text, Some(body_text))`.
/// If there's no `,` the body is None.
fn split_cond_and_body(s: &str) -> (String, Option<&str>) {
    // Find the comma that separates the condition from the body.
    // The condition may itself contain commas inside balanced <>, so we must
    // track depth.
    let bytes = s.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'<' => depth += 1,
            b'>' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            b',' if depth == 0 => {
                let expr = s[..i].trim().to_string();
                let body = &s[i + 1..];
                return (expr, Some(body.trim()));
            }
            _ => {}
        }
        i += 1;
    }
    (s.trim().to_string(), None)
}

// ---------------------------------------------------------------------------
// REPEAT parser
// ---------------------------------------------------------------------------

fn parse_repeat(
    label: Option<String>,
    after_mnem: &str,
    lines: &[String],
    pos: &mut usize,
) -> Result<SourceNode> {
    let (count_expr, body_text) = split_cond_and_body(after_mnem);

    let body = if let Some(bt) = body_text {
        if bt.trim().is_empty() || bt.trim() == "<" {
            let body_lines = collect_multiline_body(lines, pos);
            let mut p = 0usize;
            parse_block(&body_lines, &mut p, None)?
        } else {
            let body_content = if bt.starts_with('<') {
                collect_angle_body(&bt[1..])
            } else {
                bt.to_string()
            };
            let bl: Vec<String> = body_content.lines().map(|l| l.to_string()).collect();
            parse_block(&bl, &mut 0, None)?
        }
    } else {
        Vec::new()
    };

    Ok(SourceNode::Repeat {
        label,
        count_expr,
        body,
    })
}

// ---------------------------------------------------------------------------
// Helper: split label from rest of line
// ---------------------------------------------------------------------------

/// Split the optional label prefix from a line.
/// A label ends with `:` or `::`.  Returns `(label, rest)`.
fn split_label(line: &str) -> (Option<String>, &str) {
    // A label starts at column 0 (no leading space) OR the whole line is
    // `label:`.  In MACRO-10, labels may also appear with a leading space
    // if immediately followed by a mnemonic, but the convention is that
    // column-0 labels have the colon.
    //
    // We look for the first `:` that is not inside `<>` and not inside `"`
    let bytes = line.as_bytes();
    let mut depth = 0usize;
    let mut in_str = false;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'"' => in_str = !in_str,
            b'<' if !in_str => depth += 1,
            b'>' if !in_str && depth > 0 => depth -= 1,
            b':' if !in_str && depth == 0 => {
                let name = line[..i].trim();
                // Must be a valid symbol — letters/digits/_ no spaces
                if is_valid_symbol(name) {
                    let after_colon = &line[i + 1..];
                    // Eat a second colon (global label `::`)
                    let after_colon = after_colon.strip_prefix(':').unwrap_or(after_colon);
                    return (
                        Some(name.to_ascii_uppercase()),
                        after_colon,
                    );
                }
                break;
            }
            b' ' | b'\t' if !in_str && depth == 0 => {
                // If we hit whitespace before finding `:`, there is no label
                break;
            }
            _ => {}
        }
        i += 1;
    }
    (None, line)
}

fn is_valid_symbol(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' && first != '.' && first != '$' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '$')
}

// ---------------------------------------------------------------------------
// Helper: split first whitespace-delimited token
// ---------------------------------------------------------------------------

fn split_first_token(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    let end = s
        .find(|c: char| c.is_whitespace())
        .unwrap_or(s.len());
    (&s[..end], s[end..].trim_start())
}

// ---------------------------------------------------------------------------
// Helper: split comma-delimited args (depth-aware)
// ---------------------------------------------------------------------------

fn split_comma_args(s: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_str = false;
    for ch in s.chars() {
        match ch {
            '"' => {
                in_str = !in_str;
                current.push(ch);
            }
            '<' if !in_str => {
                depth += 1;
                current.push(ch);
            }
            '>' if !in_str && depth > 0 => {
                depth -= 1;
                current.push(ch);
            }
            ',' if !in_str && depth == 0 => {
                args.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    let last = current.trim().to_string();
    if !last.is_empty() {
        args.push(last);
    }
    args
}

