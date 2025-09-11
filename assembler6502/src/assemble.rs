use crate::parse::{lex_line, Line};
use crate::opcode::{lookup, AddrMode, size_for};
use crate::preprocess;
use anyhow::{Result,bail};
use std::collections::HashMap;

#[derive(Debug,Default,Clone)]
pub struct AsmOptions{ pub start_org: Option<u16> }

pub struct Assembler{ pub opts:AsmOptions }

impl Assembler{
    pub fn new(opts:AsmOptions)->Self{ Self{opts} }
    pub fn assemble(&self, source:&str)->Result<(Vec<u8>,HashMap<String,u16>)>{
        // Preprocess first
        let pre = preprocess::run(source);
        let mut lines_parsed:Vec<Line>=Vec::new();
        for raw in pre.lines.iter(){ lines_parsed.push(lex_line(raw)); }
        let mut sym:HashMap<String,u16>=HashMap::new();
        // Seed symbols from preprocessor (equates) where they resolved
        for (k,v) in pre.symbols.iter(){ if *v >=0 && *v <= 0xFFFF { sym.insert(k.to_string(), *v as u16); } }
        let mut pc:u16=self.opts.start_org.unwrap_or(0);
        // Pass 1: assign addresses using inferred sizes
        for l in &lines_parsed{
            match l{
                Line::Org(v)=>{ pc=*v; },
                Line::Equ{label,expr:_}=>{ sym.entry(label.clone()).or_insert(0); },
                Line::Label(name)=>{ sym.insert(name.clone(), pc); },
                Line::Instr{label,mnem,operand}=>{
                    if let Some(lab)=label{ sym.insert(lab.clone(), pc);} 
                    let mode = infer_mode(mnem, operand.as_deref(), &sym);
                    pc=pc.wrapping_add(size_for(mode) as u16);
                },
                Line::DataBytes{label,bytes}=>{
                    if let Some(lab)=label{ sym.insert(lab.clone(), pc);} pc=pc.wrapping_add(bytes.len() as u16);
                },
                Line::DataWord{label,value:_}=>{ if let Some(lab)=label{ sym.insert(lab.clone(), pc);} pc=pc.wrapping_add(2); },
                Line::Block{label,size}=>{ if let Some(lab)=label{ sym.insert(lab.clone(), pc);} pc=pc.wrapping_add(*size as u16); },
                _=>{}
            }
        }
        // Pass 2: encode
        pc = self.opts.start_org.unwrap_or(0);
        let mut out:Vec<u8>=Vec::new();
        for l in &lines_parsed{
            match l{
                Line::Org(v)=>{ pc=*v; while out.len()<pc as usize { out.push(0);} },
                Line::Label(_)=>{},
                Line::Equ{label,expr}=>{
                    if let Ok(v)=parse_number(expr){ sym.insert(label.clone(), v as u16);} // TODO full expression evaluator
                },
                Line::DataBytes{label:_,bytes}=>{ ensure_capacity(&mut out, pc as usize); out.extend_from_slice(bytes); pc=pc.wrapping_add(bytes.len() as u16); },
                Line::DataWord{label:_,value}=>{ ensure_capacity(&mut out, pc as usize); out.push((value & 0xFF) as u8); out.push((value>>8) as u8); pc=pc.wrapping_add(2); },
                Line::Block{label:_,size}=>{ ensure_capacity(&mut out, (pc + *size as u16) as usize); for _ in 0..*size { out.push(0);} pc=pc.wrapping_add(*size as u16); },
                Line::Instr{label:_, mnem, operand}=>{
                    let mode = infer_mode(mnem, operand.as_deref(), &sym);
                    let mut data:Vec<u8>=Vec::new();
                    if let Some(oprnd)=operand{
                        build_operand_bytes(mode, oprnd, &sym, pc, &mut data)?;
                    }
                    if let Some(op)=lookup(mnem, mode){
                        ensure_capacity(&mut out, pc as usize);
                        out.push(op); out.extend_from_slice(&data); pc=pc.wrapping_add(size_for(mode) as u16);
                    } else { bail!("Opcode not found for {} {:?}", mnem, mode); }
                },
                _=>{}
            }
        }
        Ok((out,sym))
    }
}

fn ensure_capacity(v:&mut Vec<u8>, size:usize){ if v.len()<size { v.resize(size,0);} }

fn parse_number(s:&str)->Result<u32>{
    let st=s.trim();
    if let Some(hex)=st.strip_prefix('$'){ return Ok(u32::from_str_radix(hex,16)?); }
    if st.starts_with("0x"){ return Ok(u32::from_str_radix(&st[2..],16)?); }
    if st.chars().all(|c| c.is_ascii_digit()){ return Ok(st.parse()?); }
    // crude: strip leading/trailing angle brackets from macro remnants
    let trimmed=st.trim_matches('<').trim_matches('>').trim();
    if trimmed.chars().all(|c| c.is_ascii_digit()){ return Ok(trimmed.parse()?); }
    anyhow::bail!("Unsupported number '{}'", s)
}

fn infer_mode(mnem:&str, operand:Option<&str>, sym:&HashMap<String,u16>)->AddrMode{
    use AddrMode::*;
    if operand.is_none(){
        // Accumulator form for shifts/rotates if mnemonic implies and no operand given
        let m=mnem.to_ascii_uppercase();
        if matches!(m.as_str(), "ASL"|"LSR"|"ROL"|"ROR") { return Acc; }
        return Imp;
    }
    let op=operand.unwrap().trim();
    if op.starts_with('#'){ return Imm; }
    if op.starts_with('('){
        if op.ends_with(",X)") { return IndX; }
        if op.ends_with(",Y)") { return IndY; }
        return Ind;
    }
    // Branch mnemonics use relative
    let upper=mnem.to_ascii_uppercase();
    if matches!(upper.as_str(), "BPL"|"BMI"|"BVC"|"BVS"|"BCC"|"BCS"|"BNE"|"BEQ") { return Rel; }
    // Indexed
    if op.ends_with(",X") { return if is_zeropage(op.trim_end_matches(",X"), sym) { ZpX } else { AbsX }; }
    if op.ends_with(",Y") { return if is_zeropage(op.trim_end_matches(",Y"), sym) { ZpY } else { AbsY }; }
    // Plain zp vs abs
    if is_zeropage(op, sym){ Zp } else { Abs }
}

fn is_zeropage(token:&str, sym:&HashMap<String,u16>)->bool{
    let t=token.trim().trim_start_matches('$');
    if t.is_empty(){ return false; }
    if let Ok(v)=u16::from_str_radix(t,16){ return v<0x100; }
    if let Ok(v)=parse_number(token){ return v<0x100; }
    if let Some(v)=sym.get(t){ return *v < 0x100; }
    false
}

fn build_operand_bytes(mode:AddrMode, oprnd:&str, sym:&HashMap<String,u16>, pc:u16, out:&mut Vec<u8>)->Result<()> {
    use AddrMode::*;
    let raw=oprnd.trim();
    match mode{
        Imp|Acc => {},
        Imm => { let v=parse_number(raw.trim_start_matches('#'))?; out.push((v & 0xFF) as u8); },
        Zp|ZpX|ZpY|IndX|IndY => { let v=resolve_value(raw, sym)?; out.push((v & 0xFF) as u8); },
        Abs|AbsX|AbsY|Ind => { let v=resolve_value(raw.trim_matches(|c| c=='('||c==')'), sym)?; out.push((v & 0xFF) as u8); out.push((v>>8) as u8); },
        Rel => { // branch target (support '*' as current location)
            let target = if raw.trim()=="*" { pc as i32 } else { resolve_value(raw, sym)? as i32 };
            let next_pc = pc as i32 + 2; // PC after branch
            let diff = target - next_pc;
            if diff < -128 || diff > 127 { bail!("Branch out of range to {} (diff {})", raw, diff); }
            out.push((diff as i8) as u8);
        }
    }
    Ok(())
}

fn resolve_value(token:&str, sym:&HashMap<String,u16>)->Result<u16>{
    let t=token.trim();
    // strip addressing decorations
    let t=t.trim_start_matches('#').trim();
    if t=="*" { return Ok(0); } // placeholder, actual PC should be handled contextually (branches handled earlier)
    if t.starts_with('$'){ return Ok(u16::from_str_radix(&t[1..],16)?); }
    if let Ok(v)=parse_number(t){ return Ok(v as u16); }
    let key = t.trim();
    if let Some(v)=sym.get(key){ return Ok(*v); }
    bail!("Unknown symbol '{}'", token)
}
