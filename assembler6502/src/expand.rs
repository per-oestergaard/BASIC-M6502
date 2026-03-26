use std::collections::HashMap;
use tracing::trace;
use crate::symbols::SymTable;

#[derive(Debug, Clone)]
pub struct MacroDef {
    pub params: Vec<String>,
    /// Body with `§` as statement separator.
    pub body: String,
}

pub struct Expander {
    pub syms: SymTable,
    pub macros: HashMap<String, MacroDef>,
}

impl Expander {
    pub fn new() -> Self {
        Self { syms: SymTable::new(), macros: HashMap::new() }
    }

    /// Expand a slice of preprocessed (§-joined) lines.
    /// Returns the lines ready for two-pass assembly.
    pub fn expand(&mut self, lines: &[&str]) -> Vec<String> {
        let mut out = Vec::new();
        for &line in lines {
            self.process_line(line, &mut out);
        }
        out
    }

    fn process_line(&mut self, raw: &str, out: &mut Vec<String>) {
        let line = raw.trim_end();
        let t = line.trim();
        if t.is_empty() { return; }

        let kw = keyword_of(t);
        let kw_up = kw.to_ascii_uppercase();

        // Listing directives — drop silently
        match kw_up.as_str() {
            "TITLE" | "SUBTTL" | "PAGE" | "SALL" | "SEARCH"
            | "PRINTX" | "XLIST" | "LIST" | "PURGE" | "RADIX" => {
                trace!(target: "assembler6502::expand", "drop {kw_up}");
                return;
            }
            _ => {}
        }

        // DEFINE — record macro, never emit
        if kw_up == "DEFINE" {
            self.record_define(t);
            return;
        }
        // DCI macro definition (Q=Q+1 / DC body) — record like DEFINE
        // These appear inside IFE LNGERR blocks etc; handled as DCI in assembler.

        // If1/IF2
        if kw_up == "IF1" {
            if let Some(body) = after_comma_block(t) {
                self.expand_body(&body, out);
            }
            return;
        }
        if kw_up == "IF2" {
            return; // second-pass only — skip
        }

        // Conditionals
        if let Some(rest) = strip_keyword(t, "IFE") {
            self.eval_cond(rest, |v| v == 0, out);
            return;
        }
        if let Some(rest) = strip_keyword(t, "IFN") {
            self.eval_cond(rest, |v| v != 0, out);
            return;
        }
        if let Some(rest) = strip_keyword(t, "IFNDEF") {
            let (sym, tail) = split_at_comma(rest);
            if !self.syms.is_defined(sym.trim()) {
                if let Some(body) = braced_content(tail.trim()) {
                    self.expand_body(&body, out);
                }
            }
            return;
        }
        if let Some(rest) = strip_keyword(t, "IFDEF") {
            let (sym, tail) = split_at_comma(rest);
            if self.syms.is_defined(sym.trim()) {
                if let Some(body) = braced_content(tail.trim()) {
                    self.expand_body(&body, out);
                }
            }
            return;
        }
        if let Some(rest) = strip_keyword(t, "IFDIF") {
            // IFDIF <A><B>[,<body>] — skip, only inside DT macro bodies
            trace!(target: "assembler6502::expand", "skip IFDIF: {rest}");
            return;
        }
        if let Some(rest) = strip_keyword(t, "IRPC") {
            // IRPC sym,<body> — complex, emit as-is for now
            trace!(target: "assembler6502::expand", "IRPC (pass-through): {rest}");
            out.push(t.to_string());
            return;
        }

        // Equate assignment — update symbol table AND emit
        if try_parse_equate(t, &mut self.syms) {
            out.push(t.to_string());
            return;
        }

        // Built-in pseudo-op expansion (immediate shorthands)
        if let Some(expanded) = expand_builtin_pseudo(t) {
            for l in expanded { out.push(l); }
            return;
        }

        // DEFINE macro call expansion
        if let Some(expanded) = self.try_expand_macro(t) {
            for l in expanded { out.push(l); }
            return;
        }

        // Plain line — pass through
        out.push(t.to_string());
    }

    fn eval_cond<F: Fn(i64) -> bool>(&mut self, rest: &str, pred: F, out: &mut Vec<String>) {
        let (expr_str, tail) = split_at_comma(rest);
        match self.syms.eval(expr_str.trim()) {
            Ok(v) => {
                trace!(target: "assembler6502::expand", "cond eval({:?}) = {} → {}", expr_str.trim(), v, pred(v));
                if pred(v) {
                    if let Some(body) = braced_content(tail.trim()) {
                        self.expand_body(&body, out);
                    }
                }
            }
            Err(e) => {
                trace!(target: "assembler6502::expand", "cond eval error: {e}, line: {rest}");
                // Unknown symbol — skip (conservative)
            }
        }
    }

    fn expand_body(&mut self, body: &str, out: &mut Vec<String>) {
        for stmt in body.split('§') {
            let s = stmt.trim();
            if !s.is_empty() {
                self.process_line(s, out);
            }
        }
    }

    fn record_define(&mut self, line: &str) {
        let rest = line["DEFINE".len()..].trim_start();
        let (name, after) = split_sym(rest);
        if name.is_empty() { return; }
        let name_up = name.to_ascii_uppercase();
        let after = after.trim_start();

        // Optional parameter list: (P1, P2, ...)
        let (params, after) = if after.starts_with('(') {
            if let Some(end) = after.find(')') {
                let ps: Vec<String> = after[1..end].split(',')
                    .map(|s| s.trim().to_ascii_uppercase())
                    .collect();
                (ps, after[end + 1..].trim_start())
            } else {
                (vec![], after)
            }
        } else {
            (vec![], after)
        };

        let after = after.strip_prefix(',').map(|s| s.trim_start()).unwrap_or(after);
        let body = braced_content(after).unwrap_or_default();

        trace!(target: "assembler6502::expand", "DEFINE {name_up} params={params:?}");
        self.macros.insert(name_up, MacroDef { params, body });
    }

    fn try_expand_macro(&self, line: &str) -> Option<Vec<String>> {
        let (name, rest) = split_sym(line);
        if name.is_empty() { return None; }
        let def = self.macros.get(&name.to_ascii_uppercase())?;

        let arg = rest.trim();
        // Strip surrounding parens if present
        let arg = arg.strip_prefix('(').and_then(|s| s.strip_suffix(')'))
            .map(|s| s.trim())
            .unwrap_or(arg);

        let mut body = def.body.clone();
        // Single-parameter substitution: replace <PARAM> with arg
        if def.params.len() == 1 {
            let p = &def.params[0];
            body = body.replace(&format!("<{p}>"), arg);
            // Also handle <P>+1 pattern
            body = body.replace(&format!("<{p}>+1"), &format!("{arg}+1"));
        } else if def.params.is_empty() {
            // no-arg macro, body used as-is
        }

        let lines: Vec<String> = body.split('§')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if lines.is_empty() { return None; }
        Some(lines)
    }
}

// ── Built-in pseudo-ops ───────────────────────────────────────────────────────
//
// These are assembler built-ins NOT defined as DEFINE macros in the source.
// They provide shorthand immediate and indirect addressing forms.

fn expand_builtin_pseudo(line: &str) -> Option<Vec<String>> {
    let (kw, rest) = split_sym(line);
    let rest = rest.trim();
    match kw.to_ascii_uppercase().as_str() {
        // Immediate forms
        "LDAI" => Some(vec![format!("LDA #{rest}")]),
        "LDXI" => Some(vec![format!("LDX #{rest}")]),
        "LDYI" => Some(vec![format!("LDY #{rest}")]),
        "CMPI" => Some(vec![format!("CMP #{rest}")]),
        "CPXI" => Some(vec![format!("CPX #{rest}")]),
        "CPYI" => Some(vec![format!("CPY #{rest}")]),
        "ADCI" => Some(vec![format!("ADC #{rest}")]),
        "SBCI" => Some(vec![format!("SBC #{rest}")]),
        "ANDI" => Some(vec![format!("AND #{rest}")]),
        "ORAI" => Some(vec![format!("ORA #{rest}")]),
        "EORI" => Some(vec![format!("EOR #{rest}")]),
        // ROR (real instruction in our Apple config — pass through)
        // JEQ/JNE etc. expanded here as well (per AGENTS.md)
        "JEQ" => Some(vec![format!("BNE .+5"), format!("JMP {rest}")]),
        "JNE" => Some(vec![format!("BEQ .+5"), format!("JMP {rest}")]),
        "JCS" => Some(vec![format!("BCC .+5"), format!("JMP {rest}")]),
        "JCC" => Some(vec![format!("BCS .+5"), format!("JMP {rest}")]),
        "JMI" => Some(vec![format!("BPL .+5"), format!("JMP {rest}")]),
        "JPL" => Some(vec![format!("BMI .+5"), format!("JMP {rest}")]),
        "JVS" => Some(vec![format!("BVC .+5"), format!("JMP {rest}")]),
        "JVC" => Some(vec![format!("BVS .+5"), format!("JMP {rest}")]),
        _ => None,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the first whitespace-delimited word of `s`.
fn keyword_of(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

/// Strip `keyword` (case-insensitive) from the start of `s`, returning the rest.
/// Returns `None` if `s` doesn't start with that keyword.
fn strip_keyword<'a>(s: &'a str, keyword: &str) -> Option<&'a str> {
    let kw_len = keyword.len();
    if s.len() < kw_len { return None; }
    if !s[..kw_len].eq_ignore_ascii_case(keyword) { return None; }
    let rest = &s[kw_len..];
    // Must be followed by whitespace or comma or end
    if rest.is_empty() || rest.starts_with([' ', '\t', ',']) {
        Some(rest.trim_start_matches([' ', '\t']))
    } else {
        None
    }
}

/// Return the body from `KEYWORD,<body>` — just the part after `,`.
fn after_comma_block(s: &str) -> Option<String> {
    let comma = s.find(',')?;
    braced_content(s[comma + 1..].trim())
}

/// Split at the first top-level comma (not inside `<>`).
fn split_at_comma(s: &str) -> (&str, &str) {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => return (&s[..i], &s[i + 1..]),
            _ => {}
        }
    }
    (s, "")
}

/// Extract content from `<content>` (outermost braces removed).
pub fn braced_content(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with('<') { return None; }
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut started = false;
    for (i, c) in s.char_indices() {
        match c {
            '<' => {
                if !started { start = i + 1; started = true; }
                depth += 1;
            }
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start..i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Split a leading symbol token from `s`.
fn split_sym(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    if s.is_empty() { return ("", s); }
    let c0 = s.chars().next().unwrap();
    if !c0.is_ascii_alphabetic() && c0 != '_' && c0 != '.' && c0 != '$' && c0 != '%' {
        return ("", s);
    }
    let end = s.find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '.' && c != '$')
        .unwrap_or(s.len());
    (&s[..end], &s[end..])
}

/// Parse equate of the form `NAME == expr` or `NAME = expr`.
/// Updates `syms` if the expression is evaluable.
/// Returns true if this line looks like an equate (whether or not eval succeeded).
fn try_parse_equate(line: &str, syms: &mut SymTable) -> bool {
    let (name, rest) = split_sym(line);
    if name.is_empty() { return false; }
    let rest = rest.trim_start();

    let (_strong, val_str) = if let Some(s) = rest.strip_prefix("==") {
        (true, s.trim())
    } else if let Some(s) = rest.strip_prefix('=') {
        if s.starts_with('=') { return false; } // not ===
        (false, s.trim())
    } else {
        return false;
    };

    if let Ok(v) = syms.eval(val_str) {
        syms.set(name, v);
    }
    true
}
