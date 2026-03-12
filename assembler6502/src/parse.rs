use regex::Regex;
use lazy_static::lazy_static;
use crate::opcode::AddrMode;

#[derive(Debug,Clone)]
pub enum Line {
    Empty,
    Comment(String),
    Equ {label:String, expr:String},
    Org(u16),
    Label(String),
    Instr { label:Option<String>, mnem:String, operand:Option<String>},
    DataBytes { label:Option<String>, bytes:Vec<u8>},
    DataWord { label:Option<String>, value:u16},
    Block { label:Option<String>, size:usize},
}

lazy_static!{
    // Match label with one or more trailing ':' (allow double-colon) without using a word boundary that broke cases like 'LABEL: MNEM'.
    static ref RE_LABEL:Regex=Regex::new(r"^(?P<label>[A-Za-z_.$][\w.$]*):+").unwrap();
    static ref RE_EQU:Regex=Regex::new(r"^(?P<name>[A-Za-z_.$][\w.$]*)\s*=\s*(?P<expr>.+)").unwrap();
    static ref RE_ORG_DOT:Regex=Regex::new(r"^\.org\s+\$?([0-9A-Fa-f]+)").unwrap();
    static ref RE_ORG_PLAIN:Regex=Regex::new(r"^ORG\s+([0-9]+|\$[0-9A-Fa-f]+)").unwrap();
}

pub fn lex_line(raw:&str)->Line{
    let s=raw.trim();
    if s.is_empty(){ return Line::Empty; }
    if s.starts_with(';'){ return Line::Comment(raw.to_string()); }
    if let Some(cap)=RE_ORG_DOT.captures(s){
        let v=u16::from_str_radix(&cap[1],16).unwrap_or(0);
        return Line::Org(v);
    }
    if let Some(cap)=RE_ORG_PLAIN.captures(s){
        let val=&cap[1];
        let v= if let Some(hex)=val.strip_prefix('$'){ u16::from_str_radix(hex,16).unwrap_or(0)} else { val.parse().unwrap_or(0)};
        return Line::Org(v);
    }
    if let Some(cap)=RE_EQU.captures(s){
        return Line::Equ{label:cap["name"].to_string(), expr:cap["expr"].trim().to_string()};
    }
    // Separate optional label
    let (label, rest_full) = if let Some(cap)=RE_LABEL.captures(s){
        (Some(cap["label"].to_string()), s[cap[0].len()..].trim_start())
    } else { (None,s) };
    // split off comment
    let mut parts_comment = rest_full.splitn(2,';');
    let rest = parts_comment.next().unwrap().trim();
    if rest.is_empty(){ if let Some(l)=label { return Line::Label(l); } else { return Line::Empty; } }
    // data directives
    if rest.to_ascii_lowercase().starts_with(".byte "){
        let list=&rest[6..];
        let mut bytes=Vec::new();
        for tok in list.split(','){ let t=tok.trim(); if t.is_empty(){continue;} if let Some(b)=parse_byte_token(t){ bytes.push(b);} }
        return Line::DataBytes{ label, bytes };
    }
    if rest.to_ascii_lowercase().starts_with(".word "){
        let vtok=rest[6..].trim();
        if let Some(val)=parse_word_token(vtok){ return Line::DataWord{ label, value:val }; }
    }
    if rest.to_ascii_lowercase().starts_with(".res "){
        let size_tok=rest[5..].trim();
        if let Ok(sz)=size_tok.parse(){ return Line::Block{ label, size:sz }; }
    }
    // crude split
    let mut parts=rest.split_whitespace();
    if let Some(mnem)=parts.next(){
        // If token ends with ':' and no operand, treat as label-only (defensive)
        if mnem.ends_with(':') && parts.clone().next().is_none(){
            let lbl = mnem.trim_end_matches(':').to_string();
            return Line::Label(lbl);
        }
        let upper=mnem.to_ascii_uppercase();
        if matches!(upper.as_str(), "IFN"|"IFE"|"IF1"|"IF2"|"IFNDEF") {
            return Line::Empty; // preprocessor should have removed; guard against stragglers
        }
        let operand=parts.collect::<Vec<_>>().join(" ");
        let operand_opt=if operand.is_empty(){None}else{Some(operand)};
        return Line::Instr{label, mnem:mnem.to_string(), operand:operand_opt};
    }
    Line::Comment(raw.to_string())
}

fn parse_byte_token(s:&str)->Option<u8>{
    let st=s.trim();
    if let Some(hex)=st.strip_prefix('$'){ u8::from_str_radix(hex,16).ok() }
    else if st.chars().all(|c| c.is_ascii_digit()){ st.parse().ok() }
    else { None }
}
fn parse_word_token(s:&str)->Option<u16>{
    let st=s.trim();
    if let Some(hex)=st.strip_prefix('$'){ u16::from_str_radix(hex,16).ok() }
    else if st.chars().all(|c| c.is_ascii_digit()){ st.parse().ok() }
    else { None }
}

pub fn classify_addr_mode(op:&str)->Option<AddrMode>{
    let op=op.trim();
    use AddrMode::*;
    if op.starts_with('#'){ return Some(Imm); }
    if op.starts_with('(') && op.ends_with(",X)") { return Some(IndX); }
    if op.starts_with('(') && op.ends_with(")") && op.contains(','){ /* (addr),Y */
        if op.ends_with(",Y)") { return Some(IndY); }
    }
    if op.starts_with('(') && op.ends_with(")") { return Some(Ind); }
    if op.ends_with(",X") { return Some(AbsX); }
    if op.ends_with(",Y") { return Some(AbsY); }
    if op.len()<=3 { return Some(AddrMode::Zp); }
    Some(Abs)
}
