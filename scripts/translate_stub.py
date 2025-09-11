#!/usr/bin/env python3
"""translate_stub.py (enhanced)

Incremental translator toward ca65 compatibility. Still incomplete, but:
 1. Comments out unsupported global directives (TITLE/SEARCH/SALL/RADIX/SUBTTL/SUBTTL)
 2. Converts PDP-10 style octal constants (^O123) to hex ($...)
 3. Normalizes '==' to '=' for equates
 4. Tracks simple constant definitions (NAME = expr) (expr limited to + - * / integers / known symbols)
 5. Handles simple single-angle conditional groups: IFE <expr>,<body> and IFN <expr>,<body>
     - Only linear groups with < starting and '>' or '>>' terminating line
     - Evaluates expression after symbol substitution; includes or excludes body
     - Otherwise comments the group for later
 6. Simplifies labels with double colon (::) to single ':'
 7. Leaves macro DEFINE / IRPC as TODO comments (future: .macro conversion)

Limitations:
 - Nested / multi-line bracketed conditionals beyond simple group not supported
 - Complex expressions, macro expansion, IRPC body not parsed
 - Many assembler directives remain unhandled

Goal: Reduce early syntax errors so assembler can progress further, enabling iterative refinement toward a working binary.
"""
from __future__ import annotations
import argparse, json, re, sys, pathlib
import math

META_KEYS = ["DEFINE","IFE","IFN","IRPC"]

RE_DIRECTIVE = re.compile(r"^(TITLE|SEARCH|SALL|RADIX|SUBTTL)\b", re.IGNORECASE)
RE_MACRO = re.compile(r"^(DEFINE|IRPC)\b", re.IGNORECASE)
RE_IF = re.compile(r"^(IFE|IFN)\s+([^,]+),<(?P<body_hint>.*)$", re.IGNORECASE)
RE_OCTAL = re.compile(r"\^O([0-7]+)")
RE_EQUATE = re.compile(r"^([A-Za-z_.$][\w.$]*)\s*==?\s*([^;]+)")
RE_DOUBLE_COLON = re.compile(r"^([A-Za-z_.$][\w.$]*)::")

def oct_to_hex(match: re.Match) -> str:
    value = int(match.group(1), 8)
    return f"${value:X}"

def eval_simple(expr: str, symbols: dict[str,int]) -> int | None:
    expr = expr.strip()
    if not expr:
        return None
    expr = RE_OCTAL.sub(lambda m: str(int(m.group(1),8)), expr)
    def replace_symbol(tok: str) -> str:
        return str(symbols[tok]) if tok in symbols else tok
    tokens = re.split(r"([+\-*/()])", expr)
    rebuilt = []
    for t in tokens:
        if re.fullmatch(r"[A-Za-z_.$][A-Za-z0-9_.$]*", t):
            rebuilt.append(replace_symbol(t))
        else:
            rebuilt.append(t)
    safe = ''.join(rebuilt)
    if not re.fullmatch(r"[0-9+\-*/() ]+", safe):
        return None
    try:
        return int(eval(safe, {"__builtins__":{}}, {}))
    except Exception:
        return None

def process(lines):
    counts = {k:0 for k in META_KEYS}
    out: list[str] = []
    symbols: dict[str,int] = {}
    i = 0
    n = len(lines)
    while i < n:
        raw = lines[i].rstrip('\n')
        line = raw.replace('\t',' ')
        if RE_DIRECTIVE.match(line):
            out.append(f"; ORIGINAL-DIRECTIVE: {line}")
            i += 1
            continue
        mdc = RE_DOUBLE_COLON.match(line)
        if mdc:
            label = mdc.group(1)
            line = RE_DOUBLE_COLON.sub(label+':', line, count=1)
        # Convert leading $ label to .Z (ca65 doesn't allow $ prefix as symbol start)
        if re.match(r"\$([A-Za-z0-9_]+):", line):
            line = re.sub(r"\$([A-Za-z0-9_]+):", lambda m: f"LBL_{m.group(1)}:", line, count=1)
        meq = RE_EQUATE.match(line)
        if meq:
            name, expr = meq.group(1), meq.group(2)
            expr_no_comment = expr.split(';')[0].strip()
            expr_simple = expr_no_comment.replace('==','=').replace('=',' ')
            val = eval_simple(expr_simple, symbols)
            if val is not None:
                # Check for duplicate definition
                if name in symbols and symbols[name] == val:
                    i += 1
                    continue
                symbols[name] = val
            rhs = expr_no_comment
            rhs = RE_OCTAL.sub(oct_to_hex, rhs)
            comment = ''
            if ';' in expr:
                comment = ';' + expr.split(';',1)[1]
            out.append(f"{name} = {rhs} {comment}".rstrip())
            i += 1
            continue
        mif = RE_IF.match(line)
        if mif:
            kind = mif.group(1).upper()
            expr = mif.group(2).strip()
            counts[kind] += 1
            i += 1
            body_lines = []
            closed = False
            while i < n:
                inner_raw = lines[i].rstrip('\n')
                terminator = None
                if inner_raw.endswith('>>'):
                    terminator = '>>'
                elif inner_raw.endswith('>'):
                    terminator = '>'
                if terminator:
                    inner_line = inner_raw[:-len(terminator)]
                    if inner_line:
                        body_lines.append(inner_line)
                    i += 1
                    closed = True
                    break
                body_lines.append(inner_raw)
                i += 1
            value = eval_simple(expr.replace('==','='), symbols)
            include = None
            if value is not None:
                include = (value == 0) if kind == 'IFE' else (value != 0)
            if include is None:
                out.append(f"; TODO-CONDITIONAL {kind} {expr} (expression unsupported)")
                out.extend([f"; {bl}" for bl in body_lines])
            elif include:
                out.append(f"; {kind} {expr} -> included")
                out.extend(body_lines)
            else:
                out.append(f"; {kind} {expr} -> excluded")
            if not closed:
                out.append("; WARNING: unterminated conditional group")
            continue
        if RE_MACRO.match(line):
            head = line.split()[0].upper()
            if head in counts:
                counts[head]+=1
            out.append(f"; TODO-MACRO: {line}")
            i += 1
            continue
        if '^O' in line:
            line = RE_OCTAL.sub(oct_to_hex, line)
        if '==' in line:
            line = line.replace('==','=')
        # Normalize tight equates like NAME=Value (add space) to keep ca65 parser happy
        if re.match(r"^[A-Za-z_.$][\w.$]*=", line) and ' =' not in line.split('=',1)[0]:
            line = line.replace('=', ' = ', 1)
        # If this looks like an equate inside included block, attempt duplicate skip
        m_inline = re.match(r"^([A-Za-z_.$][\w.$]*)\s*=\s*([^;]+)$", line)
        if m_inline:
            name = m_inline.group(1)
            rhs_expr = m_inline.group(2).strip()
            if name in symbols:
                # Already defined: comment out duplicate
                line = f"; DUP-DEF {line}"
            else:
                val = eval_simple(rhs_expr, symbols)
                if val is not None:
                    symbols[name] = val
        # Convert ORG to ca65 .org
        if re.match(r"\s*ORG\b", line, re.IGNORECASE):
            line = re.sub(r"\s*ORG", ".org", line, count=1, flags=re.IGNORECASE)
        # Clean stray closing angle from conditional residue
        if line.strip().endswith('>') and not line.strip().startswith(';'):
            # If no matching '<' on same line and not a macro marker, strip
            if '<' not in line:
                line = line.rstrip('>')
        out.append(line)
        i += 1
    return out, counts

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--input', required=True)
    ap.add_argument('--output', required=True)
    ap.add_argument('--summary', required=True)
    args = ap.parse_args()

    src_path = pathlib.Path(args.input)
    dst_path = pathlib.Path(args.output)
    summary_path = pathlib.Path(args.summary)

    text = src_path.read_text(encoding='utf-8', errors='ignore').splitlines(True)
    translated, counts = process(text)

    dst_path.parent.mkdir(parents=True, exist_ok=True)
    dst_path.write_text('\n'.join(translated)+'\n', encoding='utf-8')

    summary = {
        'source': str(src_path),
        'output': str(dst_path),
        'counts': counts,
        'note': 'Enhanced stub translation; still incomplete but performs limited conditional pruning and octal conversion.'
    }
    summary_path.parent.mkdir(parents=True, exist_ok=True)
    summary_path.write_text(json.dumps(summary, indent=2), encoding='utf-8')

if __name__ == '__main__':
    main()
