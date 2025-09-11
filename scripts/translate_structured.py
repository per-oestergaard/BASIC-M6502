def post_process(lines: list[str]) -> list[str]:
    """Structured second pass: normalize data directives, address tables, macros, and artifacts."""
    eq_dup = re.compile(r"^([A-Za-z_.$][\w.$]*)\s*=\s*([^;].*)")
    date_pat = re.compile(r"^\d{1,2}/\d{1,2}/\d{2}")
    seen: set[str] = set()
    out: list[str] = []
    in_block = False
    q_value = None
    for raw in lines:
        line = raw.rstrip('\n')
        s = line.lstrip()
        # BLOCK -> .res
        m = re.match(r"^([A-Za-z_.$][\w.$]*:)?\s*BLOCK\s+(\d+)\b", s, re.IGNORECASE)
        if m:
            label, size = m.group(1) or '', m.group(2)
            out.append(f"{label} .res {size}" if label else f" .res {size}")
            continue
        # ADR(label)
        m = re.match(r"^([A-Za-z_.$][\w.$]*:)\s*ADR\(([^)]+)\)\s*>?\s*(;.*)?$", s, re.IGNORECASE)
        if m:
            label, sym, cmt = m.groups()
            out.append(f"{label} .word {sym.strip()}{' '+cmt if cmt else ''}")
            continue
        m = re.match(r"^ADR\s*\(([^)]+)\)\s*(;.*)?$", s, re.IGNORECASE)
        if m:
            sym, cmt = m.groups()
            out.append(f" .word {sym.strip()}{' '+cmt if cmt else ''}")
            continue
        # Q tracking
        if re.match(r"^Q\s+\.set ", s):
            mq = re.match(r"^Q\s+\.set\s+([0-9+$A-Fa-f\-+*/ ]+)", s)
            if mq:
                expr = mq.group(1).replace('$','0x')
                try: q_value = eval(expr, {"__builtins__":{}}, {})
                except Exception: pass
            out.append(line)
            continue
        # DCI".." expansion
        m = re.match(r'^([A-Za-z_.$][\w.$]*:)?\s*DCI"([^"\\]+)"\s*>?\s*(;.*)?$', s)
        if m:
            label, word, cmt = m.groups()
            if q_value is None: q_value = 0
            q_value += 1
            out.append('Q .set Q+1')
            bytes_vals = []
            for i,ch in enumerate(word):
                b = ord(ch) & 0x7F
                if i == len(word)-1: b |= 0x80
                bytes_vals.append(f"${b:02X}")
            data = '.byte ' + ', '.join(bytes_vals)
            out.append((label+' ' if label else ' ') + data + ((' '+cmt) if cmt else ''))
            continue
        # PAGE -> comment
        if s.startswith('PAGE'):
            out.append('; ' + line)
            continue
        # Macro artifact lines starting with '('
        if s.startswith('('):
            out.append('; MACRO-ARTIFACT ' + line)
            continue
        # COMMENT blocks
        if line.strip() == '*':
            in_block = False
            out.append('; *')
            continue
        if line.startswith('COMMENT *'):
            in_block = True
            out.append('; ' + line)
            continue
        if in_block:
            out.append('; ' + line)
            continue
        # Narrative dating or copyright
        if date_pat.match(line) or line.startswith('COPYRIGHT '):
            out.append('; ' + line)
            continue
        # Duplicate equate handling
        m = eq_dup.match(line)
        if m:
            name, rest = m.groups()
            if name in seen:
                line = f"{name} .set {rest}"
            else:
                seen.add(name)
        # Label + number data
        m = re.match(r"^([A-Za-z_.$][\w.$]*:)?\s+(\d+)\s*(;.*)?$", line)
        if m and m.group(1):
            label, num, cmt = m.groups()
            try: val = int(num)
            except ValueError: val = None
            if val is not None and 255 < val <= 65535:
                line = f"{label} .word {val}{' '+cmt if cmt else ''}".rstrip()
            else:
                line = f"{label} .byte {num}{' '+cmt if cmt else ''}".rstrip()
        else:
            mnum = re.match(r"^\s*(\d+)\s*(;.*)?$", line)
            if mnum:
                num, cmt = mnum.groups()
                try: val = int(num)
                except ValueError: val = None
                if val is not None and 255 < val <= 65535:
                    line = f" .word {val}{' '+cmt if cmt else ''}".rstrip()
                else:
                    line = f" .byte {num}{' '+cmt if cmt else ''}".rstrip()
            else:
                mqs = re.match(r'^\s*("[^"]*")\s*(;.*)?$', line)
                if mqs:
                    sstr, cmt = mqs.groups()
                    line = f" .byte {sstr}{' '+cmt if cmt else ''}".rstrip()
        # Strip stray '>'
        if '>' in line and '<' not in line:
            line = re.sub(r">(?=\s*(;|$))", "", line)
            if line.endswith('>'):
                line = line.rstrip('>')
        # Char immediate CMP #"X" -> CMP #$58
        m = re.match(r'^(\s*(?:[A-Za-z_.$][\w.$]*:)?\s*(?:CMP|SBC|ADC|LDA|LDX|LDY)\s+#)"(.)"(.*)$', line)
        if m:
            pre, ch, rest = m.groups()
            line = f"{pre}${ord(ch):02X}{rest}"
        # Macro param artifacts
        if '<<' in line or re.search(r"<[^>]+>", line):
            if not line.strip().startswith(';'):
                line = '; MACRO-PARAM ' + line
        out.append(line)
    return out
                else:
                    line = f" .byte {num}{' '+cmt if cmt else ''}".rstrip()
            else:
                m_qs = re.match(r'^\s*("[^"]*")\s*(;.*)?$', line)
                if m_qs:
                    s, cmt = m_qs.groups()
                    line = f" .byte {s}{' '+cmt if cmt else ''}".rstrip()
        # Strip stray '>'
        if '>' in line and '<' not in line:
            line = re.sub(r">(?=\s*(;|$))", "", line)
            if line.endswith('>'): line = line.rstrip('>')
        # Char immediates
    m_char = re.match(r'^(\s*(?:[A-Za-z_.$][\w.$]*:?)*\s*(?:CMP|SBC|ADC|LDA|LDX|LDY)\s+#)"(.)"(.*)$', line)
        if m_char:
            pre, ch, rest = m_char.groups()
            line = f"{pre}${ord(ch):02X}{rest}"
        # Macro param artifacts
        if '<<' in line or re.search(r"<[^>]+>", line):
            if not line.strip().startswith(';'):
                line = '; MACRO-PARAM ' + line
        cleaned.append(line)
    return cleaned
            out.append(out_line)
            i += 1
            continue

        # ORG normalization (any leading spaces)
        if re.match(r"^\s*ORG\b", line, re.IGNORECASE):
            line = re.sub(r"ORG",".org", line, flags=re.IGNORECASE)

        # Strip stray trailing '>' left from macro brackets
        if line.rstrip().endswith('>') and '<' not in line:
            line = line.rstrip('>')

        # Comment unresolved macro parameter placeholders like <<WD>
        if '<<' in line or re.search(r"<[^>]+>", line):
            if not line.strip().startswith(';'):
                line = '; MACRO-PARAM ' + line

        # Octal conversion
        line = conv_octal(line)

        out.append(line)
        i += 1

    meta = {'symbols': symbols, 'stats': stats, 'output_lines': len(out), 'input_lines': len(lines), 'debug': debug}
    return out, meta

def post_process(lines: list[str]) -> list[str]:
    """Second pass cleanups:
    - Ensure all later duplicate 'NAME = expr' are '.set'
    - Strip trailing solitary '>'
    - Comment lines with macro param artifacts '<<' or '<WD>' forms
    - Comment narrative blocks (COPYRIGHT / date-stamped notes) heuristically
    - Convert raw numeric data lines & label+number forms to .byte directives
    """
    seen: set[str] = set()
    eq_pat = re.compile(r"^([A-Za-z_.$][\w.$]*)\s*=\s*([^;].*)")
    date_pat = re.compile(r"^\d{1,2}/\d{1,2}/\d{2}")
    copyright_pat = re.compile(r"^COPYRIGHT ")
    cleaned: list[str] = []
    in_comment_block = False
    q_value = None
    q_name = 'Q'
    for idx, line in enumerate(lines):
        raw = line.rstrip('\n')
        stripped = raw.lstrip()
        # Comment PAGE/BLOCK/ADR directives (unsupported in ca65 as-is)
        # Convert BLOCK n -> .res n (reserve bytes)
        m_block = re.match(r"^([A-Za-z_.$][\w.$]*:)?\s*BLOCK\s+(\d+)\b", stripped, re.IGNORECASE)
        if m_block:
            label = m_block.group(1) or ''
            size = m_block.group(2)
            if label:
                cleaned.append(f"{label} .res {size}")
            else:
                cleaned.append(f" .res {size}")
            continue
        # ADR(symbol) -> .word symbol (store address)
        m_adr = re.match(r"^([A-Za-z_.$][\w.$]*:)\s*ADR\(([^)]+)\)\s*>?\s*(;.*)?$", stripped, re.IGNORECASE)
        if m_adr:
            label, sym, cmt = m_adr.groups()
            cleaned.append(f"{label} .word {sym.strip()}{' '+cmt if cmt else ''}")
            continue
        # Standalone ADR(symbol) as table entry
        m_adr2 = re.match(r"^ADR\s*\(([^)]+)\)\s*(;.*)?$", stripped, re.IGNORECASE)
        if m_adr2:
            sym, cmt = m_adr2.groups()
            cleaned.append(f" .word {sym.strip()}{' '+cmt if cmt else ''}")
            continue
        # Track Q initialization
        if re.match(rf"^{q_name}\s+\.set ", stripped):
            # Attempt to parse numeric expression for Q
            m_q = re.match(rf"^{q_name}\s+\.set\s+([0-9+$A-Fa-f\-+*/ ]+)", stripped)
            if m_q:
                expr = m_q.group(1).strip()
                expr_eval = expr.replace('$','0x')
                try:
                    q_value = eval(expr_eval, {"__builtins__":{}}, {})
                except Exception:
                    pass
            cleaned.append(raw)
            continue
    # DCI"WORD" expansion (simulate macro: Q=Q+1 then emit string with high bit on last char)
    m_dci = re.match(r'^([A-Za-z_.$][\w.$]*:)?\s*DCI"([^"\\]+)"\s*>?\s*(;.*)?$', stripped)
    if m_dci:
            label, word, cmt = m_dci.groups()
            if q_value is None:
                q_value = 0
            q_value += 1
            cleaned.append(f"{q_name} .set {q_name}+1")  # keep assembler's Q in sync
            bytes_vals = []
            for i, ch in enumerate(word):
                code = ord(ch) & 0x7F
                if i == len(word) - 1:
                    code |= 0x80
                bytes_vals.append(f"${code:02X}")
            data_line = f".byte {', '.join(bytes_vals)}"
            if label:
                cleaned.append(f"{label} {data_line}{' '+cmt if cmt else ''}".rstrip())
            else:
                cleaned.append(f" {data_line}{' '+cmt if cmt else ''}".rstrip())
            continue
        if re.match(r"^(PAGE)\b", stripped):
            cleaned.append('; ' + raw)
            continue
        # Comment lines starting with '(' (macro arg artifacts)
        if stripped.startswith('('):
            cleaned.append('; MACRO-ARTIFACT ' + raw)
            continue
        if raw.strip() == '*':
            in_comment_block = False
            cleaned.append('; *')
            continue
        if raw.startswith('COMMENT *'):
            in_comment_block = True
            cleaned.append('; ' + raw)
            continue
        if in_comment_block:
            cleaned.append('; ' + raw)
            continue
        if copyright_pat.match(raw) or date_pat.match(raw):
            cleaned.append('; ' + raw)
            continue
        m = eq_pat.match(raw)
        if m:
            name = m.group(1)
            rest = m.group(2)
            if name in seen:
                raw = f"{name} .set {rest}"
            else:
                seen.add(name)
        # Label followed directly by a single integer -> data byte
        m_lbl_num = re.match(r"^([A-Za-z_.$][\w.$]*:)\s+(\d+)\s*(;.*)?$", raw)
        if m_lbl_num:
            label, num, cmt = m_lbl_num.groups()
            try:
                val = int(num)
            except ValueError:
                val = None
            if val is not None and val > 255 and val <= 65535:
                raw = f"{label} .word {val}{' '+cmt if cmt else ''}".rstrip()
            else:
                raw = f"{label} .byte {num}{' '+cmt if cmt else ''}".rstrip()
        else:
            # Standalone numeric data line
            m_num = re.match(r"^\s*(\d+)\s*(;.*)?$", raw)
            if m_num:
                num, cmt = m_num.groups()
                try:
                    val = int(num)
                except ValueError:
                    val = None
                if val is not None and val > 255 and val <= 65535:
                    raw = f" .word {val}{' '+cmt if cmt else ''}".rstrip()
                else:
                    raw = f" .byte {num}{' '+cmt if cmt else ''}".rstrip()
            else:
                # Standalone quoted string -> .byte "..."
                m_q = re.match(r'^\s*("[^"]*")\s*(;.*)?$', raw)
                if m_q:
                    s, cmt = m_q.groups()
                    raw = f" .byte {s}{' '+cmt if cmt else ''}".rstrip()
        # strip stray '>' including before comments (e.g. FCERR>  ;comment)
        if '>' in raw and '<' not in raw:
            raw = re.sub(r">(?=\s*(;|$))", "", raw)
            if raw.endswith('>'):
                raw = raw.rstrip('>')
        # Convert single-char immediates like CMP #"X" -> CMP #$58
        m_imm_char = re.match(r"^(\s*(?:[A-Za-z_.$][\w.$]*:)?\s*(?:CMP|SBC|ADC|LDA|LDX|LDY)\s+#)\"(.)\"(.*)$", raw)
        if m_imm_char:
            pre, ch, rest = m_imm_char.groups()
            raw = f"{pre}${ord(ch):02X}{rest}"
        # macro param artifacts
        if '<<' in raw or re.search(r"<[^>]+>", raw):
            if not raw.strip().startswith(';'):
                raw = '; MACRO-PARAM ' + raw
        cleaned.append(raw)
    return cleaned

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--input', required=True)
    ap.add_argument('--output', required=True)
    ap.add_argument('--summary', required=True)
    ap.add_argument('--debug', action='store_true')
    args = ap.parse_args()
    src = pathlib.Path(args.input)
    lines = src.read_text(encoding='utf-8', errors='ignore').splitlines()
    translated, meta = translate(lines, debug=args.debug)
    translated = post_process(translated)
    out_path = pathlib.Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text('\n'.join(translated)+'\n', encoding='utf-8')
    meta['source'] = str(src)
    meta_path = pathlib.Path(args.summary)
    meta_path.parent.mkdir(parents=True, exist_ok=True)
    meta_path.write_text(json.dumps(meta, indent=2), encoding='utf-8')
    if args.debug:
        (out_path.parent/'structured_log.txt').write_text(json.dumps(meta, indent=2), encoding='utf-8')

if __name__ == '__main__':
    main()

if __name__ == '__main__':
    main()
