#!/usr/bin/env python3
"""translate_min_ca65.py
Minimal pragmatic translator of original Microsoft 6502 BASIC source to ca65 dialect.
Covers: equate normalization, conditional inclusion (simple IFE/IFN), octal -> hex, macro elision,
pseudo immediate opcodes, BLOCK/ADR/DCI, bare numeric -> .byte/.word.
Not a full macro expander; goal is to reach a clean assembly.
"""
from __future__ import annotations
import re, sys, argparse, json, pathlib

RE_OCT = re.compile(r"\^O([0-7]+)")
RE_EQ = re.compile(r"^([A-Za-z_.$][\w.$]*)\s*==?\s*([^;]+)")
RE_IF = re.compile(r"^(IFE|IFN)\s+([^,]+),<(.*)$", re.IGNORECASE)
RE_END_BRACKET = re.compile(r">+\s*$")
RE_DEFINE = re.compile(r"^\s*DEFINE\b", re.IGNORECASE)
RE_BLOCK = re.compile(r"^([A-Za-z_.$][\w.$]*:)?\s*BLOCK\s+(\d+)\b", re.IGNORECASE)
RE_ADR = re.compile(r"^([A-Za-z_.$][\w.$]*:)?\s*ADR\(([^)]+)\)\s*>?\s*(;.*)?$", re.IGNORECASE)
RE_DCI = re.compile(r'^([A-Za-z_.$][\w.$]*:)?\s*DCI"([^"\\]+)"\s*>?\s*(;.*)?$')
RE_IMM = re.compile(r"^(\s*)([A-Za-z_.$][\w.$]*:)?\s*(LDAI|LDXI|LDYI|ADCI|SBCI|CMPI|EORI)\s+([^;]+?)(\s*;.*)?$", re.IGNORECASE)
RE_NUM_LABEL = re.compile(r"^([A-Za-z_.$][\w.$]*:)?\s*(\d+)\s*(;.*)?$")
RE_COMMENT_ONLY = re.compile(r"^\s*;.*$")
RE_LABEL_ONLY = re.compile(r"^([A-Za-z_.$][\w.$]*:)\s*$")
RE_DBL_COLON = re.compile(r"^([A-Za-z_.$][\w.$]*)::")
RE_DOLLAR_LABEL = re.compile(r"^\$([A-Za-z0-9_]+):")

IMM_MAP = { 'LDAI':'LDA', 'LDXI':'LDX', 'LDYI':'LDY', 'ADCI':'ADC', 'SBCI':'SBC', 'CMPI':'CMP', 'EORI':'EOR' }

def oct_to_hex(s:str)->str:
    return RE_OCT.sub(lambda m: f"${int(m.group(1),8):X}", s)

def simple_eval(expr:str, symbols:dict[str,int])->int|None:
    expr = oct_to_hex(expr)
    expr = expr.strip()
    # substitute known symbols with decimal
    def repl(m):
        name=m.group(0)
        return str(symbols[name]) if name in symbols else name
    expr = re.sub(r"[A-Za-z_.$][A-Za-z0-9_.$]*", repl, expr)
    # translate $HEX to decimal
    expr = re.sub(r"\$([0-9A-Fa-f]+)", lambda m: str(int(m.group(1),16)), expr)
    if not re.fullmatch(r"[0-9+\-*/() ]*", expr):
        return None
    try:
        return int(eval(expr, {"__builtins__":{}}, {}))
    except Exception:
        return None

def parse_condition_header(line:str, symbols:dict[str,int]):
    m = RE_IF.match(line)
    if not m:
        return None
    kind, expr, rest = m.groups()
    expr_clean = oct_to_hex(expr.strip())
    val = simple_eval(expr_clean, symbols)
    if val is None:
        # unknown -> include body conservatively
        return {'include': True, 'terminator_seen': False, 'kind':kind}
    include = (val == 0) if kind.upper()=='IFE' else (val != 0)
    return {'include': include, 'terminator_seen': False, 'kind':kind}

def translate(lines:list[str])->tuple[list[str],dict]:
    symbols:dict[str,int] = {}
    out:list[str] = []
    i=0
    n=len(lines)
    cond_stack:list[dict] = []
    stats={'equates':0,'conditions':0,'macros':0,'block':0,'adr':0,'dci':0,'imm':0,'numbers':0}

    while i<n:
        raw = lines[i].rstrip('\n')
        line = raw.replace('\t',' ')
        # pre-normalize some label forms
        if RE_DBL_COLON.match(line):
            line = RE_DBL_COLON.sub(lambda m: m.group(1)+':', line, count=1)
        if RE_DOLLAR_LABEL.match(line):
            line = RE_DOLLAR_LABEL.sub(lambda m: 'LBL_'+m.group(1)+':', line, count=1)
        # remove TITLE/SEARCH/SALL/RADIX/SUBTTL -> comment
        if re.match(r"^(TITLE|SEARCH|SALL|RADIX|SUBTTL)\b", line, re.IGNORECASE):
            out.append(f"; DIRECTIVE {line}")
            i+=1; continue
        # condition start
        m_if = RE_IF.match(line)
        if m_if:
            ctx = parse_condition_header(line, symbols)
            cond_stack.append(ctx)
            out.append(f"; {line}")
            stats['conditions']+=1
            i+=1; continue
        # inside conditional body? detect terminator > or >>
        if cond_stack:
            if RE_END_BRACKET.search(line):
                # strip trailing > or >>
                core = RE_END_BRACKET.sub('', line)
                if core.strip():
                    if cond_stack[-1]['include']:
                        out.append(oct_to_hex(core))
                out.append(f"; ENDIF")
                cond_stack.pop()
                i+=1; continue
            else:
                if cond_stack[-1]['include']:
                    # process included line normally (fall through)
                    pass
                else:
                    out.append(f"; EXCLUDED {line}")
                    i+=1; continue
        # macro define start -> comment until terminator
        if RE_DEFINE.match(line):
            stats['macros']+=1
            out.append(f"; {line}")
            i+=1
            # eat until line with solitary '>' or endswith > / >>
            while i<n:
                inner=lines[i].rstrip('\n')
                out.append(f"; {inner}")
                if RE_END_BRACKET.search(inner):
                    break
                i+=1
            i+=1
            continue
    # equate
        m_eq = RE_EQ.match(line)
        if m_eq:
            name, expr = m_eq.groups()
            expr_clean = expr.split(';')[0].strip()
            expr_clean = oct_to_hex(expr_clean)
            val = simple_eval(expr_clean, symbols)
            if name in symbols:
                # duplicate: if same value ignore, else comment for manual review
                if val is not None and symbols[name]==val:
                    out.append(f"; DUP-EQ {name} = {expr_clean}")
                else:
                    out.append(f"; CONFLICT-EQ {name} = {expr_clean}")
                i+=1; continue
            if val is not None:
                symbols[name]=val
            stats['equates']+=1
            out.append(f"{name} = {expr_clean}")
            i+=1; continue
        # BLOCK
        m_block=RE_BLOCK.match(line)
        if m_block:
            label,size=m_block.groups(); stats['block']+=1
            if label: out.append(f"{label} .res {size}")
            else: out.append(f" .res {size}")
            i+=1; continue
        # ADR
        m_adr=RE_ADR.match(line)
        if m_adr:
            label,sym,cmt=m_adr.groups(); stats['adr']+=1
            if label: out.append(f"{label} .word {sym.strip()}" + (f" {cmt}" if cmt else ''))
            else: out.append(f" .word {sym.strip()}" + (f" {cmt}" if cmt else ''))
            i+=1; continue
        # DCI
        m_dci=RE_DCI.match(line)
        if m_dci:
            label,word,cmt=m_dci.groups(); stats['dci']+=1
            bytes_vals=[]
            for j,ch in enumerate(word):
                b=ord(ch)&0x7F
                if j==len(word)-1: b|=0x80
                bytes_vals.append(f"${b:02X}")
            data='.byte '+', '.join(bytes_vals)
            if label: out.append(f"{label} {data}" + (f" {cmt}" if cmt else ''))
            else: out.append(f" {data}" + (f" {cmt}" if cmt else ''))
            i+=1; continue
        # immediate pseudo opcodes
        m_imm=RE_IMM.match(line)
        if m_imm:
            indent,label,op,arg,comment=m_imm.groups(); stats['imm']+=1
            real=IMM_MAP[op.upper()]
            arg=oct_to_hex(arg.strip())
            if label: out.append(f"{indent}{label} {real} #{arg}{comment or ''}")
            else: out.append(f"{indent}{real} #{arg}{comment or ''}")
            i+=1; continue
        # bare number or label + number
        m_num=RE_NUM_LABEL.match(line)
        if m_num and not line.strip().startswith(';'):
            label,num,cmt=m_num.groups(); num=int(num)
            stats['numbers']+=1
            directive='.word' if num>255 else '.byte'
            if label: out.append(f"{label} {directive} {num}" + (f" {cmt}" if cmt else ''))
            else: out.append(f" {directive} {num}" + (f" {cmt}" if cmt else ''))
            i+=1; continue
        # generic transformations & directive normalization
        line = oct_to_hex(line)
        line = re.sub(r"\bORG\b", ".org", line, flags=re.IGNORECASE)
        line = re.sub(r"==", "=", line)
        out.append(line)
        i+=1
    # Final cleanup pass for any lingering ^O / '==' sequences
    cleaned=[re.sub(r"==","=", oct_to_hex(l)) for l in out]
    return cleaned, {'symbols':symbols,'stats':stats}

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--input', required=True)
    ap.add_argument('--output', required=True)
    ap.add_argument('--summary', required=True)
    args=ap.parse_args()
    src=pathlib.Path(args.input).read_text().splitlines()
    out, meta = translate(src)
    pathlib.Path(args.output).write_text('\n'.join(out)+"\n")
    pathlib.Path(args.summary).write_text(json.dumps(meta, indent=2))

if __name__=='__main__':
    main()
