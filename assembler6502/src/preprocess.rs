use regex::Regex;
use std::collections::HashMap;

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
    let equ_re = Regex::new(r"^([A-Za-z_.$][\w.$]*)\s*==?\s*([^;]+)").unwrap();
    let oct_re = Regex::new(r"\^O([0-7]+)").unwrap();
    let if_re = Regex::new(r"^(IFE|IFN|IFNDEF)\s+([^,]+),<(.*)$").unwrap();
    let if_block_re = Regex::new(r"^(IFE|IFN|IFNDEF)\s+([^,]+),<\s*$").unwrap();
    let adr_re = Regex::new(r"^([A-Za-z_.$][\w.$]*:)?\s*ADR\(([^)]+)\)\s*>?\s*(;.*)?$").unwrap();
    let dci_re = Regex::new(r#"^([A-Za-z_.$][\w.$]*:)?\s*DCI"([^"]+)"\s*>?\s*(;.*)?$"#).unwrap();
    let dt_re = Regex::new(r#"^([A-Za-z_.$][\w.$]*:)?\s*DT"([^"]+)"\s*>?\s*(;.*)?$"#).unwrap();
    let block_re = Regex::new(r"^([A-Za-z_.$][\w.$]*:)?\s*BLOCK\s+([^\s;]+)").unwrap();
    let org_re = Regex::new(r"^\s*ORG\s+(.+?)\s*(;.*)?$").unwrap();
    let pseudo_imm_re = Regex::new(r"^\s*([A-Z]{2,4})I\s+(.+)$").unwrap();
    let angle_expr_re = Regex::new(r"<<[^>]+>").unwrap();
    let div_shift_re = Regex::new(r"0/\$100>>").unwrap();
    let angle_sym_re = Regex::new(r"<([A-Za-z_.$][\\w.$]*)>").unwrap();
    let define_re = Regex::new(r"^\s*DEFINE\b").unwrap();
    let printx_re = Regex::new(r"^\s*PRINTX\b").unwrap();
    let page_re = Regex::new(r"^\s*PAGE\b").unwrap();
    let comment_block_start_re = Regex::new(r"^\s*COMMENT\s+\*\b").unwrap();
    let comment_single_re = Regex::new(r"^\s*COMMENT\b").unwrap();
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
    let mut cond_stack: Vec<bool> = Vec::new();
    let code_start_re = Regex::new(r"^IFN\s+REALIO-3").unwrap();
    let mut code_started = false;
    let mut line_no = 0usize;
    let mut long_jmp_counter: usize = 0;
    let exp_re = Regex::new(r"^([A-Za-z_.$][\w.$]*:)?\s*EXP\s+(.+)$").unwrap();
    while i < lines.len() {
        let raw = lines[i];
        line_no += 1;
        let mut line = raw.replace('\t', " ");
        line = strip_comment(&line);
        // Strip trailing > (PDP-10 block terminator when not standalone line)
        if line.ends_with('>') && line.trim() != ">" {
            line = line[..line.len() - 1].trim_end().to_string();
        }
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        if !code_started {
            if code_start_re.is_match(line.trim_start()) || line_no >= 731 {
                code_started = true;
            } else {
                i += 1;
                continue;
            }
        }
        if let Some(pos) = line.find("::") {
            if line[..pos].chars().all(|c| c != ' ') {
                line = line.replacen("::", ":", 1);
            }
        }
        if directive_re.is_match(&line) {
            i += 1;
            continue;
        }
        if define_re.is_match(&line) {
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
        if comment_block_start_re.is_match(&line) {
            i += 1;
            while i < lines.len() {
                if lines[i].trim() == "*" {
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
        if line.trim() == "*" {
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
            if !line.contains('=')
                && !line.contains(':')
                && !first_word.starts_with("ORG")
                && !first_word.starts_with("ADR")
                && !first_word.starts_with("DCI")
                && !first_word.starts_with("BLOCK")
            {
                i += 1;
                continue;
            }
        }
        // Drop IF1, IF2 pseudo conditionals
        if matches!(first_word, "IF1" | "IF2") {
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
                if keep {
                    let inner = body.split('>').next().unwrap().trim();
                    if !inner.is_empty() {
                        // process inner as a standalone line by re-running normalization path
                        let mut inner_line = inner.to_string();
                        if let Some(cap2) = equ_re.captures(&inner_line) {
                            let name = &cap2[1];
                            let expr2 = cap2[2].split(';').next().unwrap().trim();
                            if !symbols.contains_key(name) {
                                if let Some(v) = eval_expr(expr2, &symbols) {
                                    symbols.insert(name.to_string(), v);
                                }
                            }
                            out.push(format!("{} = {}", name, normalize_numeric(expr2, &oct_re)));
                        } else if let Some(cap2) = org_re.captures(&inner_line) {
                            let expr3 = cap2[1].trim();
                            let norm = normalize_numeric(expr3, &oct_re);
                            if let Some(val2) = eval_expr(&norm, &symbols) {
                                out.push(format!(".org ${:04X}", (val2 & 0xFFFF)));
                            }
                        } else {
                            inner_line = normalize_numeric(&inner_line, &oct_re);
                            out.push(inner_line);
                        }
                    }
                }
                // no stack push for single-line
                i += 1;
                continue;
            } else {
                cond_stack.push(keep);
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
            cond_stack.push(keep);
            i += 1;
            continue;
        }
        if line.trim() == ">" && !cond_stack.is_empty() {
            cond_stack.pop();
            i += 1;
            continue;
        }
        if cond_stack.iter().any(|k| !*k) {
            i += 1;
            continue;
        }
        if let Some(cap) = equ_re.captures(&line) {
            let name = &cap[1];
            let expr = cap[2].split(';').next().unwrap().trim();
            if !symbols.contains_key(name) {
                if let Some(v) = eval_expr(expr, &symbols) {
                    symbols.insert(name.to_string(), v);
                }
            }
            out.push(format!("{} = {}", name, normalize_numeric(expr, &oct_re)));
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
        if let Some(cap) = pseudo_imm_re.captures(&line) {
            let base = &cap[1];
            let rest = cap[2].trim();
            // Only expand known immediate-capable mnemonics
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
                line = format!("{} #{}", base, rest);
            }
        }
        if printx_re.is_match(&line) {
            i += 1;
            continue;
        }
        if page_re.is_match(&line) {
            i += 1;
            continue;
        }
        // Simplify leftover macro shift/mask expressions like <<WD>&^O377> -> 0 (placeholder until evaluator improved)
        line = angle_expr_re.replace_all(&line, "0").into_owned();
        line = div_shift_re.replace_all(&line, "0").into_owned();
        // Replace simple <SYMBOL> occurrences
        line = angle_sym_re.replace_all(&line, "$1").into_owned();
        // Remove excess trailing '>' if unmatched
        if line.matches('<').count() < line.matches('>').count() {
            while line.ends_with('>') && line.matches('<').count() < line.matches('>').count() {
                line.pop();
            }
        }
        // Collapse accidental marker pairs like ><SYMBOL -> <SYMBOL
        if line.contains("><") {
            line = line.replace("><", "<");
        }
        if symbol_only_re.is_match(line.trim()) {
            line.push(':');
        }
        // If line is a bare number (decimal or $hex) treat as .byte (common from stripped macros producing 0)
        {
            let t = line.trim();
            if !t.is_empty()
                && (t.chars().all(|c| c.is_ascii_digit())
                    || (t.starts_with('$')
                        && t.len() > 1
                        && t[1..].chars().all(|c| c.is_ascii_hexdigit())))
            {
                line = format!(".byte {}", t);
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
        // Simple multi-byte load/store macro expansion (subset): LDWX/LDWD/LDXY/STWD/STWX/STXY
        // We stripped original DEFINEs, so expand calls here.
        {
            let mut label_prefix = "";
            let mut rest = line.as_str();
            if let Some(colon_pos) = line.find(':') {
                label_prefix = &line[..colon_pos + 1];
                rest = line[colon_pos + 1..].trim();
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
                    // Single-token indexed shorthand: e.g. LDADY LABEL -> LDA LABEL,Y ; CMPDY FOO -> CMP FOO,Y
                    if (opu.ends_with("DY") || opu.ends_with("DX")) && opu.len() > 2 {
                        let base = &opu[..opu.len() - 2];
                        let idx_suffix = &opu[opu.len() - 2..];
                        if matches!(
                            base,
                            "LDA" | "STA" | "CMP" | "ADC" | "SBC" | "AND" | "ORA" | "EOR" | "BIT"
                        ) {
                            let final_line = if idx_suffix == "DY" {
                                format!("{} {} {},Y", label_prefix, base, a)
                            } else {
                                format!("{} {} {},X", label_prefix, base, a)
                            };
                            let norm = normalize_numeric(&final_line, &oct_re);
                            out.push(norm);
                            i += 1;
                            continue;
                        }
                    }
                    let expand = match opu.as_str() {
                        "SYNCHK" => Some(vec![format!("{} JSR SYNCHK", label_prefix)]),
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
                        for (idx, ln) in lines_expanded.into_iter().enumerate() {
                            let normalized = normalize_numeric(&ln, &oct_re);
                            if idx == 0 {
                                out.push(normalized);
                            } else {
                                out.push(normalized);
                            }
                        }
                        i += 1;
                        continue; // move to next original source line
                    }
                    // Immediate shorthand with trailing 'I' (e.g., LDAI 10) including label-prefixed forms
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
                            let newline = format!("{} {} #{}", label_prefix, base, a);
                            let norm = normalize_numeric(&newline, &oct_re);
                            out.push(norm);
                            i += 1;
                            continue;
                        }
                    }
                }
            }
        }
        // Final guard: drop any stray IFN/IFE lines not handled above
        let fw = line.split_whitespace().next().unwrap_or("");
        if ["IFN", "IFE", "IFNDEF"]
            .iter()
            .any(|w| fw.eq_ignore_ascii_case(w))
        {
            i += 1;
            continue;
        }
        line = normalize_numeric(&line, &oct_re);
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
    if !s
        .chars()
        .all(|c| c.is_ascii_digit() || "+-*/() ".contains(c))
    {
        return None;
    }
    // simple eval: split by operators; use meval? implement minimal using eval crate? We'll implement basic left to right with + - only
    // For now just use rust eval via meval-like minimal: safely filter
    match eval_simple(&s) {
        Ok(v) => Some(v),
        Err(_) => None,
    }
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
