/// Stage 2 expander — walks Vec<SourceNode> and produces Vec<FlatStmt>.
///
/// Responsibilities:
///   - Maintain a symbol table (equates).
///   - Evaluate IFE/IFN/IFNDEF conditionals and recurse into the chosen branch.
///   - Store DEFINE bodies and expand macro calls (with argument substitution).
///   - Unroll REPEAT blocks.
///   - Expand all pseudo-ops from the AGENTS.md table into canonical 6502
///     instructions or data directives.
///   - Expand JEQ/JNE/etc. into inverted-short-branch-over-JMP form.
///   - Never error on out-of-range branches — the assembler rewrites them.
use crate::ast::{CondKind, FlatStmt, SourceNode};
use crate::parser::parse;
use crate::symbols::canonicalize_symbol_name;
use anyhow::Result;
use std::collections::HashMap;
use tracing::trace;

// ---------------------------------------------------------------------------
// MacroDef stored in the macro table
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct MacroDef {
    params: Vec<String>,
    body: Vec<String>,
}

fn strip_grouping_angles(expr: &str) -> &str {
    let expr = expr.trim();
    if !expr.starts_with('<') || !expr.ends_with('>') {
        return expr;
    }

    let mut depth = 0usize;
    for (index, ch) in expr.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    if index == expr.len() - 1 {
                        return &expr[1..index];
                    }
                    return expr;
                }
            }
            _ => {}
        }
    }

    expr
}

// ---------------------------------------------------------------------------
// Expander state
// ---------------------------------------------------------------------------

pub struct Expander {
    /// Symbol table: name → integer value (equates only).
    symbols: HashMap<String, i64>,
    /// Macro definitions: name → (params, body_lines).
    macros: HashMap<String, MacroDef>,
    /// Counter for generating unique long-jump labels.
    lj_counter: usize,
    /// Current default numeric base from RADIX directives.
    radix: u32,
}

impl Expander {
    pub fn new() -> Self {
        let mut syms: HashMap<String, i64> = HashMap::new();
        // Seed with Apple II config constants (the source tests ADDPRC, REALIO, etc.)
        syms.insert(canonicalize_symbol_name("ADDPRC"), 1);
        syms.insert(canonicalize_symbol_name("REALIO"), 4);
        syms.insert(canonicalize_symbol_name("APPLE"), 1);
        syms.insert(canonicalize_symbol_name("KIMROM"), 0);
        syms.insert(canonicalize_symbol_name("MSFT"), 0);
        Self {
            symbols: syms,
            macros: HashMap::new(),
            lj_counter: 0,
            radix: 10,
        }
    }

    pub fn expand(&mut self, nodes: Vec<SourceNode>) -> Result<Vec<FlatStmt>> {
        let mut out = Vec::new();
        for node in nodes {
            self.expand_node(node, &mut out)?;
        }
        Ok(out)
    }

    fn expand_node(&mut self, node: SourceNode, out: &mut Vec<FlatStmt>) -> Result<()> {
        match node {
            // ------------------------------------------------------------------
            SourceNode::Label { name } => {
                out.push(FlatStmt::Label(name));
            }

            // ------------------------------------------------------------------
            SourceNode::Equate { name, expr } => {
                if let Some(val) = try_eval_expr(expr.trim(), &self.symbols, self.radix) {
                    trace!(target: "assembler6502::expander", sym = %name, val, "equate");
                    self.symbols.insert(canonicalize_symbol_name(&name), val);
                } else {
                    trace!(target: "assembler6502::expander", sym = %name, expr = %expr, "deferred equate");
                }
                out.push(FlatStmt::Equate { name, expr });
            }

            // ------------------------------------------------------------------
            SourceNode::Org { expr } => {
                let addr = self.eval_expr_str(&expr) as u16;
                out.push(FlatStmt::Org(addr));
            }

            // ------------------------------------------------------------------
            SourceNode::Radix { base } => {
                self.radix = base;
                out.push(FlatStmt::Radix(base));
            }

            // ------------------------------------------------------------------
            SourceNode::MacroDef { name, params, body } => {
                trace!(target: "assembler6502::expander", macro_name = %name, "storing macro def");
                self.macros
                    .insert(name.to_ascii_uppercase(), MacroDef { params, body });
            }

            // ------------------------------------------------------------------
            SourceNode::Conditional {
                kind,
                expr,
                then_body,
                else_body,
            } => {
                let take_then = match kind {
                    CondKind::IfEq => self.eval_expr_str(&expr) == 0,
                    CondKind::IfNe => self.eval_expr_str(&expr) != 0,
                    CondKind::IfNotDef => !self
                        .symbols
                        .contains_key(&canonicalize_symbol_name(expr.trim())),
                    CondKind::If1 => true,
                    CondKind::If2 => false,
                };
                trace!(
                    target: "assembler6502::expander",
                    ?kind,
                    cond = %expr,
                    take_then,
                    "conditional"
                );
                let branch = if take_then { then_body } else { else_body };
                for n in branch {
                    self.expand_node(n, out)?;
                }
            }

            // ------------------------------------------------------------------
            SourceNode::Repeat {
                label,
                count_expr,
                body,
            } => {
                if let Some(lbl) = label {
                    out.push(FlatStmt::Label(lbl));
                }
                let count = self.eval_expr_str(&count_expr).max(0) as usize;
                for _ in 0..count {
                    for n in body.clone() {
                        self.expand_node(n, out)?;
                    }
                }
            }

            // ------------------------------------------------------------------
            SourceNode::MacroCall { label, name, arg } => {
                let upper = name.to_ascii_uppercase();
                if upper == "DCI" {
                    let current_q = self
                        .symbols
                        .get(&canonicalize_symbol_name("Q"))
                        .copied()
                        .unwrap_or(0);
                    let next_q = current_q + 1;
                    self.symbols.insert(canonicalize_symbol_name("Q"), next_q);
                    out.push(FlatStmt::Equate {
                        name: "Q".to_string(),
                        expr: "Q+1".to_string(),
                    });

                    let text = arg.unwrap_or_default().trim().to_string();
                    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
                        let content = &text[1..text.len() - 1];
                        let mut values = Vec::new();
                        for (index, byte) in content.bytes().enumerate() {
                            let encoded = if index + 1 == content.len() {
                                byte | 0x80
                            } else {
                                byte
                            };
                            values.push(format!("${encoded:02X}"));
                        }
                        out.push(FlatStmt::Bytes { label, values });
                    } else {
                        out.push(FlatStmt::Bytes {
                            label,
                            values: vec![text],
                        });
                    }
                    return Ok(());
                }

                if let Some(ref lbl) = label {
                    out.push(FlatStmt::Label(lbl.clone()));
                }
                if let Some(def) = self.macros.get(&upper).cloned() {
                    let arg_str = arg.unwrap_or_default();
                    let expanded_lines = substitute_macro_body(&def.params, &def.body, &arg_str);
                    let sub_nodes = parse(&expanded_lines.join("\n"))?;
                    for n in sub_nodes {
                        self.expand_node(n, out)?;
                    }
                } else {
                    self.expand_instr(label, &name, arg.as_deref(), out)?;
                }
            }

            // ------------------------------------------------------------------
            SourceNode::Bytes { label, args } => {
                out.push(FlatStmt::Bytes {
                    label,
                    values: args,
                });
            }

            SourceNode::Word { label, args } => {
                for (i, a) in args.into_iter().enumerate() {
                    let lbl = if i == 0 { label.clone() } else { None };
                    out.push(FlatStmt::Word {
                        label: lbl,
                        expr: a,
                    });
                }
            }

            SourceNode::Res { label, count_expr } => {
                let count = self.eval_expr_str(&count_expr).max(0) as u16;
                out.push(FlatStmt::Res { label, count });
            }

            // ------------------------------------------------------------------
            SourceNode::Instr {
                label,
                mnemonic,
                operand,
            } => {
                self.expand_instr(label, &mnemonic, operand.as_deref(), out)?;
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Instruction / pseudo-op expansion
    // -----------------------------------------------------------------------

    fn expand_instr(
        &mut self,
        label: Option<String>,
        mnemonic: &str,
        operand: Option<&str>,
        out: &mut Vec<FlatStmt>,
    ) -> Result<()> {
        let op = operand.unwrap_or("").trim();

        // Emit label first if present, before expanding multi-instruction ops
        macro_rules! emit_label {
            () => {
                if let Some(ref lbl) = label {
                    out.push(FlatStmt::Label(lbl.clone()));
                }
            };
        }

        macro_rules! instr {
            ($m:expr) => {
                FlatStmt::Instr {
                    label: None,
                    mnemonic: $m.to_string(),
                    operand: None,
                }
            };
            ($m:expr, $o:expr) => {
                FlatStmt::Instr {
                    label: None,
                    mnemonic: $m.to_string(),
                    operand: Some($o.to_string()),
                }
            };
        }

        match mnemonic {
            // ---- Immediate loads ----
            "LDAI" => {
                emit_label!();
                out.push(instr!("LDA", format!("#{}", op)));
            }
            "LDXI" => {
                emit_label!();
                out.push(instr!("LDX", format!("#{}", op)));
            }
            "LDYI" => {
                emit_label!();
                out.push(instr!("LDY", format!("#{}", op)));
            }
            "CMPI" => {
                emit_label!();
                out.push(instr!("CMP", format!("#{}", op)));
            }
            "CPXI" => {
                emit_label!();
                out.push(instr!("CPX", format!("#{}", op)));
            }
            "CPYI" => {
                emit_label!();
                out.push(instr!("CPY", format!("#{}", op)));
            }
            "ADCI" => {
                emit_label!();
                out.push(instr!("ADC", format!("#{}", op)));
            }
            "SBCI" => {
                emit_label!();
                out.push(instr!("SBC", format!("#{}", op)));
            }
            "ANDI" => {
                emit_label!();
                out.push(instr!("AND", format!("#{}", op)));
            }
            "ORAI" => {
                emit_label!();
                out.push(instr!("ORA", format!("#{}", op)));
            }
            "EORI" => {
                emit_label!();
                out.push(instr!("EOR", format!("#{}", op)));
            }

            // ---- Indirect indexed ----
            "LDADY" => {
                emit_label!();
                out.push(instr!("LDA", format!("({op}),Y")));
            }
            "STADY" => {
                emit_label!();
                out.push(instr!("STA", format!("({op}),Y")));
            }
            "LDADX" => {
                emit_label!();
                out.push(instr!("LDA", format!("({op},X)")));
            }
            "STADX" => {
                emit_label!();
                out.push(instr!("STA", format!("({op},X)")));
            }
            "CMPDY" => {
                emit_label!();
                out.push(instr!("CMP", format!("({op}),Y")));
            }
            "SBCDY" => {
                emit_label!();
                out.push(instr!("SBC", format!("({op}),Y")));
            }
            "ADCDY" => {
                emit_label!();
                out.push(instr!("ADC", format!("({op}),Y")));
            }

            // ---- 16-bit store pairs ----
            "STWD" => {
                emit_label!();
                out.push(instr!("STA", op));
                out.push(instr!("STY", format!("{}+1", op)));
            }
            "STWX" => {
                emit_label!();
                out.push(instr!("STA", op));
                out.push(instr!("STX", format!("{}+1", op)));
            }
            "STXY" => {
                emit_label!();
                out.push(instr!("STX", op));
                out.push(instr!("STY", format!("{}+1", op)));
            }

            // ---- 16-bit load pairs (absolute) ----
            "LDWD" => {
                emit_label!();
                out.push(instr!("LDA", op));
                out.push(instr!("LDY", format!("{}+1", op)));
            }
            "LDWX" => {
                emit_label!();
                out.push(instr!("LDA", op));
                out.push(instr!("LDX", format!("{}+1", op)));
            }
            "LDXY" => {
                emit_label!();
                out.push(instr!("LDX", op));
                out.push(instr!("LDY", format!("{}+1", op)));
            }

            // ---- 16-bit load-immediate pairs ----
            // LDWDI <expr>  ->  LDA #<expr   LDY #>expr
            "LDWDI" => {
                emit_label!();
                let op = strip_grouping_angles(&op);
                out.push(instr!("LDA", format!("#<{}", op)));
                out.push(instr!("LDY", format!("#>{}", op)));
            }
            "LDWXI" => {
                emit_label!();
                let op = strip_grouping_angles(&op);
                out.push(instr!("LDA", format!("#<{}", op)));
                out.push(instr!("LDX", format!("#>{}", op)));
            }
            "LDXYI" => {
                emit_label!();
                let op = strip_grouping_angles(&op);
                out.push(instr!("LDX", format!("#<{}", op)));
                out.push(instr!("LDY", format!("#>{}", op)));
            }

            // ---- Push/pull 16-bit ----
            "PSHWD" => {
                emit_label!();
                out.push(instr!("LDA", format!("{}+1", op)));
                out.push(instr!("PHA"));
                out.push(instr!("LDA", op));
                out.push(instr!("PHA"));
            }
            "PULWD" => {
                emit_label!();
                out.push(instr!("PLA"));
                out.push(instr!("STA", op));
                out.push(instr!("PLA"));
                out.push(instr!("STA", format!("{}+1", op)));
            }

            // ---- CLR / COM ----
            "CLR" => {
                emit_label!();
                out.push(instr!("LDA", "#0"));
                out.push(instr!("STA", op));
            }
            "COM" => {
                emit_label!();
                out.push(instr!("LDA", op));
                out.push(instr!("EOR", "#$FF"));
                out.push(instr!("STA", op));
            }

            // ---- SYNCHK n -> LDA #n / JSR SYNCHR ----
            "SYNCHK" => {
                emit_label!();
                out.push(instr!("LDA", format!("#{}", op)));
                out.push(instr!("JSR", "SYNCHR"));
            }

            // ---- JMPD ptr -> JMP (ptr) ----
            "JMPD" => {
                emit_label!();
                out.push(instr!("JMP", format!("({})", op)));
            }

            // ---- ACRLF -> .byte $0D, $0A ----
            "ACRLF" => {
                out.push(FlatStmt::Bytes {
                    label,
                    values: vec!["$0D".to_string(), "$0A".to_string()],
                });
            }

            // ---- Long conditional jumps ----
            "JEQ" => self.emit_long_branch("BNE", op, label, out),
            "JNE" => self.emit_long_branch("BEQ", op, label, out),
            "JCS" => self.emit_long_branch("BCC", op, label, out),
            "JCC" => self.emit_long_branch("BCS", op, label, out),
            "JMI" => self.emit_long_branch("BPL", op, label, out),
            "JPL" => self.emit_long_branch("BMI", op, label, out),
            "JVS" => self.emit_long_branch("BVC", op, label, out),
            "JVC" => self.emit_long_branch("BVS", op, label, out),

            // ---- Check if it's a user-defined macro we haven't seen yet
            //      (forward references are handled by re-parsing) ----
            _ => {
                // Check if this is a known macro
                let upper = mnemonic.to_ascii_uppercase();
                if let Some(def) = self.macros.get(&upper).cloned() {
                    if let Some(ref lbl) = label {
                        out.push(FlatStmt::Label(lbl.clone()));
                    }
                    let arg_str = operand.unwrap_or("").to_string();
                    let expanded_lines = substitute_macro_body(&def.params, &def.body, &arg_str);
                    let sub_nodes = parse(&expanded_lines.join("\n"))?;
                    for n in sub_nodes {
                        self.expand_node(n, out)?;
                    }
                } else {
                    // Real instruction — pass through
                    out.push(FlatStmt::Instr {
                        label,
                        mnemonic: mnemonic.to_string(),
                        operand: operand.map(|s| s.to_string()),
                    });
                }
            }
        }
        Ok(())
    }

    /// Emit the 5-byte inverted-branch-over-JMP form.
    fn emit_long_branch(
        &mut self,
        inv_branch: &str,
        target: &str,
        label: Option<String>,
        out: &mut Vec<FlatStmt>,
    ) {
        let lj_label = format!("__LJ{}", self.lj_counter);
        self.lj_counter += 1;

        if let Some(lbl) = label {
            out.push(FlatStmt::Label(lbl));
        }
        out.push(FlatStmt::Instr {
            label: None,
            mnemonic: inv_branch.to_string(),
            operand: Some(lj_label.clone()),
        });
        out.push(FlatStmt::Instr {
            label: None,
            mnemonic: "JMP".to_string(),
            operand: Some(target.to_string()),
        });
        out.push(FlatStmt::Label(lj_label));
    }

    // -----------------------------------------------------------------------
    // Expression evaluator (best-effort for compile-time constants)
    // -----------------------------------------------------------------------

    /// Evaluate a MACRO-10 expression string to an i64.
    /// Unknown symbols return 0 (safe for conditionals where 0 == false).
    pub fn eval_expr_str(&self, expr: &str) -> i64 {
        eval_expr(expr.trim(), &self.symbols, self.radix)
    }
}

// ---------------------------------------------------------------------------
// Expression evaluator (free function, recursive)
// ---------------------------------------------------------------------------

fn eval_expr(expr: &str, syms: &HashMap<String, i64>, radix: u32) -> i64 {
    let expr = expr.trim();
    if expr.is_empty() {
        return 0;
    }

    // Try to evaluate using a simple recursive-descent parser.
    let (val, _) = eval_add(expr, syms, radix);
    val
}

fn try_eval_expr(expr: &str, syms: &HashMap<String, i64>, radix: u32) -> Option<i64> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Some(0);
    }

    let (val, rest) = try_eval_add(expr, syms, radix)?;
    if rest.trim().is_empty() {
        Some(val)
    } else {
        None
    }
}

fn try_eval_add<'a>(s: &'a str, syms: &HashMap<String, i64>, radix: u32) -> Option<(i64, &'a str)> {
    let (mut val, rest) = try_eval_mul(s, syms, radix)?;
    let mut rest = rest.trim_start();
    loop {
        if rest.starts_with('+') {
            let (rval, r) = try_eval_mul(rest[1..].trim_start(), syms, radix)?;
            val += rval;
            rest = r.trim_start();
        } else if rest.starts_with('-') {
            let (rval, r) = try_eval_mul(rest[1..].trim_start(), syms, radix)?;
            val -= rval;
            rest = r.trim_start();
        } else if rest.starts_with('!') {
            let (rval, r) = try_eval_mul(rest[1..].trim_start(), syms, radix)?;
            val |= rval;
            rest = r.trim_start();
        } else if rest.starts_with('&') {
            let (rval, r) = try_eval_mul(rest[1..].trim_start(), syms, radix)?;
            val &= rval;
            rest = r.trim_start();
        } else {
            break;
        }
    }
    Some((val, rest))
}

fn try_eval_mul<'a>(s: &'a str, syms: &HashMap<String, i64>, radix: u32) -> Option<(i64, &'a str)> {
    let (mut val, rest) = try_eval_unary(s, syms, radix)?;
    let mut rest = rest.trim_start();
    loop {
        if rest.starts_with('*') {
            let (rval, r) = try_eval_unary(rest[1..].trim_start(), syms, radix)?;
            val *= rval;
            rest = r.trim_start();
        } else if rest.starts_with('/') {
            let (rval, r) = try_eval_unary(rest[1..].trim_start(), syms, radix)?;
            val = if rval != 0 { val / rval } else { 0 };
            rest = r.trim_start();
        } else {
            break;
        }
    }
    Some((val, rest))
}

fn try_eval_unary<'a>(
    s: &'a str,
    syms: &HashMap<String, i64>,
    radix: u32,
) -> Option<(i64, &'a str)> {
    let s = s.trim_start();
    if s.starts_with('-') {
        let (v, r) = try_eval_atom(s[1..].trim_start(), syms, radix)?;
        return Some((-v, r));
    }
    if s.starts_with('+') {
        return try_eval_atom(s[1..].trim_start(), syms, radix);
    }
    try_eval_atom(s, syms, radix)
}

fn try_eval_atom<'a>(
    s: &'a str,
    syms: &HashMap<String, i64>,
    radix: u32,
) -> Option<(i64, &'a str)> {
    let s = s.trim_start();
    if s.starts_with('<') {
        let body = collect_angle_body_str(&s[1..]);
        let val = try_eval_expr(&body, syms, radix)?;
        let consumed = body.len() + 2;
        return Some((val, &s[consumed.min(s.len())..]));
    }
    if s.starts_with("^O") || s.starts_with("^o") {
        let rest = &s[2..].trim_start();
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let digits = &rest[..end];
        let val = i64::from_str_radix(digits, 8).ok()?;
        return Some((val, &rest[end..]));
    }
    if s.starts_with("^D") || s.starts_with("^d") {
        let rest = &s[2..].trim_start();
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let digits = &rest[..end];
        let val: i64 = digits.parse().ok()?;
        return Some((val, &rest[end..]));
    }
    if s.starts_with('$') {
        let rest = &s[1..];
        let end = rest
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(rest.len());
        let digits = &rest[..end];
        let val = i64::from_str_radix(digits, 16).ok()?;
        return Some((val, &rest[end..]));
    }
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        let val = parse_bare_number(&s[..end], radix)?;
        return Some((val, &s[end..]));
    }
    if s.starts_with('\'') && s.len() >= 3 {
        let ch = s.chars().nth(1)? as i64;
        let after = s[3..].trim_start();
        return Some((ch, after));
    }
    if s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '.') {
        let end = s
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '.' && c != '$')
            .unwrap_or(s.len());
        let sym = canonicalize_symbol_name(&s[..end]);
        let val = syms.get(&sym).copied()?;
        return Some((val, &s[end..]));
    }
    None
}

fn eval_add<'a>(s: &'a str, syms: &HashMap<String, i64>, radix: u32) -> (i64, &'a str) {
    let (mut val, rest) = eval_mul(s, syms, radix);
    let mut rest = rest.trim_start();
    loop {
        if rest.starts_with('+') {
            let (rval, r) = eval_mul(rest[1..].trim_start(), syms, radix);
            val += rval;
            rest = r.trim_start();
        } else if rest.starts_with('-') {
            let (rval, r) = eval_mul(rest[1..].trim_start(), syms, radix);
            val -= rval;
            rest = r.trim_start();
        } else if rest.starts_with('!') {
            // Bitwise OR
            let (rval, r) = eval_mul(rest[1..].trim_start(), syms, radix);
            val |= rval;
            rest = r.trim_start();
        } else if rest.starts_with('&') {
            let (rval, r) = eval_mul(rest[1..].trim_start(), syms, radix);
            val &= rval;
            rest = r.trim_start();
        } else {
            break;
        }
    }
    (val, rest)
}

fn eval_mul<'a>(s: &'a str, syms: &HashMap<String, i64>, radix: u32) -> (i64, &'a str) {
    let (mut val, rest) = eval_unary(s, syms, radix);
    let mut rest = rest.trim_start();
    loop {
        if rest.starts_with('*') {
            let (rval, r) = eval_unary(rest[1..].trim_start(), syms, radix);
            val *= rval;
            rest = r.trim_start();
        } else if rest.starts_with('/') {
            let (rval, r) = eval_unary(rest[1..].trim_start(), syms, radix);
            val = if rval != 0 { val / rval } else { 0 };
            rest = r.trim_start();
        } else {
            break;
        }
    }
    (val, rest)
}

fn eval_unary<'a>(s: &'a str, syms: &HashMap<String, i64>, radix: u32) -> (i64, &'a str) {
    let s = s.trim_start();
    if s.starts_with('-') {
        let (v, r) = eval_atom(s[1..].trim_start(), syms, radix);
        return (-v, r);
    }
    if s.starts_with('+') {
        return eval_atom(s[1..].trim_start(), syms, radix);
    }
    eval_atom(s, syms, radix)
}

fn eval_atom<'a>(s: &'a str, syms: &HashMap<String, i64>, radix: u32) -> (i64, &'a str) {
    let s = s.trim_start();
    // Angle-bracket grouping
    if s.starts_with('<') {
        let body = collect_angle_body_str(&s[1..]);
        let val = eval_expr(&body, syms, radix);
        let consumed = body.len() + 2; // < body >
        return (val, &s[consumed.min(s.len())..]);
    }
    // Octal: ^O<digits> or ^O digits
    if s.starts_with("^O") || s.starts_with("^o") {
        let rest = &s[2..].trim_start();
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let digits = &rest[..end];
        let val = i64::from_str_radix(digits, 8).unwrap_or(0);
        return (val, &rest[end..]);
    }
    // Decimal: ^D<digits> or ^D digits
    if s.starts_with("^D") || s.starts_with("^d") {
        let rest = &s[2..].trim_start();
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let digits = &rest[..end];
        let val: i64 = digits.parse().unwrap_or(0);
        return (val, &rest[end..]);
    }
    // Hex: $HH
    if s.starts_with('$') {
        let rest = &s[1..];
        let end = rest
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(rest.len());
        let digits = &rest[..end];
        let val = i64::from_str_radix(digits, 16).unwrap_or(0);
        return (val, &rest[end..]);
    }
    // Decimal number
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        let val = parse_bare_number(&s[..end], radix).unwrap_or(0);
        return (val, &s[end..]);
    }
    // Character literal: 'X'
    if s.starts_with('\'') && s.len() >= 3 {
        let ch = s.chars().nth(1).unwrap_or('\0') as i64;
        let after = s[3..].trim_start();
        return (ch, after);
    }
    // Symbol
    if s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '.') {
        let end = s
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '.' && c != '$')
            .unwrap_or(s.len());
        let sym = canonicalize_symbol_name(&s[..end]);
        let val = syms.get(&sym).copied().unwrap_or(0);
        return (val, &s[end..]);
    }
    (0, s)
}

/// Like `collect_angle_body` but works on &str and returns the body as String.
fn collect_angle_body_str(s: &str) -> String {
    let mut depth = 1usize;
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '<' => {
                depth += 1;
                out.push(ch);
            }
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return out;
                }
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn parse_bare_number(digits: &str, radix: u32) -> Option<i64> {
    i64::from_str_radix(digits, radix)
        .ok()
        .or_else(|| digits.parse().ok())
}

// ---------------------------------------------------------------------------
// Macro body substitution
// ---------------------------------------------------------------------------

/// Substitute parameter `params[i]` → `args[i]` throughout `body_lines`.
///
/// In the MACRO-10 dialect, parameter references appear as `<PARAM>` inside
/// the body — but since all `<>` are balanced, the depth-counter already
/// collected the body correctly.  Simple text substitution of the param name
/// (case-insensitive) is correct for single-parameter macros.
fn substitute_macro_body(params: &[String], body: &[String], arg: &str) -> Vec<String> {
    if params.is_empty() {
        return body.to_vec();
    }
    // Build a substitution list: (param_name, replacement)
    let subs: Vec<(String, String)> = {
        // If only one param, that arg is the whole string.
        // If multiple params, split arg by comma (depth-aware).
        let arg_parts = split_macro_args(arg);
        params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let repl = arg_parts
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                (p.to_ascii_uppercase(), repl)
            })
            .collect()
    };

    body.iter()
        .map(|line| substitute_params_in_line(line, &subs))
        .collect()
}

/// Split macro argument string by commas, respecting `<>` depth.
fn split_macro_args(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    for ch in s.chars() {
        match ch {
            '<' => {
                depth += 1;
                current.push(ch);
            }
            '>' if depth > 0 => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    let last = current.trim().to_string();
    if !last.is_empty() {
        parts.push(last);
    }
    parts
}

/// Substitute param references in a single body line.
/// Handles: `<PARAM>` → replacement, and bare `PARAM` as a whole word.
fn substitute_params_in_line(line: &str, subs: &[(String, String)]) -> String {
    let mut result = line.to_string();
    for (param, repl) in subs {
        // Replace `<PARAM>` occurrences (inside angle brackets)
        let pat = format!("<{}>", param);
        result = result.replace(&pat, repl);
        // Also replace bare param name as a whole word (case-insensitive)
        result = replace_word_ci(&result, param, repl);
    }
    result
}

/// Replace all whole-word occurrences of `word` (case-insensitive) in `s`
/// with `replacement`.
fn replace_word_ci(s: &str, word: &str, replacement: &str) -> String {
    let upper_s = s.to_ascii_uppercase();
    let upper_word = word.to_ascii_uppercase();
    let mut result = String::with_capacity(s.len());
    let mut i = 0;
    let word_len = upper_word.len();
    let bytes_u = upper_s.as_bytes();
    let bytes_w = upper_word.as_bytes();
    while i < bytes_u.len() {
        if i + word_len <= bytes_u.len() && bytes_u[i..i + word_len] == *bytes_w {
            // Check word boundaries
            let before_ok =
                i == 0 || !bytes_u[i - 1].is_ascii_alphanumeric() && bytes_u[i - 1] != b'_';
            let after_ok = i + word_len >= bytes_u.len()
                || (!bytes_u[i + word_len].is_ascii_alphanumeric()
                    && bytes_u[i + word_len] != b'_');
            if before_ok && after_ok {
                result.push_str(replacement);
                i += word_len;
                continue;
            }
        }
        result.push(s.chars().nth(i).unwrap_or(' '));
        i += 1;
    }
    result
}
