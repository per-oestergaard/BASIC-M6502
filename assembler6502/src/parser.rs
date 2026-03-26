use anyhow::Result;
use tracing::trace;

use crate::ast::{CondKind, SourceNode};

// ---------------------------------------------------------------------------
// Hand-written parser — produces owned SourceNode values.
// ---------------------------------------------------------------------------

/// Parse source text (may be multi-statement) into a list of [`SourceNode`]s.
///
/// Calls [`crate::preprocess::normalize`] internally, so raw source text
/// (with block comments and inline semicolons) may be passed directly.
pub fn parse(src: &str) -> Result<Vec<SourceNode>> {
    use crate::preprocess::normalize;
    use crate::reader::StatementIter;

    let cleaned = normalize(src);
    // Collect owned Strings so the iterator can outlive `cleaned`.
    let stmts: Vec<String> = StatementIter::new(&cleaned).map(str::to_string).collect();

    let mut nodes = Vec::new();
    for stmt in &stmts {
        let s = stmt.trim();
        if s.is_empty() {
            continue;
        }
        trace!(target: "assembler6502::parser", stmt = s, "parsing");
        nodes.extend(parse_one(s)?);
    }
    Ok(nodes)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Return the first keyword token of `s` (uppercase, colons stripped).
/// Stops at whitespace, `,`, `(`, or `"` so that `DCI"END"` → `DCI`.
fn first_keyword_upper(s: &str) -> String {
    s.trim_start()
        .split(|c: char| c.is_ascii_whitespace() || c == ',' || c == '(' || c == '"')
        .next()
        .unwrap_or("")
        .trim_end_matches(':')
        .to_ascii_uppercase()
}

/// Return the portion of `s` that follows its first keyword token.
/// Also stops at `"` so that `DCI"END"` → rest = `"END"`.
fn after_first_keyword(s: &str) -> &str {
    let s = s.trim_start();
    let end = s
        .find(|c: char| c.is_ascii_whitespace() || c == ',' || c == '(' || c == '"')
        .unwrap_or(s.len());
    s[end..].trim_start()
}

/// The byte-offset of the end of the first token in `s`.
/// Stops at whitespace or `"` so that `DCI"END"` gives `fend = 3`.
fn first_token_end(s: &str) -> usize {
    s.find(|c: char| c.is_ascii_whitespace() || c == '"')
        .unwrap_or(s.len())
}

/// Return the byte-offset of the first `=` at angle-bracket depth 0, or
/// `None` if a `:` appears first (label) or if `=` is inside a `"…"` string
/// (e.g. `DCI"="` must NOT be mistaken for an equate).
fn find_eq_depth0(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    for (i, c) in s.char_indices() {
        if in_string {
            if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            ':' if depth == 0 => return None,
            '=' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Classification helpers (used by parse_one / parse_with_label)
// ---------------------------------------------------------------------------

/// True if `kw` (already uppercased) is a 6502 mnemonic or a built-in pseudo-op
/// that the expander knows how to handle.  Used to distinguish `LABEL: INSTR op`
/// (an instruction) from `LABEL: VALUE` (a data-byte expression).
fn is_known_mnemonic_or_directive(kw: &str) -> bool {
    matches!(
        kw,
        // 6502 mnemonics
        "ADC" | "AND" | "ASL" | "BCC" | "BCS" | "BEQ" | "BIT" | "BMI" | "BNE"
        | "BPL" | "BRK" | "BVC" | "BVS" | "CLC" | "CLD" | "CLI" | "CLV"
        | "CMP" | "CPX" | "CPY" | "DEC" | "DEX" | "DEY" | "EOR" | "INC"
        | "INX" | "INY" | "JMP" | "JSR" | "LDA" | "LDX" | "LDY" | "LSR"
        | "NOP" | "ORA" | "PHA" | "PHP" | "PLA" | "PLP" | "ROL" | "ROR"
        | "RTI" | "RTS" | "SBC" | "SEC" | "SED" | "SEI" | "STA" | "STX"
        | "STY" | "TAX" | "TAY" | "TSX" | "TXA" | "TXS" | "TYA"
        // expander pseudo-ops
        | "LDAI" | "LDXI" | "LDYI" | "CMPI" | "CPXI" | "CPYI" | "ADCI"
        | "SBCI" | "ANDI" | "ORAI" | "EORI" | "LDWD" | "LDWX" | "LDXY"
        | "LDWDI" | "LDWXI" | "LDXYI" | "STWD" | "STWX" | "STXY"
        | "LDADY" | "STADY" | "LDADX" | "STADX" | "CMPDY" | "SBCDY" | "ADCDY"
        | "PSHWD" | "PULWD"
        | "CLR" | "COM" | "SYNCHK" | "JEQ" | "JNE" | "JCS" | "JCC"
        | "JMI" | "JPL" | "JVS" | "JVC" | "ACRLF" | "BCCA" | "BCSA"
        | "BEQA" | "BNEA" | "BMIA" | "BPLA" | "BVCA" | "BVSA" | "INCW"
        | "SKIP1" | "SKIP2"
    )
}

/// True if `s` is a bare expression that cannot be an instruction mnemonic:
/// starts with a digit, `$`, `^O`/`^o` (octal), `"` (char literal), or `'`.
fn is_bare_data_expr(s: &str) -> bool {
    let s = s.trim();
    s.starts_with(|c: char| c.is_ascii_digit())
        || s.starts_with('$')
        || s.starts_with("^O")
        || s.starts_with("^o")
        || s.starts_with('"')
        || s.starts_with('\'')
}

/// Split `s` at the first `,` that is at angle-bracket depth 0.
fn split_comma_depth0(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            ',' if depth == 0 => return Some((&s[..i], &s[i + 1..])),
            _ => {}
        }
    }
    None
}

/// Strip the outermost `<…>` pair from a trimmed string, returning the inner
/// content.  Handles nested brackets correctly.
fn strip_outer_angles(s: &str) -> Option<&str> {
    let s = s.trim();
    if !s.starts_with('<') {
        return None;
    }
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[1..i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split a comma-separated argument list, respecting `<>` nesting.
fn split_args(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '<' => {
                depth += 1;
                cur.push(c);
            }
            '>' if depth > 0 => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                let p = cur.trim().to_string();
                if !p.is_empty() {
                    parts.push(p);
                }
                cur = String::new();
            }
            _ => cur.push(c),
        }
    }
    let last = cur.trim().to_string();
    if !last.is_empty() {
        parts.push(last);
    }
    parts
}

// ---------------------------------------------------------------------------
// parse_one — classify and dispatch a single trimmed statement string
// ---------------------------------------------------------------------------

fn parse_one(stmt: &str) -> Result<Vec<SourceNode>> {
    let kw = first_keyword_upper(stmt);

    match kw.as_str() {
        // Conditionals
        "IFE" | "IFEQ" => return parse_conditional(stmt, CondKind::IfEq),
        "IFN" | "IFNE" => return parse_conditional(stmt, CondKind::IfNe),
        "IFNDEF" => return parse_conditional(stmt, CondKind::IfNotDef),
        "IF1" => return parse_conditional(stmt, CondKind::If1),
        "IF2" => return parse_conditional(stmt, CondKind::If2),

        // Macro definition
        "DEFINE" => return parse_macro_def(stmt),

        // Repeat block
        "REPEAT" => return parse_repeat(stmt, None),

        // Origin
        "ORG" => {
            let expr = after_first_keyword(stmt).trim().to_string();
            return Ok(vec![SourceNode::Org { expr }]);
        }

        // Data directives (DT = "define text", same byte output as DC)
        "DC" | "DCI" | "DCE" | "DT" => return parse_dc(stmt, None),
        "BYTE" | "DB" | ".BYTE" => return parse_bytes(stmt, None),
        "WORD" | "DW" | ".WORD" | "XWD" => return parse_words(stmt, None),
        "BLKB" | "BLKW" | "BLOCK" | "RES" | ".RES" => return parse_res(stmt, None),

        // Assembler control / metadata: silently skip
        "TITLE" | "SUBTTL" | "SEARCH" | "SALL" | "XLIST" | "LIST" | "PAGE" | "PURGE" | "RADIX"
        | "PRINTX" | "EXP" | "ADR" | "ASECT" | "DSECT" | "IRPC" | "LET" | "END" => {
            return Ok(vec![]);
        }

        _ => {}
    }

    // --- Label: first token ends with ':' or '::' ---
    let fend = first_token_end(stmt);
    let first_tok = &stmt[..fend];
    if first_tok.ends_with("::") || first_tok.ends_with(':') {
        let label_name = first_tok.trim_end_matches(':').to_string();
        let rest = stmt[fend..].trim_start();
        if rest.is_empty() {
            return Ok(vec![SourceNode::Label { name: label_name }]);
        }
        return parse_with_label(rest, Some(label_name));
    }

    // --- Equate: NAME = EXPR or NAME == EXPR (at depth 0) ---
    if let Some(eq_pos) = find_eq_depth0(stmt) {
        let name = stmt[..eq_pos].trim().to_string();
        // Make sure the name is a single symbol (not a multi-word statement).
        if !name.is_empty() && name.split_ascii_whitespace().count() == 1 {
            let val_start = eq_pos + 1;
            // Skip second '=' for the '==' form.
            let val_start = if stmt.as_bytes().get(val_start) == Some(&b'=') {
                val_start + 1
            } else {
                val_start
            };
            let expr = stmt[val_start..].trim().to_string();
            trace!(target: "assembler6502::parser", %name, %expr, "equate");
            return Ok(vec![SourceNode::Equate { name, expr }]);
        }
    }

    // --- Bare literal (digit / hex / char) → anonymous data byte ---
    if is_bare_data_expr(stmt) {
        return Ok(vec![SourceNode::Bytes {
            label: None,
            args: split_args(stmt),
        }]);
    }
    // --- Unknown first token (not a recognised mnemonic) → data byte ---
    let top_kw = first_keyword_upper(stmt);
    if !is_known_mnemonic_or_directive(&top_kw) {
        return Ok(vec![SourceNode::Bytes {
            label: None,
            args: split_args(stmt),
        }]);
    }
    parse_instr(stmt, None)
}

/// Parse the body of a statement that was preceded by a label.
fn parse_with_label(body: &str, label: Option<String>) -> Result<Vec<SourceNode>> {
    if body.is_empty() {
        return Ok(label
            .into_iter()
            .map(|name| SourceNode::Label { name })
            .collect());
    }

    let kw = first_keyword_upper(body);
    match kw.as_str() {
        "DC" | "DCI" | "DCE" | "DT" => return parse_dc(body, label),
        "BYTE" | "DB" | ".BYTE" => return parse_bytes(body, label),
        "WORD" | "DW" | ".WORD" | "XWD" => return parse_words(body, label),
        "BLKB" | "BLKW" | "BLOCK" | "RES" | ".RES" => return parse_res(body, label),
        "REPEAT" => return parse_repeat(body, label),
        "EQU" | "DEFL" => {
            let name = label.unwrap_or_default();
            let expr = after_first_keyword(body).trim().to_string();
            return Ok(vec![SourceNode::Equate { name, expr }]);
        }
        "TITLE" | "SUBTTL" | "SEARCH" | "SALL" | "XLIST" | "LIST" | "PAGE" | "PURGE" | "RADIX"
        | "PRINTX" | "EXP" | "ADR" | "ASECT" | "DSECT" | "IRPC" => {
            return Ok(label
                .into_iter()
                .map(|name| SourceNode::Label { name })
                .collect());
        }
        _ => {}
    }

    // Known instruction or pseudo-op → instruction.
    if is_known_mnemonic_or_directive(&kw) {
        return parse_instr(body, label);
    }

    // Bare literal (digit / hex / char literal) → data bytes.
    if is_bare_data_expr(body) {
        return Ok(vec![SourceNode::Bytes {
            label,
            args: split_args(body),
        }]);
    }

    // Single token with no unquoted whitespace → symbol used as data byte
    // (e.g. `LINWID: LINLEN` where LINLEN is an equate).
    if !body.contains(|c: char| c.is_ascii_whitespace()) {
        return Ok(vec![SourceNode::Bytes {
            label,
            args: split_args(body),
        }]);
    }

    // Multi-token unknown first keyword → treat as a macro call.
    let arg_str = body[kw.len()..].trim().to_string();
    let arg = if arg_str.is_empty() {
        None
    } else {
        Some(arg_str)
    };
    Ok(vec![SourceNode::MacroCall {
        label,
        name: kw,
        arg,
    }])
}

fn parse_conditional(stmt: &str, kind: CondKind) -> Result<Vec<SourceNode>> {
    let after = after_first_keyword(stmt);

    let (expr_str, body_part): (&str, &str) = if kind == CondKind::If1 || kind == CondKind::If2 {
        // IF1,<body>  — no expression before the body
        let body = after.trim_start_matches(',').trim_start();
        ("", body)
    } else {
        // IFE expr,<body>
        match split_comma_depth0(after) {
            Some((e, b)) => (e, b),
            None => ("", after),
        }
    };

    let body_text = strip_outer_angles(body_part.trim()).unwrap_or(body_part.trim());
    let then_body = parse(body_text)?;
    trace!(target: "assembler6502::parser", ?kind, cond = expr_str, "conditional");
    Ok(vec![SourceNode::Conditional {
        kind,
        expr: expr_str.trim().to_string(),
        then_body,
        else_body: vec![],
    }])
}

fn parse_macro_def(stmt: &str) -> Result<Vec<SourceNode>> {
    let after = after_first_keyword(stmt);

    // Name: up to whitespace, '(', or ','
    let name_end = after
        .find(|c: char| c.is_ascii_whitespace() || c == '(' || c == ',')
        .unwrap_or(after.len());
    let name = after[..name_end].to_ascii_uppercase();
    let rest = after[name_end..].trim_start();

    let (params, after_params): (Vec<String>, &str) = if rest.starts_with('(') {
        let close = rest.find(')').unwrap_or(rest.len().saturating_sub(1));
        let params_str = &rest[1..close];
        let params = params_str
            .split(',')
            .map(|p| p.trim().to_ascii_uppercase())
            .filter(|p| !p.is_empty())
            .collect();
        (params, &rest[close + 1..])
    } else {
        (vec![], rest)
    };

    let after_params = after_params.trim_start();
    let body_src = if after_params.starts_with(',') {
        after_params[1..].trim_start()
    } else {
        after_params
    };
    let body_text = strip_outer_angles(body_src).unwrap_or(body_src);
    let body: Vec<String> = body_text.lines().map(str::to_string).collect();

    trace!(target: "assembler6502::parser", macro_name = %name, params = ?params, "macro def");
    Ok(vec![SourceNode::MacroDef { name, params, body }])
}

fn parse_repeat(stmt: &str, label: Option<String>) -> Result<Vec<SourceNode>> {
    let after = after_first_keyword(stmt);
    let (count_str, body_part) = match split_comma_depth0(after) {
        Some(p) => p,
        None => return Ok(vec![]),
    };
    let body_text = strip_outer_angles(body_part.trim()).unwrap_or(body_part.trim());
    let body = parse(body_text)?;
    Ok(vec![SourceNode::Repeat {
        label,
        count_expr: count_str.trim().to_string(),
        body,
    }])
}

fn parse_dc(stmt: &str, label: Option<String>) -> Result<Vec<SourceNode>> {
    let kw = first_keyword_upper(stmt);
    let rest = after_first_keyword(stmt);
    let set_hi = kw == "DCI";
    if let Some(start) = rest.find('"') {
        let after_q = &rest[start + 1..];
        let end = after_q.rfind('"').unwrap_or(after_q.len());
        let content = &after_q[..end];
        let args: Vec<String> = content
            .bytes()
            .map(|b| {
                let byte = if set_hi { b | 0x80 } else { b };
                format!("${byte:02X}")
            })
            .collect();
        return Ok(vec![SourceNode::Bytes { label, args }]);
    }
    // Fallback: comma-separated list
    Ok(vec![SourceNode::Bytes {
        label,
        args: split_args(rest),
    }])
}

fn parse_bytes(stmt: &str, label: Option<String>) -> Result<Vec<SourceNode>> {
    let rest = after_first_keyword(stmt);
    Ok(vec![SourceNode::Bytes {
        label,
        args: split_args(rest),
    }])
}

fn parse_words(stmt: &str, label: Option<String>) -> Result<Vec<SourceNode>> {
    let rest = after_first_keyword(stmt);
    Ok(vec![SourceNode::Word {
        label,
        args: split_args(rest),
    }])
}

fn parse_res(stmt: &str, label: Option<String>) -> Result<Vec<SourceNode>> {
    let kw = first_keyword_upper(stmt);
    let rest = after_first_keyword(stmt).trim().to_string();
    // BLKW counts 16-bit words; represent the count in bytes.
    let count_expr = if kw == "BLKW" {
        format!("<{rest}>*2")
    } else {
        rest
    };
    Ok(vec![SourceNode::Res { label, count_expr }])
}

fn parse_instr(stmt: &str, label: Option<String>) -> Result<Vec<SourceNode>> {
    let stmt = stmt.trim();
    let (mnem, operand) = match stmt.find(|c: char| c.is_ascii_whitespace()) {
        Some(i) => (&stmt[..i], stmt[i + 1..].trim()),
        None => (stmt, ""),
    };
    let mnemonic = mnem.to_ascii_uppercase();
    let operand = if operand.is_empty() {
        None
    } else {
        Some(operand.to_string())
    };
    trace!(target: "assembler6502::parser", %mnemonic, has_op = operand.is_some(), "instr");
    Ok(vec![SourceNode::Instr {
        label,
        mnemonic,
        operand,
    }])
}
