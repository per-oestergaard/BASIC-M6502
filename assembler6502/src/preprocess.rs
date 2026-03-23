use regex::Regex;
use std::collections::HashMap;
use tracing::trace;

// Conditional state: track whether we're in the active branch
#[derive(Debug, Clone)]
struct Cond {
    active: bool,    // Is this branch being assembled?
    seen_else: bool, // Have we processed an ELSE for this IF?
}

// Preprocess original source for one target configuration (REALIO=4):
// - Strip TITLE/SEARCH/SALL/RADIX/SUBTTL
// - Handle simple equates NAME==value / NAME=value with ^O octal and $hex
// - Evaluate IFE sym / IFN sym (sym may appear in simple arithmetic sym-const or sym const difference) keeping only REALIO=4 path
// - Expand BLOCK n -> .block n (placeholder) and ADR(label) -> .word label, DCI"TEXT" -> bytes with high bit last char
// - Remove macro definitions (DEFINE ... < ... >) but keep calls as-is (later we can hardcode required ones)

pub struct PreprocessResult {
    pub lines: Vec<String>,
    pub symbols: HashMap<String, i64>,
}

pub fn run(src: &str) -> PreprocessResult {
    const OPCODES: &[&str] = &[
        "LDA", "LDX", "LDY", "STA", "STX", "STY", "ADC", "SBC", "INC", "INX", "INY", "DEX", "DEY",
        "DEC", "JMP", "JSR", "RTS", "RTI", "CLC", "SEC", "CLI", "SEI", "CLV", "CLD", "SED", "TAX",
        "TXA", "TAY", "TYA", "TSX", "TXS", "PHA", "PLA", "PHP", "PLP", "AND", "ORA", "EOR", "CMP",
        "CPX", "CPY", "BIT", "ASL", "LSR", "ROL", "ROR", "BPL", "BMI", "BVC", "BVS", "BCC", "BCS",
        "BNE", "BEQ", "BRK", "NOP",
    ];
    let directive_re = Regex::new(r"^(?i)(TITLE|SEARCH|SALL|RADIX|SUBTTL)\b").unwrap();
    let equ_re = Regex::new(r"^\s*([A-Za-z_.$][\w.$]*)\s*==?\s*([^;]+)").unwrap();
    let oct_re = Regex::new(r"\^O([0-7]+)").unwrap();
    let if_re = Regex::new(r"^(IFE|IFN|IFNDEF)\s+([^,]+),<(.*)$").unwrap();
    let if_block_re = Regex::new(r"^(IFE|IFN|IFNDEF)\s+([^,]+),<\s*$").unwrap();
    let adr_re =
        Regex::new(r"^\s*([A-Za-z_.$][\w.$]*:)?\s*ADR\s*\(([^)]+)\)\s*>?\s*(;.*)?$").unwrap();
    let dci_re =
        Regex::new(r#"^\s*([A-Za-z_.$][\w.$]*:)?\s*DCI\s*"([^"]+)"\s*>?\s*(;.*)?$"#).unwrap();
    let dt_re =
        Regex::new(r#"^\s*([A-Za-z_.$][\w.$]*:)?\s*DT\s*"([^"]+)"\s*>?\s*(;.*)?$"#).unwrap();
    let dce_re =
        Regex::new(r#"^\s*([A-Za-z_.$][\w.$]*:)?\s*DCE\s*"([^"]+)"\s*>?\s*(;.*)?$"#).unwrap();
    let dc_re =
        Regex::new(r#"^\s*([A-Za-z_.$][\w.$]*:)?\s*DC\s*"([^"]+)"\s*>?\s*(;.*)?$"#).unwrap();
    let block_re = Regex::new(r"^\s*([A-Za-z_.$][\w.$]*:)?\s*BLOCK\s+([^\s;]+)").unwrap();
    let org_re = Regex::new(r"^\s*ORG\s+(.+?)\s*(;.*)?$").unwrap();

    let define_re = Regex::new(r"^\s*DEFINE\b").unwrap();
    let printx_re = Regex::new(r"^\s*PRINTX\b").unwrap();
    let page_re = Regex::new(r"^\s*PAGE\b").unwrap();
    let repeat_re = Regex::new(r"^\s*([A-Za-z_.$][\w.$]*:)?\s*REPEAT\s+([^,]+),<(.*)$").unwrap();
    let comment_block_start_re = Regex::new(r"^\s*COMMENT\s+(.)").unwrap();
    let comment_single_re = Regex::new(r"^\s*COMMENT\s*$").unwrap();
    let symbol_only_re = Regex::new(r"^[A-Za-z_.$][\w.$]*$").unwrap();
    let date_line_re = Regex::new(r"^(\d{1,2})/(\d{1,2})/(\d{2}) ").unwrap();
    let label_hex_bytes_re =
        Regex::new(r"^([A-Za-z_.$][\w.$]*:)?\s*(\$[0-9A-Fa-f]{1,2})(\s+\$[0-9A-Fa-f]{1,2})*$")
            .unwrap();
    let label_single_num_re = Regex::new(r"^([A-Za-z_.$][\w.$]*:)\s*(\d+)$").unwrap();
    let label_single_sym_re =
        Regex::new(r"^([A-Za-z_.$][\w.$]*:)\s*([A-Za-z_.$][\w.$]*)$").unwrap();
    let mut out = Vec::new();
    let mut symbols: HashMap<String, i64> = HashMap::new();
    let mut i = 0;
    let lines: Vec<&str> = src.lines().collect();
    let mut cond_stack: Vec<Cond> = Vec::new();
    let mut _line_no = 0usize;
    let mut long_jmp_counter: usize = 0;
    let exp_re = Regex::new(r"^([A-Za-z_.$][\w.$]*:)?\s*EXP\s+(.+)$").unwrap();
    while i < lines.len() {
        let raw = lines[i];
        _line_no += 1;
        let mut line = raw.replace('\t', " ");
        line = strip_comment(&line);

        // CRITICAL: Skip lines inside inactive conditionals BEFORE processing closing > characters
        // This prevents us from incorrectly popping the conditional stack when processing
        // content that should be skipped (like REPEAT blocks inside inactive IFN/IFE)
        let should_skip_line = cond_stack.iter().any(|k| !k.active);

        // Process closing > characters to pop conditional stack
        // This must happen regardless of whether we're in an inactive block,
        // because that's how we know when the inactive block ends!
        if line.contains('>') {
            let trimmed = line.trim_end();
            if trimmed.ends_with('>') && trimmed != ">" {
                // For conditional directives, only look after the comma for closing >
                let check_str = if let Some(comma_pos) = trimmed.find(',') {
                    &trimmed[comma_pos + 1..]
                } else {
                    trimmed
                };

                // Count unmatched > characters (those without a matching < in the same expression)
                let open_count = check_str.matches('<').count();
                let close_count = check_str.matches('>').count();

                if close_count > open_count {
                    let num_closers = close_count - open_count;
                    // Pop stack once for each unmatched >
                    for _ in 0..num_closers {
                        if !cond_stack.is_empty() {
                            cond_stack.pop();
                        }
                    }
                    // Only strip closers from the line if we're NOT in an inactive block
                    // (inactive lines will be skipped anyway, so no need to modify them)
                    if !should_skip_line {
                        let mut end = trimmed.len();
                        for _ in 0..num_closers {
                            if end > 0 && trimmed.as_bytes()[end - 1] == b'>' {
                                end -= 1;
                            }
                        }
                        // Also strip a trailing comma left by the ",>" MACRO-10 block-end convention
                        while end > 0 && trimmed.as_bytes()[end - 1] == b',' {
                            end -= 1;
                        }
                        line = trimmed[..end].trim_end().to_string();
                    }
                }
            }
        }

        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        if let Some(pos) = line.find("::") {
            if line[..pos].chars().all(|c| c != ' ') {
                line = line.replacen("::", ":", 1);
            }
        }
        // Strip ! after : in labels (MACRO-10 local label notation)
        if let Some(pos) = line.find(":!") {
            if line[..pos].chars().all(|c| c != ' ') {
                line = line.replacen(":!", ":", 1);
            }
        }
        if directive_re.is_match(&line) {
            i += 1;
            continue;
        }
        if let Some(cap) = comment_block_start_re.captures(&line) {
            let delimiter = &cap[1];
            i += 1;
            while i < lines.len() {
                if lines[i].trim() == delimiter {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if comment_single_re.is_match(&line) {
            i += 1;
            continue;
        }
        // Skip lone delimiter lines (potential comment block terminators that slipped through)
        if line.trim().len() == 1 && !line.trim().chars().next().unwrap().is_alphanumeric() {
            i += 1;
            continue;
        }
        let trimmed_dash = line.trim();
        if !trimmed_dash.is_empty() && trimmed_dash.chars().all(|c| c == '-' || c == ' ') {
            i += 1;
            continue;
        }
        // Skip change-log date lines like 7/27/78 ...
        if date_line_re.is_match(line.trim_start()) {
            i += 1;
            continue;
        }
        // Skip parenthetical inline comment lines
        if line.trim_start().starts_with('(') && !line.contains(':') && !line.contains('=') {
            i += 1;
            continue;
        }
        // Skip plain text prose lines (no colon/equals and first word not opcode or known directive)
        let first_word = line.split_whitespace().next().unwrap_or("");
        if first_word.chars().all(|c| c.is_ascii_alphabetic()) && !OPCODES.contains(&first_word) {
            // Also skip the heuristic if the first word looks like a macro call we expand:
            // - ends with 'I' (immediate shorthand: LDAI, LDXI, LDYI, CMPI, ADCI, etc.)
            // - ends with "DY" or "DX" (indirect indexed: LDADY, STADY, LDADX, STADX)
            // - is a known multi-word macro name
            let is_known_macro = first_word.ends_with('I')
                || first_word.ends_with("DY")
                || first_word.ends_with("DX")
                || matches!(
                    first_word,
                    "STWD"
                        | "STWX"
                        | "STXY"
                        | "LDWD"
                        | "LDWX"
                        | "LDXY"
                        | "LDWDI"
                        | "LDWXI"
                        | "LDXYI"
                        | "PSHWD"
                        | "PULWD"
                        | "CLR"
                        | "COM"
                        | "SYNCHK"
                        | "JEQ"
                        | "JNE"
                        | "JCS"
                        | "JCC"
                        | "JMI"
                        | "JPL"
                        | "JVS"
                        | "JVC"
                        | "BCCA"
                        | "BCSA"
                        | "BEQA"
                        | "BNEA"
                        | "BMIA"
                        | "BPLA"
                        | "BVCA"
                        | "BVSA"
                        | "INCW"
                        | "SKIP1"
                        | "SKIP2"
                        | "ACRLF"
                );
            if !is_known_macro
                && !line.contains('=')
                && !line.contains(':')
                && !first_word.starts_with("ORG")
                && !first_word.starts_with("ADR")
                && !first_word.starts_with("DCI")
                && !first_word.starts_with("BLOCK")
                && !matches!(
                    first_word,
                    "IFE" | "IFN" | "IFNDEF" | "ELSE" | "ENDIF" | "REPEAT" | "DEFINE"
                )
            {
                i += 1;
                continue;
            }
        }
        // IF1 = pass-1 code (simulator addresses), IF2 = pass-2 (real 6502 addresses).
        // We target pass-2, so IF1 body is skipped.  Must push to cond_stack for
        // multi-line IF1/IF2 so that the body's closing > doesn't pop an outer block.
        // Note: these appear as "IF1,<" (comma attached, no space) so first_word is "IF1,<".
        let if12_directive = first_word.split(',').next().unwrap_or("");
        if matches!(if12_directive, "IF1" | "IF2") {
            if line.contains('<') {
                let after_comma = line.find(',').map(|p| &line[p + 1..]).unwrap_or("");
                let is_multiline =
                    after_comma.matches('<').count() > after_comma.matches('>').count();
                if is_multiline {
                    let include = if12_directive == "IF2";
                    let active = include && !should_skip_line;
                    cond_stack.push(Cond {
                        active,
                        seen_else: false,
                    });
                }
            }
            i += 1;
            continue;
        }
        // Normalize 'IF:' label (actual code label) Ensure trailing ':'
        if first_word == "IF:" {
            line = line.replacen("IF:", "IF:", 1);
        }
        if let Some(cap) = org_re.captures(&line) {
            let expr = cap[1].trim();
            let norm = normalize_numeric(expr, &oct_re);
            if let Some(val) = eval_expr(&norm, &symbols) {
                out.push(format!(".org ${:04X}", (val & 0xFFFF)));
            } else {
                // attempt direct symbol lookup
                if let Some(v) = symbols.get(expr) {
                    out.push(format!(".org ${:04X}", (v & 0xFFFF) as i64));
                } else {
                    out.push(format!("; unresolved org {}", expr));
                }
            }
            i += 1;
            continue;
        }
        // Handle label-prefixed conditionals: "OUTDO:  IFN REALIO,<"
        // Strip the label, emit it, and let the conditional be processed normally.
        // This must happen BEFORE if_re/if_block_re so they can match the keyword.
        {
            let t = line.trim();
            if let Some(colon_pos) = t.find(':') {
                let after_label = t[colon_pos + 1..].trim();
                let fw_cond = after_label
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_ascii_uppercase();
                if matches!(fw_cond.as_str(), "IFE" | "IFN" | "IFNDEF") {
                    if !should_skip_line {
                        out.push(format!("{}:", t[..colon_pos].trim()));
                    }
                    line = after_label.to_string();
                }
            }
        }
        if let Some(cap) = if_re.captures(&line) {
            let kind = &cap[1];
            let expr = cap[2].trim();
            let body = cap[3].trim();
            let val = eval_expr(expr, &symbols).unwrap_or(0);
            let keep = (val == 0 && kind.eq_ignore_ascii_case("IFE"))
                || (val != 0 && kind.eq_ignore_ascii_case("IFN"))
                || (kind.eq_ignore_ascii_case("IFNDEF") && !symbols.contains_key(expr));
            // Single-line conditional if body contains '>'
            if body.contains('>') {
                if keep && !should_skip_line {
                    let inner = body.split('>').next().unwrap().trim();
                    if !inner.is_empty() {
                        // process inner as a standalone line by re-running normalization path
                        let mut inner_line = inner.to_string();
                        if let Some(cap2) = equ_re.captures(&inner_line) {
                            let name = &cap2[1];
                            let expr2 = cap2[2].split(';').next().unwrap().trim();
                            if let Some(v) = eval_expr(expr2, &symbols) {
                                symbols.insert(name.to_string(), v);
                            }
                            out.push(format!("{} = {}", name, normalize_numeric(expr2, &oct_re)));
                        } else if let Some(cap2) = org_re.captures(&inner_line) {
                            let expr3 = cap2[1].trim();
                            let norm = normalize_numeric(expr3, &oct_re);
                            if let Some(val2) = eval_expr(&norm, &symbols) {
                                out.push(format!(".org ${:04X}", (val2 & 0xFFFF)));
                            }
                        } else if let Some(cap2) = block_re.captures(&inner_line) {
                            let label = cap2.get(1).map(|m| m.as_str()).unwrap_or("");
                            let size = cap2.get(2).unwrap().as_str();
                            if label.is_empty() {
                                out.push(format!(".res {}", size));
                            } else {
                                out.push(format!("{} .res {}", label, size));
                            }
                        } else if let Some(cap2) = dt_re.captures(&inner_line) {
                            let label = cap2.get(1).map(|m| m.as_str()).unwrap_or("");
                            let word = cap2.get(2).unwrap().as_str();
                            let bytes: Vec<String> = word
                                .chars()
                                .map(|ch| format!("${:02X}", (ch as u8) & 0x7F))
                                .collect();
                            let data = format!(".byte {}", bytes.join(", "));
                            if label.is_empty() {
                                out.push(data);
                            } else {
                                out.push(format!("{} {}", label, data));
                            }
                        } else if let Some(cap2) = dci_re.captures(&inner_line) {
                            let label = cap2.get(1).map(|m| m.as_str()).unwrap_or("");
                            let word = cap2.get(2).unwrap().as_str();
                            let mut bytes: Vec<String> = Vec::new();
                            for (j, ch) in word.chars().enumerate() {
                                let mut b = (ch as u8) & 0x7F;
                                if j == word.len() - 1 {
                                    b |= 0x80;
                                }
                                bytes.push(format!("${:02X}", b));
                            }
                            let data = format!(".byte {}", bytes.join(", "));
                            if label.is_empty() {
                                out.push(data);
                            } else {
                                out.push(format!("{} {}", label, data));
                            }
                        } else {
                            inner_line = normalize_numeric(&inner_line, &oct_re);
                            // Handle LABEL: number/symbol/hex/octal patterns
                            if label_single_num_re.is_match(inner_line.trim()) {
                                let cap2 = label_single_num_re.captures(inner_line.trim()).unwrap();
                                inner_line = format!("{} .byte {}", &cap2[1], &cap2[2]);
                            } else if label_single_sym_re.is_match(inner_line.trim()) {
                                let cap2 = label_single_sym_re.captures(inner_line.trim()).unwrap();
                                inner_line = format!("{} .byte {}", &cap2[1], &cap2[2]);
                            } else {
                                let t = inner_line.trim();
                                if let Some(colon_pos) = t.find(':') {
                                    let after_colon = t[colon_pos + 1..].trim();
                                    let is_hex = after_colon.starts_with('$')
                                        && after_colon.len() > 1
                                        && after_colon[1..].chars().all(|c| c.is_ascii_hexdigit());
                                    let is_octal = after_colon.starts_with("^O")
                                        && after_colon.len() > 2
                                        && after_colon[2..].chars().all(|c| c.is_digit(8));
                                    if is_hex || is_octal {
                                        let label = &t[..=colon_pos];
                                        inner_line = format!("{} .byte {}", label, after_colon);
                                    }
                                }
                            }
                            // Convert standalone numbers to .byte directives
                            let trimmed_inner = inner_line.trim_start();
                            if !trimmed_inner.is_empty()
                                && !trimmed_inner.starts_with('.')
                                && !trimmed_inner.contains(':')
                            {
                                let num_part =
                                    trimmed_inner.split_whitespace().next().unwrap_or("");
                                if !num_part.is_empty() {
                                    let is_num = num_part.chars().all(|c| c.is_ascii_digit())
                                        || (num_part.starts_with('$')
                                            && num_part[1..]
                                                .chars()
                                                .all(|c| c.is_ascii_hexdigit()))
                                        || (num_part.starts_with("^O")
                                            && num_part[2..].chars().all(|c| c.is_digit(8)));
                                    if is_num {
                                        inner_line = format!(".byte {}", inner_line.trim());
                                    }
                                }
                            }
                            // Handle pseudo-immediate instructions (LDAI, ORAI, etc.)
                            let first_word = inner_line.split_whitespace().next().unwrap_or("");
                            if first_word.len() >= 4
                                && first_word.ends_with('I')
                                && first_word.chars().all(|c| c.is_ascii_uppercase())
                            {
                                let base = &first_word[..first_word.len() - 1];
                                if matches!(
                                    base,
                                    "LDA"
                                        | "LDX"
                                        | "LDY"
                                        | "ADC"
                                        | "SBC"
                                        | "AND"
                                        | "ORA"
                                        | "EOR"
                                        | "CMP"
                                        | "CPX"
                                        | "CPY"
                                ) {
                                    // Find where first_word ends in the line
                                    if let Some(pos) = inner_line.find(first_word) {
                                        let rest = inner_line[pos + first_word.len()..].trim();
                                        inner_line = format!("{} #{}", base, rest);
                                    }
                                }
                            }
                            // Skip assembly-time directives that have no machine code
                            let fw_inner = inner_line
                                .split_whitespace()
                                .next()
                                .unwrap_or("")
                                .to_ascii_uppercase();
                            if matches!(
                                fw_inner.as_str(),
                                "PRINTX" | "PAGE" | "SUBTTL" | "TITLE" | "SALL" | "RADIX"
                            ) {
                                // Don't push to output
                            } else {
                                out.push(inner_line);
                            }
                        }
                    }
                }
                // no stack push for single-line
                i += 1;
                continue;
            } else {
                // Multi-line conditional: push to stack
                // If we're already in an inactive block, this new block is also inactive
                let active = if should_skip_line { false } else { keep };
                cond_stack.push(Cond {
                    active,
                    seen_else: false,
                });
                i += 1;
                continue;
            }
        }
        if let Some(cap) = if_block_re.captures(&line) {
            let kind = &cap[1];
            let expr = cap[2].trim();
            let val = eval_expr(expr, &symbols).unwrap_or(0);
            let keep = (val == 0 && kind.eq_ignore_ascii_case("IFE"))
                || (val != 0 && kind.eq_ignore_ascii_case("IFN"))
                || (kind.eq_ignore_ascii_case("IFNDEF") && !symbols.contains_key(expr));
            // If we're already in an inactive block, this new block is also inactive
            let active = if should_skip_line { false } else { keep };
            cond_stack.push(Cond {
                active,
                seen_else: false,
            });
            i += 1;
            continue;
        }
        // Handle ELSE directive
        if line.trim().eq_ignore_ascii_case("ELSE") && !cond_stack.is_empty() {
            if let Some(cond) = cond_stack.last_mut() {
                if !cond.seen_else {
                    cond.active = !cond.active;
                    cond.seen_else = true;
                }
            }
            i += 1;
            continue;
        }
        // Handle standalone ENDIF or ">"
        if line.trim().eq_ignore_ascii_case("ENDIF") || line.trim() == ">" {
            if !cond_stack.is_empty() {
                cond_stack.pop();
            }
            i += 1;
            continue;
        }

        // NOW skip this line if we're inside an inactive conditional
        // (after processing all conditional directives which must be tracked even in inactive blocks)
        if should_skip_line {
            i += 1;
            continue;
        }
        // Skip DEFINE bodies (active context only — inactive blocks are already handled above)
        if define_re.is_match(&line) {
            trace!(target: "assembler6502::preprocess", line=%line.trim(), "skipping DEFINE body");
            i += 1;
            while i < lines.len() {
                let l = lines[i].trim_end();
                if l.ends_with('>') {
                    break;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if let Some(cap) = equ_re.captures(&line) {
            let name = &cap[1];
            let expr = cap[2].split(';').next().unwrap().trim();
            if let Some(v) = eval_expr(expr, &symbols) {
                symbols.insert(name.to_string(), v);
                // Emit the numeric value so the assembler doesn't need to re-evaluate
                // complex expressions involving <lo-byte> or >hi-byte operators.
                out.push(format!("{} = {}", name, v));
                trace!(target: "assembler6502::preprocess", name=%name, value=%v, "equate evaluated");
            } else {
                out.push(format!("{} = {}", name, normalize_numeric(expr, &oct_re)));
            }
            i += 1;
            continue;
        }
        if let Some(cap) = adr_re.captures(&line) {
            let label = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let sym = cap.get(2).unwrap().as_str().trim();
            out.push(format!("{} .word {}", label, sym).trim().to_string());
            i += 1;
            continue;
        }
        if let Some(cap) = dci_re.captures(&line) {
            let label = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let word = cap.get(2).unwrap().as_str();
            let mut bytes: Vec<String> = Vec::new();
            for (j, ch) in word.chars().enumerate() {
                let mut b = (ch as u8) & 0x7F;
                if j == word.len() - 1 {
                    b |= 0x80;
                }
                bytes.push(format!("${:02X}", b));
            }
            let data = format!(".byte {}", bytes.join(", "));
            if label.is_empty() {
                out.push(data);
            } else {
                out.push(format!("{} {}", label, data));
            }
            i += 1;
            continue;
        }
        if let Some(cap) = dt_re.captures(&line) {
            let label = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let word = cap.get(2).unwrap().as_str();
            let bytes: Vec<String> = word
                .chars()
                .map(|ch| format!("${:02X}", (ch as u8) & 0x7F))
                .collect();
            let data = format!(".byte {}", bytes.join(", "));
            if label.is_empty() {
                out.push(data);
            } else {
                out.push(format!("{} {}", label, data));
            }
            i += 1;
            continue;
        }
        if let Some(cap) = dce_re.captures(&line) {
            let label = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let word = cap.get(2).unwrap().as_str();
            let bytes: Vec<String> = word
                .chars()
                .map(|ch| format!("${:02X}", (ch as u8) & 0x7F))
                .collect();
            let data = format!(".byte {}", bytes.join(", "));
            if label.is_empty() {
                out.push(data);
            } else {
                out.push(format!("{} {}", label, data));
            }
            i += 1;
            continue;
        }
        if let Some(cap) = dc_re.captures(&line) {
            let label = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let word = cap.get(2).unwrap().as_str();
            let bytes: Vec<String> = word
                .chars()
                .map(|ch| format!("${:02X}", (ch as u8) & 0x7F))
                .collect();
            let data = format!(".byte {}", bytes.join(", "));
            if label.is_empty() {
                out.push(data);
            } else {
                out.push(format!("{} {}", label, data));
            }
            i += 1;
            continue;
        }
        if let Some(cap) = block_re.captures(&line) {
            let label = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let size = cap.get(2).unwrap().as_str();
            if label.is_empty() {
                out.push(format!(".res {}", size));
            } else {
                out.push(format!("{} .res {}", label, size));
            }
            i += 1;
            continue;
        }
        // EXP pseudo-op: either defines bytes (if comma list or leading number) or a word reference
        if let Some(cap) = exp_re.captures(&line) {
            let label = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let arg = cap.get(2).unwrap().as_str().trim();
            let is_byte_list = arg.contains(',')
                || arg
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false);
            if is_byte_list {
                let mut bytes: Vec<String> = Vec::new();
                for part in arg.split(',') {
                    let p = part.trim();
                    if p.is_empty() {
                        continue;
                    }
                    if p.starts_with('$') {
                        bytes.push(p.to_string());
                    } else if let Ok(v) = p.parse::<i32>() {
                        bytes.push(format!("${:02X}", v & 0xFF));
                    } else {
                        bytes.push(p.to_string());
                    }
                }
                if !bytes.is_empty() {
                    if label.is_empty() {
                        out.push(format!(".byte {}", bytes.join(", ")));
                    } else {
                        out.push(format!("{} .byte {}", label, bytes.join(", ")));
                    }
                }
            } else {
                if label.is_empty() {
                    out.push(format!(".word {}", arg));
                } else {
                    out.push(format!("{} .word {}", label, arg));
                }
            }
            i += 1;
            continue;
        }
        if printx_re.is_match(&line) {
            i += 1;
            continue;
        }
        if page_re.is_match(&line) {
            i += 1;
            continue;
        }
        // Handle REPEAT n,<body> - repeat body n times
        if let Some(cap) = repeat_re.captures(&line) {
            let label_opt = cap.get(1).map(|m| m.as_str());
            let count_expr = cap[2].trim();
            // Evaluate the expression to get the count
            let count = if let Some(val) = eval_expr(count_expr, &symbols) {
                val.max(0) as usize
            } else {
                // Try parsing as plain number if eval fails
                count_expr.parse().unwrap_or(1)
            };
            let mut body = cap[3].trim();
            // Strip trailing > characters
            while body.ends_with('>') {
                body = &body[..body.len() - 1].trim_end();
            }
            for k in 0..count {
                let prefix = if k == 0 && label_opt.is_some() {
                    format!("{} ", label_opt.unwrap())
                } else {
                    " ".to_string()
                };
                let normalized = prefix + &normalize_numeric(body, &oct_re);
                out.push(normalized);
            }
            i += 1;
            continue;
        }
        if symbol_only_re.is_match(line.trim()) {
            line.push(':');
        }
        //  If line is a bare number (decimal or $hex or ^Ooctal) treat as .byte (common from stripped macros producing 0)
        {
            let t = line.trim();
            if !t.is_empty()
                && (t.chars().all(|c| c.is_ascii_digit())
                    || (t.starts_with('$')
                        && t.len() > 1
                        && t[1..].chars().all(|c| c.is_ascii_hexdigit()))
                    || (t.starts_with("^O")
                        && t.len() > 2
                        && t[2..].chars().all(|c| c.is_digit(8))))
            {
                line = format!(".byte {}", t);
            } else if !t.is_empty() && !t.contains(':') && !t.contains('=') {
                // Try to evaluate as expression (e.g., "333--ADDPRC")
                if let Some(val) = eval_expr(t, &symbols) {
                    // Output in octal format (^O prefix) to match RADIX 8 context
                    if val >= 0 && val <= 511 {
                        line = format!(".byte ^O{:o}", val);
                    }
                }
            }
        }
        // If line is sequence of hex byte tokens (e.g. $82 $0A $FF) convert to .byte list
        {
            let t = line.trim();
            if !t.is_empty() {
                let parts: Vec<&str> = t.split_whitespace().collect();
                if parts.len() > 1
                    && parts.iter().all(|p| {
                        p.starts_with('$')
                            && p.len() > 1
                            && p[1..].chars().all(|c| c.is_ascii_hexdigit())
                            && p.len() <= 3
                    })
                {
                    line = format!(".byte {}", parts.join(", "));
                }
            }
        }
        // If line is label + hex bytes or single hex with optional label, convert
        {
            if label_hex_bytes_re.is_match(line.trim()) {
                let mut segments = line.trim().split_whitespace();
                let first = segments.next().unwrap();
                let (label_opt, rest_iter) = if first.ends_with(':') {
                    (Some(first.to_string()), segments.collect::<Vec<_>>())
                } else {
                    (
                        None,
                        std::iter::once(first).chain(segments).collect::<Vec<_>>(),
                    )
                };
                let bytes: Vec<String> = rest_iter.into_iter().map(|b| b.to_string()).collect();
                if !bytes.is_empty() {
                    if let Some(lbl) = label_opt {
                        line = format!("{} .byte {}", lbl, bytes.join(", "));
                    } else {
                        line = format!(".byte {}", bytes.join(", "));
                    }
                }
            }
        }
        // If line is LABEL: single-number (like NULCNT: 0), convert to .byte
        {
            if label_single_num_re.is_match(line.trim()) {
                let cap = label_single_num_re.captures(line.trim()).unwrap();
                let label = &cap[1];
                let num = &cap[2];
                line = format!("{} .byte {}", label, num);
            }
        }
        // If line is LABEL: SYMBOL (like LINWID: LINLEN), convert to .byte
        {
            if label_single_sym_re.is_match(line.trim()) {
                let cap = label_single_sym_re.captures(line.trim()).unwrap();
                let label = &cap[1];
                let sym = &cap[2];
                line = format!("{} .byte {}", label, sym);
            }
        }
        // If line is LABEL: $HEX (like PIVAL: $82), convert to .byte
        {
            let t = line.trim();
            if let Some(colon_pos) = t.find(':') {
                let after_colon = t[colon_pos + 1..].trim();
                let is_hex = after_colon.starts_with('$')
                    && after_colon.len() > 1
                    && after_colon[1..].chars().all(|c| c.is_ascii_hexdigit());
                let is_octal = after_colon.starts_with("^O")
                    && after_colon.len() > 2
                    && after_colon[2..].chars().all(|c| c.is_digit(8));
                if is_hex || is_octal {
                    let label = &t[..=colon_pos];
                    line = format!("{} .byte {}", label, after_colon);
                }
            }
        }
        // If line is a quoted single character (like "T" or "("+128), convert to .byte
        {
            let t = line.trim();
            if t.starts_with('"') && t.len() >= 3 {
                // Match patterns like "X" or "X"+128 or "X"-1
                if let Some(close_quote) = t[1..].find('"') {
                    let char_part = &t[1..close_quote + 1];
                    if char_part.len() == 1 {
                        let ch = char_part.chars().next().unwrap();
                        let base_val = ch as u8;
                        let rest = t[close_quote + 2..].trim();
                        if rest.is_empty() {
                            line = format!(".byte ${:02X}", base_val);
                        } else if let Some(add) = rest.strip_prefix('+') {
                            if let Ok(offset) = add.trim().parse::<i32>() {
                                let val = (base_val as i32 + offset) & 0xFF;
                                line = format!(".byte ${:02X}", val);
                            }
                        } else if let Some(sub) = rest.strip_prefix('-') {
                            if let Ok(offset) = sub.trim().parse::<i32>() {
                                let val = (base_val as i32 - offset) & 0xFF;
                                line = format!(".byte ${:02X}", val);
                            }
                        }
                    }
                }
            }
        } // Simple multi-byte load/store macro expansion (subset): LDWX/LDWD/LDXY/STWD/STWX/STXY
          // We stripped original DEFINEs, so expand calls here.
        {
            let mut label_prefix = "";
            let mut rest = line.as_str();
            if let Some(colon_pos) = line.find(':') {
                // Only treat as label if everything before ':' (trimmed) is valid label chars.
                // This avoids mistaking the ':' inside quoted strings like `":"` as a label sep.
                let before = line[..colon_pos].trim();
                if !before.is_empty()
                    && before
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "_.$".contains(c))
                {
                    label_prefix = &line[..colon_pos + 1];
                    rest = line[colon_pos + 1..].trim();
                }
            }
            let mut parts = rest.split_whitespace();
            if let Some(op) = parts.next() {
                let arg = parts.next();
                if let Some(a) = arg {
                    let opu = op.to_ascii_uppercase();
                    if opu == "STY" && a.to_ascii_uppercase() == "BUF,X" {
                        let newline = format!("{} STY BUF", label_prefix);
                        let norm = normalize_numeric(&newline, &oct_re);
                        out.push(norm);
                        i += 1;
                        continue;
                    }
                    // Long conditional jump macros JEQ/JNE/JCS/JCC/JMI/JPL/JVS/JVC -> invert branch to skip + JMP target
                    if matches!(
                        opu.as_str(),
                        "JEQ" | "JNE" | "JCS" | "JCC" | "JMI" | "JPL" | "JVS" | "JVC"
                    ) {
                        let (invert_branch, target) = match opu.as_str() {
                            "JEQ" => ("BNE", a),
                            "JNE" => ("BEQ", a),
                            "JCS" => ("BCC", a),
                            "JCC" => ("BCS", a),
                            "JMI" => ("BPL", a),
                            "JPL" => ("BMI", a),
                            "JVS" => ("BVC", a),
                            "JVC" => ("BVS", a),
                            _ => ("", a),
                        };
                        let skip_label = format!("__LJ{}", long_jmp_counter);
                        long_jmp_counter += 1;
                        let first = format!("{} {} {}", label_prefix, invert_branch, skip_label);
                        let second = format!(" JMP {}", target);
                        let third = format!("{}:", skip_label);
                        for ln in [first, second, third] {
                            let norm = normalize_numeric(&ln, &oct_re);
                            out.push(norm);
                        }
                        i += 1;
                        continue;
                    }
                    // Indirect-indexed shorthand:
                    //   LDADY addr -> LDA (addr),Y  (indirect indexed, opcode $B1)
                    //   STADY addr -> STA (addr),Y  (indirect indexed, opcode $91)
                    //   LDADX addr -> LDA (addr,X)  (indexed indirect, opcode $A1)
                    //   STADX addr -> STA (addr,X)  (indexed indirect, opcode $81)
                    // NOTE: these are NOT the same as absolute indexed (LDA addr,Y / LDA addr,X).
                    if (opu.ends_with("DY") || opu.ends_with("DX")) && opu.len() > 2 {
                        let base = &opu[..opu.len() - 2];
                        let idx_suffix = &opu[opu.len() - 2..];
                        if matches!(
                            base,
                            "LDA" | "STA" | "CMP" | "ADC" | "SBC" | "AND" | "ORA" | "EOR"
                        ) {
                            let final_line = if idx_suffix == "DY" {
                                // (addr),Y  — indirect indexed (Y is OUTSIDE the parens)
                                format!("{} {} ({}),Y", label_prefix, base, a)
                            } else {
                                // (addr,X)  — indexed indirect (X is inside the parens)
                                format!("{} {} ({},X)", label_prefix, base, a)
                            };
                            trace!(target: "assembler6502::preprocess", src=%line, expanded=%final_line, "indirect-indexed expansion");
                            let norm = normalize_numeric(&final_line, &oct_re);
                            out.push(norm);
                            i += 1;
                            continue;
                        }
                    }
                    let expand = match opu.as_str() {
                        "SYNCHK" => Some(vec![
                            format!("{} LDA #{}", label_prefix, a),
                            "JSR SYNCHR".to_string(),
                        ]),
                        // Push 16-bit value (heuristic: low then high)
                        "PSHWD" => Some(vec![
                            format!("{} LDA {}", label_prefix, a),
                            "PHA".to_string(),
                            format!("LDA {}+1", a),
                            "PHA".to_string(),
                        ]),
                        // Pull 16-bit value (high then low, reverse of push)
                        "PULWD" => Some(vec![
                            format!("{} PLA", label_prefix),
                            format!("STA {}+1", a),
                            "PLA".to_string(),
                            format!("STA {}", a),
                        ]),
                        // CLR addr: emulate by loading A with 0 then storing
                        "CLR" => Some(vec![
                            format!("{} LDA #0", label_prefix),
                            format!("STA {}", a),
                        ]),
                        // COM addr: complement (XOR with $FF)
                        "COM" => Some(vec![
                            format!("{} LDA {}", label_prefix, a),
                            format!("EOR #$FF"),
                            format!("STA {}", a),
                        ]),
                        "LDWX" => Some(vec![
                            format!("{} LDA {}", label_prefix, a),
                            format!("LDX {}+1", a),
                        ]),
                        "LDWD" => Some(vec![
                            format!("{} LDA {}", label_prefix, a),
                            format!("LDY {}+1", a),
                        ]),
                        "LDXY" => Some(vec![
                            format!("{} LDX {}", label_prefix, a),
                            format!("LDY {}+1", a),
                        ]),
                        "STWD" => Some(vec![
                            format!("{} STA {}", label_prefix, a),
                            format!("STY {}+1", a),
                        ]),
                        "STWX" => Some(vec![
                            format!("{} STA {}", label_prefix, a),
                            format!("STX {}+1", a),
                        ]),
                        "STXY" => Some(vec![
                            format!("{} STX {}", label_prefix, a),
                            format!("STY {}+1", a),
                        ]),
                        // Branch-always aliases (DEFINE-defined in source)
                        "BCCA" => Some(vec![format!("{} BCC {}", label_prefix, a)]),
                        "BCSA" => Some(vec![format!("{} BCS {}", label_prefix, a)]),
                        "BEQA" => Some(vec![format!("{} BEQ {}", label_prefix, a)]),
                        "BNEA" => Some(vec![format!("{} BNE {}", label_prefix, a)]),
                        "BMIA" => Some(vec![format!("{} BMI {}", label_prefix, a)]),
                        "BPLA" => Some(vec![format!("{} BPL {}", label_prefix, a)]),
                        "BVCA" => Some(vec![format!("{} BVC {}", label_prefix, a)]),
                        "BVSA" => Some(vec![format!("{} BVS {}", label_prefix, a)]),
                        "LDWDI" => {
                            let inner = if a.starts_with('<') && a.ends_with('>') && a.len() > 2 {
                                &a[1..a.len() - 1]
                            } else {
                                a
                            };
                            Some(vec![
                                format!("{} LDA #<{}", label_prefix, inner),
                                format!("LDY #>{}", inner),
                            ])
                        }
                        "LDWXI" => {
                            let inner = if a.starts_with('<') && a.ends_with('>') && a.len() > 2 {
                                &a[1..a.len() - 1]
                            } else {
                                a
                            };
                            Some(vec![
                                format!("{} LDA #<{}", label_prefix, inner),
                                format!("LDX #>{}", inner),
                            ])
                        }
                        "LDXYI" => {
                            let inner = if a.starts_with('<') && a.ends_with('>') && a.len() > 2 {
                                &a[1..a.len() - 1]
                            } else {
                                a
                            };
                            Some(vec![
                                format!("{} LDX #<{}", label_prefix, inner),
                                format!("LDY #>{}", inner),
                            ])
                        }
                        _ => None,
                    };
                    if let Some(lines_expanded) = expand {
                        trace!(target: "assembler6502::preprocess", src=%line, n=%lines_expanded.len(), "macro expansion");
                        for ln in lines_expanded {
                            let normalized = normalize_numeric(&ln, &oct_re);
                            out.push(normalized);
                        }
                        i += 1;
                        continue; // move to next original source line
                    }
                    if opu.ends_with('I') && opu.len() >= 3 {
                        let base = &opu[..opu.len() - 1];
                        if matches!(
                            base,
                            "LDA"
                                | "LDX"
                                | "LDY"
                                | "ADC"
                                | "SBC"
                                | "AND"
                                | "ORA"
                                | "EOR"
                                | "CMP"
                                | "CPX"
                                | "CPY"
                        ) {
                            let operand = simplify_hi_lo_angles(a, &symbols);
                            let newline = format!("{} {} #{}", label_prefix, base, operand);
                            trace!(target: "assembler6502::preprocess", src=%line, expanded=%newline, "immediate shorthand");
                            let norm = normalize_numeric(&newline, &oct_re);
                            out.push(norm);
                            i += 1;
                            continue;
                        }
                    }
                }
            }
        }
        // Final guard: drop any stray IFN/IFE/IF1/IF2 lines not handled above
        let fw = line.split_whitespace().next().unwrap_or("");
        if ["IFN", "IFE", "IFNDEF", "IF1", "IF2"]
            .iter()
            .any(|w| fw.to_ascii_uppercase().starts_with(w))
        {
            i += 1;
            continue;
        }
        line = normalize_numeric(&line, &oct_re);
        // Convert standalone numbers to .byte directives
        let trimmed = line.trim_start();
        if !trimmed.is_empty()
            && !trimmed.starts_with('.')
            && !trimmed.contains(':')
            && (trimmed.chars().next().unwrap().is_ascii_digit()
                || trimmed.starts_with('$')
                || trimmed.starts_with("^O"))
        {
            // Check if it's just a number (possibly with whitespace/comment)
            let num_part = trimmed.split_whitespace().next().unwrap_or("");
            if !num_part.is_empty() {
                let is_num = num_part.chars().all(|c| c.is_ascii_digit())
                    || (num_part.starts_with('$')
                        && num_part[1..].chars().all(|c| c.is_ascii_hexdigit()))
                    || (num_part.starts_with("^O") && num_part[2..].chars().all(|c| c.is_digit(8)));
                if is_num {
                    let old = line.trim().to_string();
                    line = format!(".byte {}", old);
                    trace!(target: "assembler6502::preprocess", src=%old, expanded=%line, "bare number → .byte");
                }
            }
        }
        out.push(line);
        i += 1;
    }
    // Inject platform fallback symbols if absent (REALIO=4 => APPLE)
    let mut need_inject = Vec::new();
    for (name, val) in [
        ("APPLE", 1),
        ("COMMODORE", 0),
        ("OSI", 0),
        ("KIM", 0),
        ("STM", 0),
    ] {
        if !symbols.contains_key(name) {
            need_inject.push((name, val));
        }
    }
    if !need_inject.is_empty() {
        for (n, v) in &need_inject {
            out.insert(2, format!("{} = {}", n, v));
            symbols.insert((*n).to_string(), *v);
        }
    }
    PreprocessResult {
        lines: out,
        symbols,
    }
}

fn normalize_numeric(s: &str, oct_re: &Regex) -> String {
    oct_re
        .replace_all(s, |m: &regex::Captures| {
            format!("${:X}", i64::from_str_radix(&m[1], 8).unwrap_or(0))
        })
        .into_owned()
}

fn eval_expr(expr: &str, symbols: &HashMap<String, i64>) -> Option<i64> {
    let mut s = expr.trim().to_string();
    s = s.replace("==", "=");
    // Replace octal ^O
    let oct_re = Regex::new(r"\^O([0-7]+)").unwrap();
    s = oct_re
        .replace_all(&s, |m: &regex::Captures| {
            format!("{}", i64::from_str_radix(&m[1], 8).unwrap_or(0))
        })
        .into_owned();
    // hex $
    let hex_re = Regex::new(r"\$([0-9A-Fa-f]+)").unwrap();
    s = hex_re
        .replace_all(&s, |m: &regex::Captures| {
            format!("{}", i64::from_str_radix(&m[1], 16).unwrap_or(0))
        })
        .into_owned();
    // substitute symbols
    let sym_re = Regex::new(r"[A-Za-z_.$][A-Za-z0-9_.$]*").unwrap();
    s = sym_re
        .replace_all(&s, |m: &regex::Captures| {
            symbols
                .get(m.get(0).unwrap().as_str())
                .map(|v| v.to_string())
                .unwrap_or("0".to_string())
        })
        .into_owned();
    // Replace quoted single-char literals "X" with ASCII decimal value (e.g. "Z" → 90)
    let char_re = Regex::new(r#""(.)""#).unwrap();
    s = char_re
        .replace_all(&s, |m: &regex::Captures| {
            format!("{}", m[1].chars().next().unwrap_or('\0') as u32)
        })
        .into_owned();
    // Expand <...> bracketed groups using a depth-tracking stack walk.
    // <> in this source is purely arithmetic grouping (like parentheses) —
    // the correct tool is a state machine, not a regex.
    s = expand_angle_groups(&s);
    if !s
        .chars()
        .all(|c| c.is_ascii_digit() || "+-*/() ".contains(c))
    {
        return None;
    }
    match eval_simple(&s) {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

/// Simplify angle-bracket arithmetic in an instruction OPERAND before emitting.
/// Uses the state-machine depth counter to find balanced `<inner>` groups.
///
/// When the inner content can be fully evaluated (all symbols known), replace
/// with the number.  When it can't (code label not yet resolved), apply the
/// semantic mappings that the downstream assembler understands:
///
///   <<X>/256>   →  >X     (high byte of X)
///   <<X>/^O400> →  >X     (same, octal form)
///   <<X>&^O377> →  <X     (low byte of X)
///   <<X>&255>   →  <X
///
fn simplify_hi_lo_angles(s: &str, symbols: &HashMap<String, i64>) -> String {
    // Fast path: if the whole expression can be fully evaluated, return the number.
    if let Some(v) = eval_expr(s, symbols) {
        return v.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            let mut depth = 1usize;
            let mut j = i + 1;
            while j < chars.len() && depth > 0 {
                if chars[j] == '<' {
                    depth += 1;
                }
                if chars[j] == '>' {
                    depth -= 1;
                }
                j += 1;
            }
            let inner: String = chars[i + 1..j - 1].iter().collect();
            // First try: fully evaluate (works when all sub-symbols are constants).
            let expanded = expand_angle_groups(inner.trim());
            if let Ok(v) = eval_simple(&expanded) {
                out.push_str(&v.to_string());
                i = j;
                continue;
            }
            // Still has unknown symbols — apply semantic /256 → > and &255 → < mappings.
            // Both patterns take the form <<EXPR>/N> or <<EXPR>&N>.
            // Use a nested depth-walk to find the leading <EXPR> sub-group.
            if inner.starts_with('<') {
                // Find the balanced sub-group at the start of `inner`.
                let inner_chars: Vec<char> = inner.chars().collect();
                let mut d2 = 1usize;
                let mut k = 1;
                while k < inner_chars.len() && d2 > 0 {
                    if inner_chars[k] == '<' {
                        d2 += 1;
                    }
                    if inner_chars[k] == '>' {
                        d2 -= 1;
                    }
                    k += 1;
                }
                // inner_chars[1..k-1] = the sub-expression; inner_chars[k..] = operator + divisor
                let sub_expr: String = inner_chars[1..k - 1].iter().collect();
                let tail: String = inner_chars[k..]
                    .iter()
                    .collect::<String>()
                    .trim_start()
                    .to_string();
                let lo_byte = tail.starts_with("&^O377")
                    || tail.starts_with("&255")
                    || tail.starts_with("&$FF")
                    || tail.starts_with("&$ff");
                let hi_byte = tail.starts_with("/^O400")
                    || tail.starts_with("/256")
                    || tail.starts_with("/$100");
                if hi_byte {
                    out.push('>');
                    out.push_str(sub_expr.trim());
                    i = j;
                    continue;
                } else if lo_byte {
                    out.push('<');
                    out.push_str(sub_expr.trim());
                    i = j;
                    continue;
                }
            }
            // Unknown pattern — keep as-is; assembler will report an error if needed.
            out.push('<');
            out.push_str(&inner);
            out.push('>');
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// When a balanced `<inner>` pair is found, evaluate `inner` recursively
/// and replace the whole `<inner>` with the numeric result.
fn expand_angle_groups(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            // Find the matching > using a depth counter
            let mut depth = 1usize;
            let mut j = i + 1;
            while j < chars.len() && depth > 0 {
                if chars[j] == '<' {
                    depth += 1;
                }
                if chars[j] == '>' {
                    depth -= 1;
                }
                j += 1;
            }
            // chars[i+1..j-1] is the balanced inner content
            let inner: String = chars[i + 1..j - 1].iter().collect();
            // Recursively expand nested groups, then try to evaluate
            let expanded = expand_angle_groups(inner.trim());
            if let Ok(v) = eval_simple(&expanded) {
                out.push_str(&v.to_string());
            } else {
                // Can't evaluate — keep as-is (will cause None from caller)
                out.push('<');
                out.push_str(&inner);
                out.push('>');
            }
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn eval_simple(s: &str) -> Result<i64, ()> {
    // extremely small recursive descent for + - * /
    let tokens = tokenize(s);
    let (v, idx) = parse_expr(&tokens, 0)?;
    if idx != tokens.len() {
        return Err(());
    }
    Ok(v)
}

#[derive(Clone, Debug)]
enum Tok {
    Num(i64),
    Op(char),
    LPar,
    RPar,
}
fn tokenize(s: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut num = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
            continue;
        }
        if !num.is_empty() {
            if let Ok(v) = num.parse() {
                out.push(Tok::Num(v));
            }
            num.clear();
        }
        match ch {
            '+' => out.push(Tok::Op('+')),
            '-' => out.push(Tok::Op('-')),
            '*' => out.push(Tok::Op('*')),
            '/' => out.push(Tok::Op('/')),
            '(' => out.push(Tok::LPar),
            ')' => out.push(Tok::RPar),
            ' ' | '\t' => {}
            _ => {}
        }
    }
    if !num.is_empty() {
        if let Ok(v) = num.parse() {
            out.push(Tok::Num(v));
        }
    }
    out
}
fn parse_expr(t: &[Tok], i: usize) -> Result<(i64, usize), ()> {
    parse_add(t, i)
}
fn parse_add(t: &[Tok], i: usize) -> Result<(i64, usize), ()> {
    let (mut v, mut idx) = parse_mul(t, i)?;
    while idx < t.len() {
        if let Tok::Op(op) = t[idx] {
            if op == '+' || op == '-' {
                let (rhs, idx2) = parse_mul(t, idx + 1)?;
                if op == '+' {
                    v += rhs;
                } else {
                    v -= rhs;
                }
                idx = idx2;
                continue;
            }
        }
        break;
    }
    Ok((v, idx))
}
fn parse_mul(t: &[Tok], i: usize) -> Result<(i64, usize), ()> {
    let (mut v, mut idx) = parse_atom(t, i)?;
    while idx < t.len() {
        if let Tok::Op(op) = t[idx] {
            if op == '*' || op == '/' {
                let (rhs, idx2) = parse_atom(t, idx + 1)?;
                if op == '*' {
                    v *= rhs;
                } else {
                    if rhs == 0 {
                        return Err(());
                    }
                    v /= rhs;
                }
                idx = idx2;
                continue;
            }
        }
        break;
    }
    Ok((v, idx))
}
fn parse_atom(t: &[Tok], i: usize) -> Result<(i64, usize), ()> {
    if i >= t.len() {
        return Err(());
    }
    match &t[i] {
        Tok::Num(v) => Ok((*v, i + 1)),
        Tok::LPar => {
            let (v, j) = parse_expr(t, i + 1)?;
            if j >= t.len() {
                return Err(());
            }
            match t[j] {
                Tok::RPar => Ok((v, j + 1)),
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

fn strip_comment(s: &str) -> String {
    let mut in_quote = false;
    let mut out = String::new();
    for ch in s.chars() {
        if ch == '"' {
            in_quote = !in_quote;
            out.push(ch);
            continue;
        }
        if ch == ';' && !in_quote {
            break;
        }
        out.push(ch);
    }
    out.trim_end().to_string()
}
