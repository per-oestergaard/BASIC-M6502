/// Two-pass 6502 assembler.
///
/// Input:  `&[FlatStmt]`  (output of the expander)
/// Output: `(Vec<u8>, HashMap<String,u16>)`  (binary image, symbol map)
///
/// The binary image runs from address 0 through the highest address written.
/// Gaps (ORG jumps over unwritten space) are filled with 0x00.
use std::collections::HashMap;

use tracing::trace;

use crate::ast::FlatStmt;
use crate::opcode::AddrMode::{self, *};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct Assembler;

impl Assembler {
    pub fn new() -> Self {
        Self
    }

    /// Assemble a flat statement list into a binary image.
    pub fn assemble(&self, stmts: &[FlatStmt]) -> anyhow::Result<(Vec<u8>, HashMap<String, u16>)> {
        let symbols = pass1(stmts);
        let (bytes, _min_addr) = pass2(stmts, &symbols);

        // Only expose labels (integer-valued equates with address meaning are
        // excluded — they are kept as i64 equates in the full map).
        let labels: HashMap<String, u16> = symbols
            .iter()
            .filter_map(|(k, &v)| {
                if v >= 0 && v <= 0xFFFF {
                    Some((k.clone(), v as u16))
                } else {
                    None
                }
            })
            .collect();

        Ok((bytes, labels))
    }
}

// ---------------------------------------------------------------------------
// Pass 1 — build symbol table, assign addresses to labels
// ---------------------------------------------------------------------------

fn pass1(stmts: &[FlatStmt]) -> HashMap<String, i64> {
    let mut sym: HashMap<String, i64> = HashMap::new();
    let mut pc: u16 = 0;

    for stmt in stmts {
        match stmt {
            FlatStmt::Org(addr) => {
                pc = *addr;
            }
            FlatStmt::Label(name) => {
                sym.insert(name.clone(), pc as i64);
                trace!(target: "assembler6502::assemble", label = %name, pc, "label");
            }
            FlatStmt::Equate { name, value } => {
                sym.insert(name.clone(), *value);
            }
            FlatStmt::Bytes { label, values } => {
                if let Some(lbl) = label {
                    sym.insert(lbl.clone(), pc as i64);
                }
                pc = pc.wrapping_add(values.len() as u16);
            }
            FlatStmt::Word { label, .. } => {
                if let Some(lbl) = label {
                    sym.insert(lbl.clone(), pc as i64);
                }
                pc = pc.wrapping_add(2);
            }
            FlatStmt::Res { label, count } => {
                if let Some(lbl) = label {
                    sym.insert(lbl.clone(), pc as i64);
                }
                pc = pc.wrapping_add(*count);
            }
            FlatStmt::Instr {
                label,
                mnemonic,
                operand,
            } => {
                if let Some(lbl) = label {
                    sym.insert(lbl.clone(), pc as i64);
                }
                let size = instr_size(mnemonic, operand.as_deref(), pc, &sym);
                pc = pc.wrapping_add(size as u16);
            }
        }
    }

    sym
}

// ---------------------------------------------------------------------------
// Pass 2 — emit bytes
// ---------------------------------------------------------------------------

fn pass2(stmts: &[FlatStmt], sym: &HashMap<String, i64>) -> (Vec<u8>, u16) {
    // Write into the full 64 KB address space, then trim.
    let mut mem = vec![0u8; 0x10000];
    let mut pc: u16 = 0;
    let mut min_written: u16 = 0xFFFF;
    let mut max_written: u16 = 0;
    let mut anything_written = false;

    let write = |mem: &mut Vec<u8>, addr: u16, b: u8| {
        mem[addr as usize] = b;
    };

    let mark = |addr: u16, anything_written: &mut bool, min_w: &mut u16, max_w: &mut u16| {
        if !*anything_written || addr < *min_w {
            *min_w = addr;
        }
        if addr >= *max_w {
            *max_w = addr + 1;
        }
        *anything_written = true;
    };

    for stmt in stmts {
        match stmt {
            FlatStmt::Org(addr) => {
                pc = *addr;
            }
            FlatStmt::Label(_) | FlatStmt::Equate { .. } => {}
            FlatStmt::Res { count, .. } => {
                pc = pc.wrapping_add(*count);
            }
            FlatStmt::Bytes { values, .. } => {
                for v in values {
                    let b = eval_byte(v, pc, sym);
                    write(&mut mem, pc, b);
                    mark(
                        pc,
                        &mut anything_written,
                        &mut min_written,
                        &mut max_written,
                    );
                    pc = pc.wrapping_add(1);
                }
            }
            FlatStmt::Word { expr, .. } => {
                let val = eval_expr(expr, pc, sym) as u16;
                write(&mut mem, pc, (val & 0xFF) as u8);
                mark(
                    pc,
                    &mut anything_written,
                    &mut min_written,
                    &mut max_written,
                );
                write(&mut mem, pc + 1, (val >> 8) as u8);
                mark(
                    pc + 1,
                    &mut anything_written,
                    &mut min_written,
                    &mut max_written,
                );
                pc = pc.wrapping_add(2);
            }
            FlatStmt::Instr {
                mnemonic, operand, ..
            } => {
                let op = operand.as_deref();
                let (mode, val) = parse_operand(op, pc, sym, mnemonic);

                let opcode = resolve_opcode(mnemonic, mode);
                if let Some(oc) = opcode {
                    trace!(
                        target: "assembler6502::assemble",
                        pc,
                        %mnemonic,
                        ?mode,
                        "emit"
                    );
                    write(&mut mem, pc, oc);
                    mark(
                        pc,
                        &mut anything_written,
                        &mut min_written,
                        &mut max_written,
                    );
                    pc = pc.wrapping_add(1);

                    match crate::opcode::size_for(mode) - 1 {
                        1 => {
                            // For Rel, val is already a signed offset.
                            let byte = if mode == Rel {
                                (val & 0xFF) as u8
                            } else {
                                (val & 0xFF) as u8
                            };
                            write(&mut mem, pc, byte);
                            mark(
                                pc,
                                &mut anything_written,
                                &mut min_written,
                                &mut max_written,
                            );
                            pc = pc.wrapping_add(1);
                        }
                        2 => {
                            write(&mut mem, pc, (val & 0xFF) as u8);
                            mark(
                                pc,
                                &mut anything_written,
                                &mut min_written,
                                &mut max_written,
                            );
                            write(&mut mem, pc + 1, ((val >> 8) & 0xFF) as u8);
                            mark(
                                pc + 1,
                                &mut anything_written,
                                &mut min_written,
                                &mut max_written,
                            );
                            pc = pc.wrapping_add(2);
                        }
                        _ => {} // Imp/Acc: 1 byte already written
                    }
                } else {
                    // Unknown opcode — advance by estimated size so subsequent
                    // labels stay in sync with pass-1 addresses.
                    let size = instr_size(mnemonic, op, pc, sym) as u16;
                    trace!(
                        target: "assembler6502::assemble",
                        pc,
                        %mnemonic,
                        "no opcode — skipping"
                    );
                    pc = pc.wrapping_add(size);
                }
            }
        }
    }

    if !anything_written {
        return (vec![], 0);
    }

    let out = mem[min_written as usize..max_written as usize].to_vec();
    (out, min_written)
}

// ---------------------------------------------------------------------------
// Opcode resolution (tries exact mode, then ZP→Abs fallbacks)
// ---------------------------------------------------------------------------

fn resolve_opcode(mnemonic: &str, mode: AddrMode) -> Option<u8> {
    let m = mnemonic.to_ascii_uppercase();
    if let Some(oc) = crate::opcode::lookup(&m, mode) {
        return Some(oc);
    }
    let fallback = match mode {
        Zp => Some(Abs),
        ZpX => Some(AbsX),
        ZpY => Some(AbsY),
        _ => None,
    };
    fallback.and_then(|fb| crate::opcode::lookup(&m, fb))
}

// ---------------------------------------------------------------------------
// Instruction size estimation (used by both passes)
// ---------------------------------------------------------------------------

fn instr_size(mnemonic: &str, operand: Option<&str>, pc: u16, sym: &HashMap<String, i64>) -> usize {
    let (mode, _) = parse_operand(operand, pc, sym, mnemonic);
    crate::opcode::size_for(mode)
}

// ---------------------------------------------------------------------------
// Operand parser → (AddrMode, value)
// ---------------------------------------------------------------------------

fn parse_operand(
    operand: Option<&str>,
    pc: u16,
    sym: &HashMap<String, i64>,
    mnemonic: &str,
) -> (AddrMode, i64) {
    let s = match operand {
        None => return (Imp, 0),
        Some(s) => s.trim().trim_end_matches(',').trim(),
    };

    if s.is_empty() {
        return (Imp, 0);
    }

    // Accumulator: "A" or "A,"
    if s.eq_ignore_ascii_case("A") {
        let m = mnemonic.to_ascii_uppercase();
        if matches!(m.as_str(), "ASL" | "LSR" | "ROL" | "ROR") {
            return (Acc, 0);
        }
        return (Imp, 0);
    }

    // Immediate: #expr
    if let Some(rest) = s.strip_prefix('#') {
        let val = eval_expr(rest, pc, sym);
        return (Imm, val);
    }

    // Indexed indirect: (expr,X)
    if s.starts_with('(') {
        let inner = &s[1..];
        let upper = inner.to_ascii_uppercase();
        if let Some(xi) = upper.find(",X)") {
            let val = eval_expr(&inner[..xi], pc, sym);
            return (IndX, val);
        }
        // Indirect indexed: (expr),Y
        if let Some(ci) = inner.find(')') {
            let addr_str = &inner[..ci];
            let after = inner[ci + 1..]
                .trim()
                .trim_end_matches(',')
                .to_ascii_uppercase();
            let val = eval_expr(addr_str, pc, sym);
            if after == ",Y" || after == "Y" {
                return (IndY, val);
            }
            // Indirect absolute: (expr)
            return (Ind, val);
        }
    }

    // Check for indexed suffix: find last `,X` or `,Y` at depth 0
    if let Some(ci) = find_last_index_comma(s) {
        let addr_str = s[..ci].trim();
        let idx = s[ci + 1..]
            .trim()
            .trim_end_matches(',')
            .to_ascii_uppercase();
        let val = eval_expr(addr_str, pc, sym);
        if idx == "X" {
            let m = mnemonic.to_ascii_uppercase();
            if val >= 0 && val < 256 && crate::opcode::lookup(&m, ZpX).is_some() {
                return (ZpX, val);
            }
            return (AbsX, val);
        }
        if idx == "Y" {
            let m = mnemonic.to_ascii_uppercase();
            if val >= 0 && val < 256 && crate::opcode::lookup(&m, ZpY).is_some() {
                return (ZpY, val);
            }
            return (AbsY, val);
        }
    }

    // Plain address: relative (branches), ZP, or absolute
    let val = eval_expr(s, pc, sym);
    let m = mnemonic.to_ascii_uppercase();
    if matches!(
        m.as_str(),
        "BCC" | "BCS" | "BEQ" | "BMI" | "BNE" | "BPL" | "BVC" | "BVS"
    ) {
        // Convert absolute address to signed 8-bit relative offset.
        let offset = val - (pc as i64 + 2);
        return (Rel, offset);
    }
    if val >= 0 && val < 256 && crate::opcode::lookup(&m, Zp).is_some() {
        return (Zp, val);
    }
    (Abs, val)
}

/// Find the last `,X` or `,Y` suffix at angle-bracket / paren depth 0.
/// Returns the byte index of the comma, or None.
fn find_last_index_comma(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth: i32 = 0;
    let mut last_comma: Option<usize> = None;

    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' | b'<' => depth += 1,
            b')' | b'>' if depth > 0 => depth -= 1,
            b',' if depth == 0 => {
                // Only accept if what follows (trimming trailing commas) is X or Y
                let rest = s[i + 1..].trim().trim_end_matches(',').to_ascii_uppercase();
                if rest == "X" || rest == "Y" {
                    last_comma = Some(i);
                }
            }
            _ => {}
        }
    }
    last_comma
}

// ---------------------------------------------------------------------------
// Byte evaluator
// ---------------------------------------------------------------------------

fn eval_byte(expr: &str, pc: u16, sym: &HashMap<String, i64>) -> u8 {
    (eval_expr(expr.trim(), pc, sym) & 0xFF) as u8
}

// ---------------------------------------------------------------------------
// Expression evaluator (recursive descent)
// Handles: decimal, $hex, ^Ooct, "char", 'char', symbols, `.` (PC),
//          +  -  *  /  &(AND)  !(OR)  <expr>(group)  <expr(lowbyte)  >expr(hibyte)
// ---------------------------------------------------------------------------

pub fn eval_expr(expr: &str, pc: u16, sym: &HashMap<String, i64>) -> i64 {
    let expr = expr.trim();
    if expr.is_empty() {
        return 0;
    }
    let (val, _) = expr_add(expr, pc, sym);
    val
}

fn expr_add<'a>(s: &'a str, pc: u16, sym: &HashMap<String, i64>) -> (i64, &'a str) {
    let (mut val, rest) = expr_mul(s, pc, sym);
    let mut rest = rest.trim_start();
    loop {
        if rest.starts_with('+') && !rest.starts_with("++") {
            let (r, rem) = expr_mul(rest[1..].trim_start(), pc, sym);
            val += r;
            rest = rem.trim_start();
        } else if rest.starts_with('-') {
            let (r, rem) = expr_mul(rest[1..].trim_start(), pc, sym);
            val -= r;
            rest = rem.trim_start();
        } else if rest.starts_with('!') {
            let (r, rem) = expr_mul(rest[1..].trim_start(), pc, sym);
            val |= r;
            rest = rem.trim_start();
        } else if rest.starts_with('&') {
            let (r, rem) = expr_mul(rest[1..].trim_start(), pc, sym);
            val &= r;
            rest = rem.trim_start();
        } else {
            break;
        }
    }
    (val, rest)
}

fn expr_mul<'a>(s: &'a str, pc: u16, sym: &HashMap<String, i64>) -> (i64, &'a str) {
    let (mut val, rest) = expr_unary(s, pc, sym);
    let mut rest = rest.trim_start();
    loop {
        if rest.starts_with('*') {
            let (r, rem) = expr_unary(rest[1..].trim_start(), pc, sym);
            val *= r;
            rest = rem.trim_start();
        } else if rest.starts_with('/') {
            let (r, rem) = expr_unary(rest[1..].trim_start(), pc, sym);
            val = if r != 0 { val / r } else { 0 };
            rest = rem.trim_start();
        } else {
            break;
        }
    }
    (val, rest)
}

fn expr_unary<'a>(s: &'a str, pc: u16, sym: &HashMap<String, i64>) -> (i64, &'a str) {
    let s = s.trim_start();
    if s.starts_with('-') {
        let (v, r) = expr_atom(&s[1..].trim_start(), pc, sym);
        return (-v, r);
    }
    if s.starts_with('+') {
        return expr_atom(&s[1..].trim_start(), pc, sym);
    }
    // High-byte prefix: >expr → (eval(expr) >> 8) & 0xFF
    if s.starts_with('>') && !s[1..].trim_start().starts_with('<') {
        let (v, r) = expr_atom(&s[1..].trim_start(), pc, sym);
        return ((v >> 8) & 0xFF, r);
    }
    expr_atom(s, pc, sym)
}

fn expr_atom<'a>(s: &'a str, pc: u16, sym: &HashMap<String, i64>) -> (i64, &'a str) {
    let s = s.trim_start();
    if s.is_empty() {
        return (0, s);
    }

    // Angle-bracket group: <expr> or <expr (unbalanced = low-byte)
    if s.starts_with('<') {
        let body = collect_angle(s);
        let val = eval_expr(&body, pc, sym);
        let consumed = (body.len() + 2).min(s.len());
        // Check if it was actually balanced (has matching >)
        let balanced = s[1..].contains('>');
        let result = if balanced { val } else { val & 0xFF };
        return (result, &s[consumed..]);
    }

    // Paren grouping: (expr)
    if s.starts_with('(') {
        if let Some(ci) = find_close_paren(s) {
            let inner = &s[1..ci];
            let val = eval_expr(inner, pc, sym);
            return (val, &s[ci + 1..]);
        }
    }

    // Current PC: `.`
    if s.starts_with('.') {
        let next = s.as_bytes().get(1);
        let is_sym_cont =
            next.is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.');
        if !is_sym_cont {
            return (pc as i64, &s[1..]);
        }
    }

    // Octal: ^O or ^o
    if s.starts_with("^O") || s.starts_with("^o") {
        let rest = s[2..].trim_start();
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let val = i64::from_str_radix(&rest[..end], 8).unwrap_or(0);
        return (val, &rest[end..]);
    }

    // Hex: $xx
    if s.starts_with('$') {
        let rest = &s[1..];
        let end = rest
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(rest.len());
        let val = i64::from_str_radix(&rest[..end], 16).unwrap_or(0);
        return (val, &rest[end..]);
    }

    // Decimal
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        let val: i64 = s[..end].parse().unwrap_or(0);
        return (val, &s[end..]);
    }

    // Double-quoted char literal: "c"  (value = ASCII of first char)
    if s.starts_with('"') {
        let rest = &s[1..];
        let end = rest.find('"').unwrap_or(rest.len());
        let val = rest.bytes().next().map(|b| b as i64).unwrap_or(0);
        let skip = (end + 2).min(s.len());
        return (val, &s[skip..]);
    }

    // Single-quoted char literal: 'c'
    if s.starts_with('\'') {
        let val = s.chars().nth(1).unwrap_or('\0') as i64;
        let skip = if s.len() >= 3 { 3 } else { s.len() };
        return (val, &s[skip..]);
    }

    // Symbol: starts with letter, _, ., $, or %  (% for MACRO-10 local labels)
    if s.starts_with(|c: char| {
        c.is_ascii_alphabetic() || c == '_' || c == '.' || c == '$' || c == '%'
    }) {
        let end = s
            .find(|c: char| {
                !c.is_ascii_alphanumeric() && c != '_' && c != '.' && c != '$' && c != '%'
            })
            .unwrap_or(s.len());
        let name = s[..end].to_ascii_uppercase();
        let val = sym.get(&name).copied().unwrap_or(0);
        return (val, &s[end..]);
    }

    (0, s)
}

fn collect_angle(s: &str) -> String {
    // s starts with '<'; collect body up to matching '>'
    let mut depth = 1usize;
    let mut out = String::new();
    for ch in s[1..].chars() {
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

fn find_close_paren(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
