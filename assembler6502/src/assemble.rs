use crate::opcode::{lookup, size_for, AddrMode};
use crate::parse::{lex_line, Line};
use crate::preprocess;
use anyhow::{bail, Result};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct AsmOptions {
    pub start_org: Option<u16>,
}

pub struct Assembler {
    pub opts: AsmOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixupKind {
    Plain,
    Low,
    High,
}

#[derive(Debug, Clone)]
struct Fixup {
    offset: usize,
    symbol: String,
    mode: AddrMode,
    instr_addr: u16,
    addend: i16,
    mul: i16,
    kind: FixupKind,
}

impl Assembler {
    pub fn new(opts: AsmOptions) -> Self {
        Self { opts }
    }
    pub fn assemble(&self, source: &str) -> Result<(Vec<u8>, HashMap<String, u16>)> {
        // Preprocess first
        let pre = preprocess::run(source);
        let mut lines_parsed: Vec<Line> = Vec::new();
        for raw in pre.lines.iter() {
            lines_parsed.push(lex_line(raw));
        }
        let mut sym: HashMap<String, u16> = HashMap::new();
        // Seed symbols from preprocessor (equates) where they resolved
        for (k, v) in pre.symbols.iter() {
            if *v >= 0 && *v <= 0xFFFF {
                sym.insert(k.to_string(), *v as u16);
            }
        }
        let mut pc: u16 = self.opts.start_org.unwrap_or(0);
        // Pass 1: assign addresses using inferred sizes
        for l in &lines_parsed {
            match l {
                Line::Org(v) => {
                    pc = *v;
                }
                Line::Equ { label, expr: _ } => {
                    sym.entry(label.clone()).or_insert(0);
                } // placeholder, resolved pass2
                Line::Label(name) => {
                    define_symbol(&mut sym, name, pc)?;
                }
                Line::Instr {
                    label,
                    mnem,
                    operand,
                } => {
                    if let Some(lab) = label {
                        define_symbol(&mut sym, lab, pc)?;
                    }
                    let mode = infer_mode(mnem, operand.as_deref(), &sym);
                    pc = pc.wrapping_add(size_for(mode) as u16);
                }
                Line::DataBytes { label, bytes } => {
                    if let Some(lab) = label {
                        define_symbol(&mut sym, lab, pc)?;
                    }
                    pc = pc.wrapping_add(bytes.len() as u16);
                }
                Line::DataWord { label, expr: _ } => {
                    if let Some(lab) = label {
                        define_symbol(&mut sym, lab, pc)?;
                    }
                    pc = pc.wrapping_add(2);
                }
                Line::Block { label, expr } => {
                    if let Some(lab) = label {
                        define_symbol(&mut sym, lab, pc)?;
                    }
                    // Evaluate the expression to get the size
                    let size = if let Ok(val) = expr.parse::<usize>() {
                        val
                    } else if let Ok(val) = eval_multi_symbol_expr(expr, &sym) {
                        val as usize
                    } else {
                        // If symbol not yet defined, assume 0 for now (will be resolved in later passes)
                        0
                    };
                    pc = pc.wrapping_add(size as u16);
                }
                _ => {}
            }
        }
        // Pass 1.5: Resolve equates (with multiple iterations for forward references)
        let max_iterations = 10;
        for _iteration in 0..max_iterations {
            let mut changed = false;
            for l in &lines_parsed {
                if let Line::Equ { label, expr } = l {
                    // Skip if already resolved to non-zero
                    if let Some(&v) = sym.get(label.as_str()) {
                        if v != 0 {
                            continue;
                        }
                    }
                    // Try to parse as a number first
                    if let Ok(v) = parse_number(expr) {
                        sym.insert(label.clone(), v as u16);
                        changed = true;
                    // Otherwise try to resolve as a symbol reference
                    } else if let Some(&v) = sym.get(expr.trim()) {
                        if v != 0 {
                            // Only resolve if target is resolved
                            sym.insert(label.clone(), v);
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        // Pass 2: encode
        pc = self.opts.start_org.unwrap_or(0);
        let mut out: Vec<u8> = Vec::new();
        let mut fixups: Vec<Fixup> = Vec::new();
        for (line_idx, l) in lines_parsed.iter().enumerate() {
            match l {
                Line::Org(v) => {
                    pc = *v;
                    while out.len() < pc as usize {
                        out.push(0);
                    }
                }
                Line::Label(_) => {}
                Line::Equ { label: _, expr: _ } => {
                    // Equates already resolved in pass 1.5
                }
                Line::DataBytes { label: _, bytes } => {
                    ensure_capacity(&mut out, pc as usize);
                    out.extend_from_slice(bytes);
                    pc = pc.wrapping_add(bytes.len() as u16);
                }
                Line::DataWord { label: _, expr } => {
                    ensure_capacity(&mut out, pc as usize);
                    // Try to resolve the expression
                    if let Ok(value) = resolve_value(expr, &sym) {
                        out.push((value & 0xFF) as u8);
                        out.push((value >> 8) as u8);
                    } else if let Ok(value) = eval_multi_symbol_expr(expr, &sym) {
                        out.push((value & 0xFF) as u8);
                        out.push((value >> 8) as u8);
                    } else if is_symbol(expr.trim()) {
                        // Forward reference - add fixup
                        let offset = out.len();
                        out.push(0);
                        out.push(0);
                        fixups.push(Fixup {
                            offset,
                            symbol: expr.trim().to_string(),
                            mode: AddrMode::Abs,
                            instr_addr: pc,
                            addend: 0,
                            mul: 1,
                            kind: FixupKind::Plain,
                        });
                    } else {
                        bail!("Cannot resolve .word expression: {}", expr);
                    }
                    pc = pc.wrapping_add(2);
                }
                Line::Block { label: _, expr } => {
                    // Evaluate the expression to get the size
                    let size = if let Ok(val) = expr.parse::<usize>() {
                        val
                    } else if let Ok(val) = eval_multi_symbol_expr(expr, &sym) {
                        val as usize
                    } else {
                        bail!("Cannot resolve .res expression: {}", expr);
                    };
                    ensure_capacity(&mut out, (pc + size as u16) as usize);
                    for _ in 0..size {
                        out.push(0);
                    }
                    pc = pc.wrapping_add(size as u16);
                }
                Line::Instr {
                    label: _,
                    mnem,
                    operand,
                } => {
                    let mode = infer_mode(mnem, operand.as_deref(), &sym);
                    let out_offset = out.len(); // where opcode will be placed
                    if let Some(op) = lookup(mnem, mode) {
                        ensure_capacity(&mut out, pc as usize);
                        out.push(op); // placeholder for opcode
                                      // ensure space for full instruction so operand writer can use absolute offsets
                        while out.len() < out_offset + size_for(mode) {
                            out.push(0);
                        } // reserve
                        if let Some(oprnd) = operand {
                            build_operand_bytes(
                                mode,
                                oprnd,
                                &sym,
                                pc,
                                out_offset,
                                &mut out,
                                &mut fixups,
                                mnem,
                            )?;
                        }
                        pc = pc.wrapping_add(size_for(mode) as u16);
                    } else {
                        bail!("Opcode not found for {} {:?} at preprocessed line {}", mnem, mode, line_idx + 1);
                    }
                }
                _ => {}
            }
        }
        // Resolve fixups now that all labels collected
        for f in &fixups {
            if std::env::var("DEBUG_FIXUP").is_ok() {
                eprintln!("Fixup: symbol='{}', offset={}, mode={:?}", f.symbol, f.offset, f.mode);
            }
            let target = sym
                .get(&f.symbol)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("Unresolved symbol {}", f.symbol))?
                as u16;
            match f.mode {
                AddrMode::Abs | AddrMode::AbsX | AddrMode::AbsY | AddrMode::Ind => {
                    if f.offset + 2 > out.len() {
                        bail!("Fixup offset out of range for {}", f.symbol);
                    }
                    let prod = (target as u32).wrapping_mul(f.mul as u32) as u16;
                    let adj = prod.wrapping_add(f.addend as u16);
                    out[f.offset] = (adj & 0xFF) as u8;
                    out[f.offset + 1] = (adj >> 8) as u8;
                }
                AddrMode::Imm
                | AddrMode::Zp
                | AddrMode::ZpX
                | AddrMode::ZpY
                | AddrMode::IndX
                | AddrMode::IndY => {
                    if f.offset >= out.len() {
                        bail!("Fixup offset out of range for {}", f.symbol);
                    }
                    let prod = (target as u32).wrapping_mul(f.mul as u32) as u16;
                    let adj_full = prod.wrapping_add(f.addend as u16);
                    let val = match f.kind {
                        FixupKind::Plain => adj_full & 0xFF,
                        FixupKind::Low => adj_full & 0xFF,
                        FixupKind::High => (adj_full >> 8) & 0xFF,
                    } as u8;
                    out[f.offset] = val;
                }
                AddrMode::Rel => {
                    let next = f.instr_addr.wrapping_add(2) as i32;
                    let prod = (target as u32).wrapping_mul(f.mul as u32) as u16;
                    let adj = prod.wrapping_add(f.addend as u16);
                    let diff = adj as i32 - next;
                    if diff < -128 || diff > 127 {
                        bail!("Branch to {} out of range (diff {})", f.symbol, diff);
                    }
                    if f.offset >= out.len() {
                        bail!("Fixup offset out of range (rel) for {}", f.symbol);
                    }
                    out[f.offset] = diff as i8 as u8;
                }
                _ => {}
            }
        }
        Ok((out, sym))
    }
}

fn ensure_capacity(v: &mut Vec<u8>, size: usize) {
    if v.len() < size {
        v.resize(size, 0);
    }
}

fn parse_number(s: &str) -> Result<u32> {
    let st = s.trim();
    if let Some(hex) = st.strip_prefix('$') {
        return Ok(u32::from_str_radix(hex, 16)?);
    }
    if st.starts_with("0x") {
        return Ok(u32::from_str_radix(&st[2..], 16)?);
    }
    if st.chars().all(|c| c.is_ascii_digit()) {
        return Ok(st.parse()?);
    }
    if st.contains('-') {
        // treat as expression handled later, return dummy so caller may still proceed
        return Ok(0);
    }
    // crude: strip leading/trailing angle brackets from macro remnants
    let trimmed = st.trim_matches('<').trim_matches('>').trim();
    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Ok(trimmed.parse()?);
    }
    anyhow::bail!("Unsupported number '{}'", s)
}

// Define or re-define a symbol; allow updating placeholder (0) or identical value; error on differing value.
fn define_symbol(sym: &mut HashMap<String, u16>, name: &str, value: u16) -> Result<()> {
    if let Some(prev) = sym.get(name) {
        // If already defined to a non-zero concrete value, keep the first definition.
        if *prev != 0 {
            return Ok(());
        }
    }
    sym.insert(name.to_string(), value); // insert or update placeholder 0 with real value
    Ok(())
}

fn infer_mode(mnem: &str, operand: Option<&str>, sym: &HashMap<String, u16>) -> AddrMode {
    use AddrMode::*;
    if operand.is_none() {
        // Accumulator form for shifts/rotates if mnemonic implies and no operand given
        let m = mnem.to_ascii_uppercase();
        if matches!(m.as_str(), "ASL" | "LSR" | "ROL" | "ROR") {
            return Acc;
        }
        return Imp;
    }
    let op = operand.unwrap().trim();
    // Check for explicit "A" operand for accumulator mode
    if op.eq_ignore_ascii_case("a") {
        let m = mnem.to_ascii_uppercase();
        if matches!(m.as_str(), "ASL" | "LSR" | "ROL" | "ROR") {
            return Acc;
        }
        // For other instructions, "A" shouldn't be a valid operand, but fall through
    }
    if op.starts_with('#') {
        return Imm;
    }
    if op.starts_with('(') {
        if op.ends_with(",X)") {
            return IndX;
        }
        if op.ends_with(",Y)") {
            return IndY;
        }
        return Ind;
    }
    // Branch mnemonics use relative
    let upper = mnem.to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "BPL" | "BMI" | "BVC" | "BVS" | "BCC" | "BCS" | "BNE" | "BEQ"
    ) {
        return Rel;
    }
    // Indexed
    if op.ends_with(",X") {
        // STY doesn't support AbsX on 6502, only ZpX
        if mnem.to_ascii_uppercase() == "STY" {
            return ZpX;
        }
        return if is_zeropage(op.trim_end_matches(",X"), sym) {
            ZpX
        } else {
            AbsX
        };
    }
    if op.ends_with(",Y") {
        let base = op.trim_end_matches(",Y");
        if is_zeropage(base, sym) {
            let up = mnem.to_ascii_uppercase();
            if matches!(up.as_str(), "LDX" | "STX") {
                return ZpY;
            } else {
                return AbsY;
            }
        } else {
            return AbsY;
        }
    }
    // Plain zp vs abs
    let mut mode = if is_zeropage(op, sym) { Zp } else { Abs };
    // Force valid modes for JSR/JMP (no zero-page variant)
    if upper == "JSR" || upper == "JMP" {
        if matches!(mode, Zp | ZpX | ZpY) {
            mode = Abs;
        }
    }
    match mode {
        ZpY => {
            let m = upper.as_str();
            if !matches!(m, "LDX" | "STX") {
                mode = AbsY;
            }
        }
        ZpX => {
            let m = upper.as_str();
            if matches!(m, "CPX" | "CPY") {
                mode = AbsX;
            }
        }
        _ => {}
    }
    // Downgrade illegal indexed store forms (STY AbsX -> Abs, STX AbsY -> Abs)
    if upper == "STY" && matches!(mode, AbsX) {
        mode = Abs;
    }
    if upper == "STX" && matches!(mode, AbsY) {
        mode = Abs;
    }
    mode
}

fn is_zeropage(token: &str, sym: &HashMap<String, u16>) -> bool {
    let t = token.trim().trim_start_matches('$');
    if t.is_empty() {
        return false;
    }
    if let Ok(v) = u16::from_str_radix(t, 16) {
        return v < 0x100;
    }
    if let Ok(v) = parse_number(token) {
        return v < 0x100;
    }
    if let Some(v) = sym.get(t) {
        return *v < 0x100;
    }
    false
}

fn build_operand_bytes(
    mode: AddrMode,
    oprnd: &str,
    sym: &HashMap<String, u16>,
    pc: u16,
    opcode_offset: usize,
    out: &mut Vec<u8>,
    fixups: &mut Vec<Fixup>,
    mnem: &str,
) -> Result<()> {
    use AddrMode::*;
    let mut raw_str = oprnd.trim().to_string();
    while raw_str.ends_with(',') {
        raw_str.pop();
    }
    let raw = raw_str.as_str();
    match mode {
        Imp | Acc => {}
        Imm => {
            let tok = raw.trim_start_matches('#').trim();
            if (tok.starts_with('<') || tok.starts_with('>')) && tok.len() > 1 {
                let hi = tok.starts_with('>');
                let inner = &tok[1..];
                if let Some((sym_name, mul, addend, kind_expr)) = parse_add_mul_expr(tok) {
                    // full expression with markers handled
                    fixups.push(Fixup {
                        offset: opcode_offset + 1,
                        symbol: sym_name.to_string(),
                        mode,
                        instr_addr: pc,
                        addend,
                        mul,
                        kind: match kind_expr {
                            FixupKind::Plain => {
                                if hi {
                                    FixupKind::High
                                } else {
                                    FixupKind::Low
                                }
                            }
                            FixupKind::Low | FixupKind::High => kind_expr,
                        },
                    });
                } else if let Some((sym_name, addend)) = split_symbol_addend(inner) {
                    fixups.push(Fixup {
                        offset: opcode_offset + 1,
                        symbol: sym_name.to_string(),
                        mode,
                        instr_addr: pc,
                        addend,
                        mul: 1,
                        kind: if hi { FixupKind::High } else { FixupKind::Low },
                    });
                } else if is_symbol(inner) {
                    fixups.push(Fixup {
                        offset: opcode_offset + 1,
                        symbol: inner.to_string(),
                        mode,
                        instr_addr: pc,
                        addend: 0,
                        mul: 1,
                        kind: if hi { FixupKind::High } else { FixupKind::Low },
                    });
                } else {
                    bail!("Unknown immediate {}", tok);
                }
            } else if tok.len() == 3 && tok.starts_with('"') && tok.ends_with('"') {
                out[opcode_offset + 1] = tok.as_bytes()[1] & 0x7F;
            } else if tok.len() == 1 {
                // bare single char like : ; , etc.
                out[opcode_offset + 1] = tok.as_bytes()[0] & 0x7F;
            } else if let Ok(v) = parse_number(tok) {
                out[opcode_offset + 1] = (v & 0xFF) as u8;
            } else if let Some((sym_name, mul, addend, kind_expr)) = parse_add_mul_expr(tok) {
                fixups.push(Fixup {
                    offset: opcode_offset + 1,
                    symbol: sym_name.to_string(),
                    mode,
                    instr_addr: pc,
                    addend,
                    mul,
                    kind: kind_expr,
                });
            } else if is_symbol(tok) {
                fixups.push(Fixup {
                    offset: opcode_offset + 1,
                    symbol: tok.to_string(),
                    mode,
                    instr_addr: pc,
                    addend: 0,
                    mul: 1,
                    kind: FixupKind::Plain,
                });
            } else {
                bail!("Unknown immediate {}", tok);
            }
        }
        Zp | ZpX | ZpY | IndX | IndY => {
            let base = if raw.ends_with(",X") {
                raw.trim_end_matches(",X").trim()
            } else if raw.ends_with(",Y") {
                raw.trim_end_matches(",Y").trim()
            } else if raw.starts_with('(') && raw.ends_with(",X)") {
                let inner = &raw[1..raw.len() - 1];
                if let Some(p) = inner.rfind(',') {
                    inner[..p].trim()
                } else {
                    inner.trim()
                }
            } else if raw.starts_with('(') && raw.contains("),Y") {
                let mut t = raw.trim();
                if t.ends_with(",Y") {
                    t = t.trim_end_matches(",Y");
                }
                t = t.trim_end_matches(')');
                t = t.trim_start_matches('(');
                t.trim()
            } else {
                raw
            };
            if let Some((sym_name, mul_expr, addend_expr, kind_expr)) = parse_add_mul_expr(base) {
                if !matches!(kind_expr, FixupKind::Plain) { /* low/high extraction invalid for zp, treat as plain low byte */
                }
                if let Some(val) = sym.get(sym_name) {
                    let v = ((val.wrapping_mul(mul_expr as u16)).wrapping_add(addend_expr as u16))
                        & 0xFFFF;
                    out[opcode_offset + 1] = (v & 0xFF) as u8;
                } else if is_symbol(sym_name) {
                    fixups.push(Fixup {
                        offset: opcode_offset + 1,
                        symbol: sym_name.to_string(),
                        mode,
                        instr_addr: pc,
                        addend: addend_expr,
                        mul: mul_expr,
                        kind: FixupKind::Plain,
                    });
                } else {
                    bail!("Unknown operand {} for {}", raw, mnem);
                }
                return Ok(());
            }
            if let Some((sym_name, addend, mul)) = parse_mul_add(base) {
                if let Some(val) = sym.get(sym_name) {
                    let v = ((val.wrapping_mul(mul as u16)).wrapping_add(addend as u16)) & 0xFFFF;
                    out[opcode_offset + 1] = (v & 0xFF) as u8;
                } else if is_symbol(sym_name) {
                    fixups.push(Fixup {
                        offset: opcode_offset + 1,
                        symbol: sym_name.to_string(),
                        mode,
                        instr_addr: pc,
                        addend,
                        mul,
                        kind: FixupKind::Plain,
                    });
                } else {
                    bail!("Unknown operand {} for {}", raw, mnem);
                }
                return Ok(());
            }
            // Multi-term constant + symbol chains (e.g. 1+2+LABEL)
            if let Some((sym_name, addend)) = parse_symbol_add_chain(base) {
                if let Some(val) = sym.get(sym_name) {
                    let v = val.wrapping_add(addend as u16);
                    out[opcode_offset + 1] = (v & 0xFF) as u8;
                } else if is_symbol(sym_name) {
                    fixups.push(Fixup {
                        offset: opcode_offset + 1,
                        symbol: sym_name.to_string(),
                        mode,
                        instr_addr: pc,
                        addend,
                        mul: 1,
                        kind: FixupKind::Plain,
                    });
                } else {
                    bail!("Unknown operand {} for {}", raw, mnem);
                }
            }
            if let Some((sym_name, addend)) = split_symbol_addend(base) {
                if let Some(val) = sym.get(sym_name) {
                    let v = val.wrapping_add(addend as u16);
                    out[opcode_offset + 1] = (v & 0xFF) as u8;
                } else if is_symbol(sym_name) {
                    fixups.push(Fixup {
                        offset: opcode_offset + 1,
                        symbol: sym_name.to_string(),
                        mode,
                        instr_addr: pc,
                        addend,
                        mul: 1,
                        kind: FixupKind::Plain,
                    });
                } else {
                    bail!("Unknown operand {} for {}", raw, mnem);
                }
            } else if let Ok(v) = resolve_value(base, sym) {
                out[opcode_offset + 1] = (v & 0xFF) as u8;
            } else if is_symbol(base) {
                fixups.push(Fixup {
                    offset: opcode_offset + 1,
                    symbol: base.to_string(),
                    mode,
                    instr_addr: pc,
                    addend: 0,
                    mul: 1,
                    kind: FixupKind::Plain,
                });
            } else {
                bail!("Unknown operand {} for {}", raw, mnem);
            }
        }
        Abs | AbsX | AbsY | Ind => {
            // Strip any ,X or ,Y suffix (even if mode mis-inferred) before symbol lookup.
            let mut base = raw.trim();
            if base.ends_with(",X") {
                base = base.trim_end_matches(",X").trim();
            } else if base.ends_with(",Y") {
                base = base.trim_end_matches(",Y").trim();
            }
            // If there's still a comma (unexpected), take part before it.
            if let Some(pos) = base.find(',') {
                base = base[..pos].trim();
            }
            let cleaned = base.trim_matches(|c| c == '(' || c == ')');
            if let Some((sym_name, mul_expr, addend_expr, kind_expr)) = parse_add_mul_expr(cleaned)
            {
                if let Some(val) = sym.get(sym_name) {
                    let prod = (val.wrapping_mul(mul_expr as u16)).wrapping_add(addend_expr as u16);
                    let out_val = match kind_expr {
                        FixupKind::Plain => prod,
                        FixupKind::Low => prod,
                        FixupKind::High => prod >> 8,
                    };
                    out[opcode_offset + 1] = (out_val & 0xFF) as u8;
                    if !matches!(kind_expr, FixupKind::Low | FixupKind::High) {
                        out[opcode_offset + 2] = (prod >> 8) as u8;
                    }
                } else if is_symbol(sym_name) {
                    fixups.push(Fixup {
                        offset: opcode_offset + 1,
                        symbol: sym_name.to_string(),
                        mode,
                        instr_addr: pc,
                        addend: addend_expr,
                        mul: mul_expr,
                        kind: kind_expr,
                    });
                } else {
                    bail!("Unknown operand {} for {}", raw, mnem);
                }
            } else if let Some((sym_name, addend, mul)) = parse_mul_add(cleaned) {
                if let Some(val) = sym.get(sym_name) {
                    let v = ((val.wrapping_mul(mul as u16)).wrapping_add(addend as u16)) & 0xFFFF;
                    out[opcode_offset + 1] = (v & 0xFF) as u8;
                    out[opcode_offset + 2] = (v >> 8) as u8;
                } else if is_symbol(sym_name) {
                    fixups.push(Fixup {
                        offset: opcode_offset + 1,
                        symbol: sym_name.to_string(),
                        mode,
                        instr_addr: pc,
                        addend,
                        mul,
                        kind: FixupKind::Plain,
                    });
                } else {
                    bail!("Unknown operand {} for {}", raw, mnem);
                }
            } else if let Some((sym_name, addend)) =
                parse_symbol_add_chain(cleaned).or_else(|| split_symbol_addend(cleaned))
            {
                if let Some(val) = sym.get(sym_name) {
                    let v = val.wrapping_add(addend as u16);
                    out[opcode_offset + 1] = (v & 0xFF) as u8;
                    out[opcode_offset + 2] = (v >> 8) as u8;
                } else if is_symbol(sym_name) {
                    fixups.push(Fixup {
                        offset: opcode_offset + 1,
                        symbol: sym_name.to_string(),
                        mode,
                        instr_addr: pc,
                        addend,
                        mul: 1,
                        kind: FixupKind::Plain,
                    });
                } else {
                    bail!("Unknown operand {} for {}", raw, mnem);
                }
            } else if let Ok(v) = resolve_value(cleaned, sym) {
                out[opcode_offset + 1] = (v & 0xFF) as u8;
                out[opcode_offset + 2] = (v >> 8) as u8;
            } else if let Ok(v) = eval_multi_symbol_expr(cleaned, sym) {
                // Try evaluating multi-symbol expressions like "FOUTBL+2+ADDPRC"
                out[opcode_offset + 1] = (v & 0xFF) as u8;
                out[opcode_offset + 2] = (v >> 8) as u8;
            } else if is_symbol(cleaned) {
                fixups.push(Fixup {
                    offset: opcode_offset + 1,
                    symbol: cleaned.to_string(),
                    mode,
                    instr_addr: pc,
                    addend: 0,
                    mul: 1,
                    kind: FixupKind::Plain,
                });
            } else {
                bail!("Unknown operand {} for {}", raw, mnem);
            }
        }
        Rel => {
            if raw.trim() == "*" {
                out[opcode_offset + 1] = 0;
            } else if let Ok(v) = resolve_value(raw, sym) {
                let next_pc = pc as i32 + 2;
                let diff = v as i32 - next_pc;
                if diff < -128 || diff > 127 {
                    bail!("Branch out of range {}", raw);
                }
                out[opcode_offset + 1] = diff as i8 as u8;
            } else if is_symbol(raw) {
                fixups.push(Fixup {
                    offset: opcode_offset + 1,
                    symbol: raw.to_string(),
                    mode,
                    instr_addr: pc,
                    addend: 0,
                    mul: 1,
                    kind: FixupKind::Plain,
                });
            } else {
                bail!("Unknown branch target {}", raw);
            }
        }
    }
    Ok(())
}

fn is_symbol(s: &str) -> bool {
    let st = s.trim();
    !st.is_empty()
        && st.chars().next().unwrap().is_ascii_alphabetic()
        && st
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '$')
}

fn split_symbol_addend(s: &str) -> Option<(&str, i16)> {
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'+' || b == b'-' {
            let (lhs, rhs) = s.split_at(i);
            if !is_symbol(lhs) {
                return None;
            }
            let sign = if b == b'+' { 1 } else { -1 };
            let num_str = rhs[1..].trim();
            if num_str.is_empty() {
                return None;
            }
            if let Ok(val) = if num_str.starts_with('$') {
                u16::from_str_radix(&num_str[1..], 16).map(|v| v as i32)
            } else {
                num_str.parse::<i32>()
            } {
                if val >= -32768 && val <= 32767 {
                    return Some((lhs, (sign * val) as i16));
                }
            }
            return None;
        }
    }
    None
}

// Parse additive chain like 1+2+SYMBOL or SYMBOL+1+2 returning ("SYMBOL", total_addend)
// Only supports '+' between positive decimal/hex constants and exactly one symbol.
fn parse_symbol_add_chain(s: &str) -> Option<(&str, i16)> {
    if s.find('+').is_none() {
        return None;
    }
    let mut symbol: Option<&str> = None;
    let mut total: i32 = 0;
    for part in s.split('+').map(|p| p.trim()) {
        if part.is_empty() {
            return None;
        }
        if is_symbol(part) {
            if symbol.is_some() {
                return None;
            } // more than one symbol
            symbol = Some(part);
        } else {
            let val_res: Result<i32, _> = if part.starts_with('$') {
                i32::from_str_radix(&part[1..], 16)
            } else {
                part.parse()
            };
            if let Ok(v) = val_res {
                total = total.wrapping_add(v);
            } else {
                return None;
            }
        }
    }
    if let Some(sym_name) = symbol {
        if total >= -32768 && total <= 32767 {
            return Some((sym_name, total as i16));
        }
    }
    None
}

// Parse simple multiply+add chain: A*SYMBOL + c1 + c2 ... or SYMBOL*A + consts
fn parse_mul_add(s: &str) -> Option<(&str, i16, i16)> {
    // Split on '+' first to gather additive constants after a possible multiplied symbol term
    let pieces = s.split('+').map(|p| p.trim()).collect::<Vec<_>>();
    if pieces.is_empty() {
        return None;
    }
    let mut symbol: Option<&str> = None;
    let mut mul: i16 = 1;
    let mut add: i32 = 0;
    for p in pieces.iter() {
        if let Some(star) = p.find('*') {
            let (lhs, rhs_with) = p.split_at(star);
            let rhs = &rhs_with[1..];
            let l = lhs.trim();
            let r = rhs.trim();
            if is_symbol(l) && !r.is_empty() && !is_symbol(r) {
                if symbol.is_some() {
                    return None;
                }
                symbol = Some(l);
                mul = parse_small_int(r)?;
            } else if is_symbol(r) && !l.is_empty() && !is_symbol(l) {
                if symbol.is_some() {
                    return None;
                }
                symbol = Some(r);
                mul = parse_small_int(l)?;
            } else {
                return None;
            }
        } else if is_symbol(p) {
            if symbol.is_some() {
                return None;
            }
            symbol = Some(*p);
        } else {
            // numeric constant
            add = add.wrapping_add(parse_small_int(p)? as i32);
        }
    }
    if let Some(sym) = symbol {
        if add >= -32768 && add <= 32767 {
            return Some((sym, add as i16, mul));
        }
    }
    None
}

fn parse_small_int(s: &str) -> Option<i16> {
    if s.is_empty() {
        return None;
    }
    if s.starts_with('$') {
        i16::from_str_radix(&s[1..], 16).ok()
    } else {
        s.parse::<i16>().ok()
    }
}

// Parse expressions like 257+13+<2*ADDPRC or 12+>WORDPTR or 3*TABLE+5
// Returns (symbol, mul, addend, kind)
fn parse_add_mul_expr(s: &str) -> Option<(&str, i16, i16, FixupKind)> {
    if !(s.contains('+') || s.contains('*') || s.starts_with('<') || s.starts_with('>')) {
        return None;
    }
    let mut add: i32 = 0;
    let mut symbol: Option<&str> = None;
    let mut mul: i16 = 1;
    let mut kind = FixupKind::Plain;
    for raw_part in s.split('+') {
        let mut part = raw_part.trim();
        // Strip balanced outer parentheses repeatedly
        loop {
            if part.len() >= 2 && part.starts_with('(') && part.ends_with(')') {
                let inner = &part[1..part.len() - 1];
                // Avoid stripping if parentheses are unmatched inside
                if inner.contains('(') && inner.contains(')') { /* heuristic: still strip */ }
                part = inner.trim();
            } else {
                break;
            }
        }
        if part.is_empty() {
            return None;
        }
        let mut local_kind = FixupKind::Plain;
        if part.starts_with('<') {
            local_kind = FixupKind::Low;
            part = &part[1..];
        } else if part.starts_with('>') {
            local_kind = FixupKind::High;
            part = &part[1..];
        }
        // If marker used with closing '>' as in <EXPR> strip trailing '>' now
        if matches!(local_kind, FixupKind::Low | FixupKind::High) && part.ends_with('>') {
            part = &part[..part.len() - 1];
        }
        if local_kind != FixupKind::Plain {
            if kind != FixupKind::Plain && kind != local_kind {
                return None;
            }
            kind = local_kind;
        }
        if part.contains('*') {
            let star = part.find('*')?;
            let (lhs, rhs_with) = part.split_at(star);
            let rhs = &rhs_with[1..];
            let l = lhs.trim();
            let r = rhs.trim();
            let (maybe_sym, maybe_num) = if is_symbol(l) && !r.is_empty() && !is_symbol(r) {
                (l, r)
            } else if is_symbol(r) && !l.is_empty() && !is_symbol(l) {
                (r, l)
            } else {
                return None;
            };
            if symbol.is_some() && symbol.unwrap() != maybe_sym {
                return None;
            }
            symbol = Some(maybe_sym);
            let m = parse_small_int(maybe_num)?;
            if m == 0 {
                return None;
            }
            mul = mul.saturating_mul(m);
        } else if is_symbol(part) {
            if symbol.is_some() && symbol.unwrap() != part {
                return None;
            }
            symbol = Some(part);
        } else {
            // numeric constant
            if let Some(v) = parse_small_int(part) {
                add = add.wrapping_add(v as i32);
            } else {
                return None;
            }
        }
    }
    if let Some(sym) = symbol {
        if add >= -32768 && add <= 32767 {
            return Some((sym, mul, add as i16, kind));
        }
    }
    None
}

fn resolve_value(token: &str, sym: &HashMap<String, u16>) -> Result<u16> {
    let t = token.trim();
    // strip addressing decorations
    let t = t.trim_start_matches('#').trim();
    if t == "*" {
        return Ok(0);
    } // placeholder, actual PC should be handled contextually (branches handled earlier)
    if let Some(idx) = t.find('-') {
        let (lhs, rhs_with) = t.split_at(idx);
        let rhs = &rhs_with[1..];
        if let Some(base) = sym.get(lhs) {
            if rhs == ":" {
                return Ok(base.wrapping_sub(':' as u16));
            }
            if rhs.starts_with('"') && rhs.ends_with('"') && rhs.len() == 3 {
                return Ok(base.wrapping_sub(rhs.as_bytes()[1] as u16));
            }
        }
    }
    if t.starts_with('$') {
        return Ok(u16::from_str_radix(&t[1..], 16)?);
    }
    if let Ok(v) = parse_number(t) {
        return Ok(v as u16);
    }
    let key = t.trim();
    if let Some(v) = sym.get(key) {
        return Ok(*v);
    }
    bail!("Unknown symbol '{}'", token)
}

/// Evaluate complex multi-symbol expressions like "FOUTBL+2+ADDPRC"
/// Returns the computed value if all symbols are resolved
fn eval_multi_symbol_expr(expr: &str, sym: &HashMap<String, u16>) -> Result<u16> {
    use std::collections::VecDeque;
    
    let expr = expr.trim();
    
    // First pass: substitute all symbols with their numeric values
    let mut processed = String::new();
    let mut current_symbol = String::new();
    let mut in_hex = false;
    
    for ch in expr.chars() {
        if ch == '$' {
            if !current_symbol.is_empty() {
                // Flush any pending symbol
                if let Some(&val) = sym.get(&current_symbol) {
                    processed.push_str(&val.to_string());
                } else {
                    bail!("Unknown symbol '{}' in expression '{}'", current_symbol, expr);
                }
                current_symbol.clear();
            }
            processed.push(ch);
            in_hex = true;
        } else if ch.is_alphanumeric() || ch == '_' || ch == '.' {
            if in_hex {
                processed.push(ch);
            } else {
                current_symbol.push(ch);
            }
        } else {
            in_hex = false;
            if !current_symbol.is_empty() {
                // Check if it's a symbol or a number
                if let Ok(num) = current_symbol.parse::<i64>() {
                    processed.push_str(&num.to_string());
                } else if let Some(&val) = sym.get(&current_symbol) {
                    processed.push_str(&val.to_string());
                } else {
                    bail!("Unknown symbol '{}' in expression '{}'", current_symbol, expr);
                }
                current_symbol.clear();
            }
            processed.push(ch);
        }
    }
    
    // Flush any remaining symbol
    if !current_symbol.is_empty() {
        if let Ok(num) = current_symbol.parse::<i64>() {
            processed.push_str(&num.to_string());
        } else if let Some(&val) = sym.get(&current_symbol) {
            processed.push_str(&val.to_string());
        } else {
            bail!("Unknown symbol '{}' in expression '{}'", current_symbol, expr);
        }
    }
    
    // Second pass: evaluate arithmetic expression with operator precedence
    // Convert hex numbers to decimal
    let processed = {
        let mut result = String::new();
        let mut chars = processed.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '$' {
                let mut hex = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_ascii_hexdigit() {
                        hex.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if let Ok(val) = i64::from_str_radix(&hex, 16) {
                    result.push_str(&val.to_string());
                } else {
                    result.push('$');
                    result.push_str(&hex);
                }
            } else {
                result.push(ch);
            }
        }
        result
    };
    
    // Simple expression evaluator: handles +, -, *, / with precedence
    fn eval_expr(s: &str) -> Result<i64> {
        // Remove whitespace
        let s = s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        
        // Parse into tokens
        let mut tokens = Vec::new();
        let mut num = String::new();
        
        for ch in s.chars() {
            if ch.is_ascii_digit() {
                num.push(ch);
            } else if ch == '+' || ch == '-' || ch == '*' || ch == '/' {
                if !num.is_empty() {
                    tokens.push(num.clone());
                    num.clear();
                }
                tokens.push(ch.to_string());
            } else if ch == '(' || ch == ')' {
                if !num.is_empty() {
                    tokens.push(num.clone());
                    num.clear();
                }
                tokens.push(ch.to_string());
            }
        }
        if !num.is_empty() {
            tokens.push(num);
        }
        
        // Simple recursive descent parser
        fn parse_expr(tokens: &mut VecDeque<String>) -> Result<i64> {
            parse_add_sub(tokens)
        }
        
        fn parse_add_sub(tokens: &mut VecDeque<String>) -> Result<i64> {
            let mut left = parse_mul_div(tokens)?;
            while !tokens.is_empty() {
                let op = tokens.front().map(|s| s.as_str());
                if op == Some("+") {
                    tokens.pop_front();
                    let right = parse_mul_div(tokens)?;
                    left = left.wrapping_add(right);
                } else if op == Some("-") {
                    tokens.pop_front();
                    let right = parse_mul_div(tokens)?;
                    left = left.wrapping_sub(right);
                } else {
                    break;
                }
            }
            Ok(left)
        }
        
        fn parse_mul_div(tokens: &mut VecDeque<String>) -> Result<i64> {
            let mut left = parse_primary(tokens)?;
            while !tokens.is_empty() {
                let op = tokens.front().map(|s| s.as_str());
                if op == Some("*") {
                    tokens.pop_front();
                    let right = parse_primary(tokens)?;
                    left = left.wrapping_mul(right);
                } else if op == Some("/") {
                    tokens.pop_front();
                    let right = parse_primary(tokens)?;
                    if right == 0 {
                        bail!("Division by zero");
                    }
                    left = left / right;
                } else {
                    break;
                }
            }
            Ok(left)
        }
        
        fn parse_primary(tokens: &mut VecDeque<String>) -> Result<i64> {
            if tokens.is_empty() {
                bail!("Unexpected end of expression");
            }
            
            let token = tokens.pop_front().unwrap();
            if token == "(" {
                let result = parse_expr(tokens)?;
                if tokens.is_empty() || tokens.pop_front().unwrap() != ")" {
                    bail!("Missing closing parenthesis");
                }
                Ok(result)
            } else if let Ok(num) = token.parse::<i64>() {
                Ok(num)
            } else {
                bail!("Unexpected token: {}", token);
            }
        }
        
        let mut token_queue: VecDeque<String> = tokens.into_iter().collect();
        parse_expr(&mut token_queue)
    }
    
    let result = eval_expr(&processed)?;
    Ok((result & 0xFFFF) as u16)
}
