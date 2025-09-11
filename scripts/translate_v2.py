#!/usr/bin/env python3
"""translate_v2.py
Clean two-pass translator of Microsoft 6502 BASIC (PDP-10 style) to ca65-friendly form.
Focus: Apple II target REALIO=4. Minimal macro emulation (DCI, ADR, BLOCK, immediate pseudo ops).
"""
from __future__ import annotations
import re, argparse, pathlib, json, sys

TARGET_REALIO = 4

# Token types
class Tok:
    def __init__(self, kind:str, raw:str, **kw):
        self.kind=kind; self.raw=raw; self.__dict__.update(kw)
    def __repr__(self): return f"Tok({self.kind},{self.raw!r})"

RE_DIRECTIVE = re.compile(r"^(TITLE|SEARCH|SALL|RADIX|SUBTTL)\b", re.IGNORECASE)
RE_OCT = re.compile(r"\^O([0-7]+)")
RE_EQU = re.compile(r"^\s*([A-Za-z_.$][\w.$]*)\s*==?\s*([^;]+)")
RE_IF = re.compile(r"^(IFE|IFN)\s+([^,]+),<", re.IGNORECASE)
RE_IF_PASS = re.compile(r"^IF\d+,<")
RE_DEFINE = re.compile(r"^\s*DEFINE\s+([A-Za-z_][A-Za-z0-9_]*)\s*(\(([^)]*)\))?,<", re.IGNORECASE)
RE_LABEL = re.compile(r"^([A-Za-z_.$][\w.$]*:)")
RE_BLOCK = re.compile(r"^([A-Za-z_.$][\w.$]*:)?\s*BLOCK\s+(\d+)\b", re.IGNORECASE)
RE_ADR = re.compile(r"^([A-Za-z_.$][\w.$]*:)?\s*ADR\(([^)]+)\)\s*>?\s*(;.*)?$", re.IGNORECASE)
RE_DCI = re.compile(r'^([A-Za-z_.$][\w.$]*:)?\s*DCI"([^"\\]+)"\s*>?\s*(;.*)?$')
RE_IMM_MAC = re.compile(r"^(\s*)(?:([A-Za-z_.$][\w.$]*):\s*)?(LDAI|LDXI|LDYI|CMPI|SBCI|ADCI)\s+([^;]+?)(\s*;.*)?$")
RE_NUM = re.compile(r"^\s*(\d+)\s*(;.*)?$")
RE_LABEL_NUM = re.compile(r"^([A-Za-z_.$][\w.$]*:)\s+(\d+)\s*(;.*)?$")

IMM_MAP = {'LDAI':'LDA','LDXI':'LDX','LDYI':'LDY','CMPI':'CMP','SBCI':'SBC','ADCI':'ADC'}

# Utilities

def conv_octal(s:str)->str:
    return RE_OCT.sub(lambda m: f"${int(m.group(1),8):X}", s)

def simple_eval(expr:str, symbols:dict[str,int])->int|None:
    expr=conv_octal(expr)
    expr=expr.replace('==','=').strip()
    def repl(m):
        n=m.group(0)
        return str(symbols[n]) if n in symbols else n
    expr=re.sub(r"[A-Za-z_.$][A-Za-z0-9_.$]*", repl, expr)
    expr=re.sub(r"\$([0-9A-Fa-f]+)", lambda m: str(int(m.group(1),16)), expr)
    if not re.fullmatch(r"[0-9+\-*/() ]*", expr):
        return None
    try: return int(eval(expr, {"__builtins__":{}}, {}))
    except: return None

class Parser:
    def __init__(self, lines:list[str]):
        self.lines=lines
        self.i=0
        self.symbols={'REALIO':TARGET_REALIO}
        self.defined=set()
        self.tokens:list[Tok]=[]

    def extract_block(self,start:int)->tuple[list[str],int]:
        body=[]; i=start
        while i < len(self.lines):
            raw=self.lines[i].rstrip('\n')
            if raw.endswith('>>'):
                body.append(raw[:-2]); return body,i+1
            if raw.endswith('>'):
                body.append(raw[:-1]); return body,i+1
            body.append(raw); i+=1
        return body,i

    def parse(self):
        while self.i < len(self.lines):
            raw=self.lines[self.i].rstrip('\n')
            line=raw.replace('\t',' ')
            # directives -> comment tokens
            if RE_DIRECTIVE.match(line):
                self.tokens.append(Tok('COMMENT', raw))
                self.i+=1; continue
            # equate
            m=RE_EQU.match(line)
            if m:
                name, expr = m.group(1), m.group(2).split(';')[0].strip()
                expr_c=conv_octal(expr)
                val=simple_eval(expr_c,self.symbols)
                if val is not None: self.symbols[name]=val
                self.tokens.append(Tok('EQU', raw, name=name, expr=expr_c))
                self.i+=1; continue
            # conditional block
            m=RE_IF.match(line)
            if m:
                kind=m.group(1).upper(); expr=m.group(2).strip()
                body, ni=self.extract_block(self.i+1)
                # store conditional kind in separate attr 'cond'
                self.tokens.append(Tok('IF', raw, cond=kind, expr=expr, body=body))
                self.i=ni; continue
            if RE_IF_PASS.match(line):
                body, ni=self.extract_block(self.i+1)
                self.tokens.append(Tok('IFPASS', raw, body=body))
                self.i=ni; continue
            # macro definition
            m=RE_DEFINE.match(line)
            if m:
                name=m.group(1); params=(m.group(3) or '').strip()
                body, ni=self.extract_block(self.i+1)
                self.tokens.append(Tok('MACRODEF', raw, name=name, params=params, body=body))
                self.i=ni; continue
            # immediate macro line
            m=RE_IMM_MAC.match(line)
            if m:
                indent,label,mac,arg,comment = m.groups()
                arg=conv_octal(arg.strip())
                self.tokens.append(Tok('IMMACRO', raw, indent=indent,label=label,mac=mac,arg=arg,comment=comment or ''))
                self.i+=1; continue
            # block / adr / dci / data recognized later in emit
            self.tokens.append(Tok('RAW', raw))
            self.i+=1
        return self.tokens

class Emitter:
    def __init__(self, tokens:list[Tok], symbols:dict[str,int]):
        self.tokens=tokens
        self.symbols=symbols
        self.out:list[str]=["; translate_v2 output (REALIO=4)"]
        self.q_active=False
    self.comment_mode=False

    def include_if(self, cond:str, expr:str)->bool|None:
        val=simple_eval(expr,self.symbols)
        if val is None: return None
        return (val==0) if cond=='IFE' else (val!=0)

    def emit(self):
        for t in self.tokens:
            if t.kind=='COMMENT':
                self.out.append('; '+t.raw)
            elif t.kind=='EQU':
                expr = t.expr.rstrip('>')
                if '<<' in expr or re.search(r"<[^>]*>", expr):
                    self.out.append(f"; MACRO-PARAM {t.name} .set {expr}")
                else:
                    self.out.append(f"{t.name} .set {expr}")
            elif t.kind=='IMMACRO':
                base=IMM_MAP[t.mac]
                # transform legacy high-byte macro param pattern <<SYMBOL>/$100>> to >SYMBOL
                arg=t.arg
                arg=re.sub(r"<<([A-Za-z_.$][A-Za-z0-9_.$]*)>\s*/\s*\$100>>", r">\1", arg)
                # if any remaining macro markers, just comment the line
                if '<<' in arg or '>>' in arg:
                    self.out.append(f"; MACRO-PARAM {t.raw}"); continue
                # handle single char immediate
                if re.fullmatch(r'"."', arg):
                    arg=f"${ord(arg[1]):02X}"
                m_expr=re.match(r'^(\d+)-"(.)"$', arg)
                if m_expr:
                    n=int(m_expr.group(1)); ch=ord(m_expr.group(2)); arg=f"${(n-ch)&0xFFFF:X}"
                self.out.append(f"{t.label+': ' if t.label else ''}{base} #{arg}{t.comment}")
            elif t.kind=='IF':
                cond=getattr(t,'cond','IFE')
                inc=self.include_if(cond, t.expr)
                self.out.append(f"; {cond} {t.expr} -> {'include' if inc else 'exclude' if inc==False else 'unknown'}")
                if inc:
                    sub_tokens=Parser([*t.body]).parse()
                    self.out.extend(Emitter(sub_tokens,self.symbols).emit())
            elif t.kind=='IFPASS':
                self.out.append("; IFPASS assumed include")
                sub_tokens=Parser([*t.body]).parse()
                self.out.extend(Emitter(sub_tokens,self.symbols).emit())
            elif t.kind=='MACRODEF':
                self.out.append(f"; MACRO {t.name}({t.params}) len={len(t.body)} (stubbed)")
            else: # RAW
                self.handle_raw(t.raw)
        return self.out

    def handle_raw(self,line:str):
        l=line.strip()
        # Enter/maintain prose comment mode after COMMENT * marker
        if 'COMMENT *' in l:
            self.comment_mode=True
            self.out.append('; '+line)
            return
        if self.comment_mode:
            code_pat=r'^\s*([A-Za-z_.$][\w.$]*:)?\s*(\.[A-Za-z]+|ADC|AND|ASL|BCC|BCS|BEQ|BIT|BMI|BNE|BPL|BRK|BVC|BVS|CLC|CLD|CLI|CLV|CMP|CPX|CPY|DEC|DEX|DEY|EOR|INC|INX|INY|JMP|JSR|LDA|LDX|LDY|LSR|NOP|ORA|PHA|PHP|PLA|PLP|ROL|ROR|RTI|RTS|SBC|SEC|SED|SEI|STA|STX|STY|TAX|TAY|TSX|TXA|TXS|TYA)\b'
            if re.match(code_pat,l):
                self.comment_mode=False
            else:
                self.out.append('; '+line)
                return
        # Neutralize double-angle macro param patterns early
        if '<<' in l:
            self.out.append('; MACRO-PARAM '+line)
            return
        # ORG
        if re.match(r'^ORG\b', l, re.IGNORECASE):
            self.out.append(re.sub(r'ORG', '.org', line, flags=re.IGNORECASE)); return
        # Double-colon labels to single
        if re.match(r'^\$Z::?', line):
            # ca65 does not accept '$' starting label; map to ZSTART
            self.out.append('ZSTART: ; renamed from $Z'); return
        # Lines that are pure text (no opcode) early banner (avoid assembler treating words as labels)
        if re.match(r'^(COPYRIGHT|[-]{2,}|[-0-9/]{3,}|COMMENT \*)', l):
            self.out.append('; TEXT '+line); return
        # Changelog style lines: date or tabs with uppercase words
        if re.match(r'^[0-9]{1,2}/[0-9]{1,2}/[0-9]{2}', l):
            self.out.append('; CHANGE '+line); return
        if re.match(r'^\t[A-Z]', line):
            self.out.append('; CHANGE '+line); return
        # Uppercase prose lines (heuristic: multiple spaces and no opcode mnemonic) -> comment
        if re.match(r'^[A-Z0-9 \,\"\-]{6,}$', l) and not re.match(r'^(ADC|AND|ASL|BCC|BCS|BEQ|BIT|BMI|BNE|BPL|BRK|BVC|BVS|CLC|CLD|CLI|CLV|CMP|CPX|CPY|DEC|DEX|DEY|EOR|INC|INX|INY|JMP|JSR|LDA|LDX|LDY|LSR|NOP|ORA|PHA|PHP|PLA|PLP|ROL|ROR|RTI|RTS|SBC|SEC|SED|SEI|STA|STX|STY|TAX|TAY|TSX|TXA|TXS|TYA)\b', l):
            self.out.append('; TEXT '+line); return
        # BLOCK
        m=RE_BLOCK.match(l)
        if m:
            label,size=m.group(1) or '', m.group(2)
            if label: self.out.append(f"{label} .res {size}")
            else: self.out.append(f" .res {size}"); return
        # ADR
        m=RE_ADR.match(l)
        if m:
            label,sym,cmt=m.groups();
            if label: self.out.append(f"{label} .word {sym.strip()}{' '+cmt if cmt else ''}")
            else: self.out.append(f" .word {sym.strip()}{' '+cmt if cmt else ''}"); return
        # DCI
        m=RE_DCI.match(l)
        if m:
            label,word,cmt=m.groups()
            self.out.append('Q .set Q+1')
            bytes_vals=[]
            for i,ch in enumerate(word):
                b=ord(ch)&0x7F
                if i==len(word)-1: b|=0x80
                bytes_vals.append(f"${b:02X}")
            data='.byte '+', '.join(bytes_vals)
            self.out.append((label+' ' if label else ' ')+data+(' '+cmt if cmt else ''))
            return
        # numeric data (label + number)
        m=RE_LABEL_NUM.match(l)
        if m:
            label,num,cmt=m.groups(); val=int(num)
            if 255 < val <= 65535: self.out.append(f"{label} .word {val}{' '+cmt if cmt else ''}")
            else: self.out.append(f"{label} .byte {val}{' '+cmt if cmt else ''}")
            return
        m=RE_NUM.match(l)
        if m:
            num,cmt=m.groups(); val=int(num)
            if 255 < val <= 65535: self.out.append(f" .word {val}{' '+cmt if cmt else ''}")
            else: self.out.append(f" .byte {val}{' '+cmt if cmt else ''}")
            return
        # char immediates like CMP #"X" -> after initial pass we do a regex replace
        line2=re.sub(r'(?:CMP|SBC|ADC|LDA|LDX|LDY)\s+#"(.)"', lambda m:f"{m.group(0)[:m.group(0).index('#')]}#${ord(m.group(1)):02X}", line)
        # strip stray '>' with no matching '<'
        if '>' in line2 and '<' not in line2:
            line2=re.sub(r">(?=\s*(;|$))","", line2)
        # macro param artifacts
        if '<<' in line2 or re.search(r"<[^>]+>", line2):
            if not line2.strip().startswith(';'):
                line2='; MACRO-PARAM '+line2
        self.out.append(line2)


def run(input_path:str, output_path:str, summary_path:str, debug:bool=False):
    lines=pathlib.Path(input_path).read_text(encoding='utf-8',errors='ignore').splitlines()
    p=Parser(lines)
    tokens=p.parse()
    em=Emitter(tokens,p.symbols)
    out_lines=em.emit()
    pathlib.Path(output_path).parent.mkdir(parents=True, exist_ok=True)
    pathlib.Path(output_path).write_text('\n'.join(out_lines)+'\n',encoding='utf-8')
    meta={
        'input':input_path,
        'output':output_path,
        'lines_in':len(lines),
        'lines_out':len(out_lines),
        'symbols':p.symbols,
        'token_counts':{k:sum(1 for t in tokens if t.kind==k) for k in {t.kind for t in tokens}}
    }
    pathlib.Path(summary_path).write_text(json.dumps(meta,indent=2),encoding='utf-8')
    if debug:
        sys.stderr.write(json.dumps(meta,indent=2)+'\n')

if __name__=='__main__':
    ap=argparse.ArgumentParser()
    ap.add_argument('--input',required=True)
    ap.add_argument('--output',required=True)
    ap.add_argument('--summary',required=True)
    ap.add_argument('--debug',action='store_true')
    a=ap.parse_args()
    run(a.input,a.output,a.summary,a.debug)
