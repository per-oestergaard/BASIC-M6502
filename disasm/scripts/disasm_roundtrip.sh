#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ORIG_BIN="$ROOT/orig/basic.bin"
CFG="$ROOT/basic.cfg"
WORK="$ROOT/work"
SRC_DIR="$ROOT/src"
OUT_DISASM="$WORK/basic.s"
REASM_OBJ="$WORK/basic.o"
REASM_BIN="$WORK/basic_rebuilt.bin"
REPORT="$WORK/report.txt"

if [[ ! -f "$ORIG_BIN" ]]; then
  echo "Missing original binary at $ORIG_BIN" >&2
  exit 2
fi
mkdir -p "$WORK" "$SRC_DIR"

da65 -C "$CFG" -o "$OUT_DISASM" "$ORIG_BIN"

# Copy curated source target on first run if empty
if [[ ! -f "$SRC_DIR/basic.asm" ]]; then
  cp "$OUT_DISASM" "$SRC_DIR/basic.asm"
fi

# Assemble with ca65
ca65 -o "$REASM_OBJ" "$SRC_DIR/basic.asm" 2>>"$REPORT"
# Link with a simple binary cfg (create if absent)
CFG_LINK="$WORK/temp_link.cfg"
cat > "$CFG_LINK" <<EOF
MEMORY { ROM: start = $0800, size = $2000, file = %O; }
SEGMENTS { CODE: load = ROM, type = ro; }
EOF

ld65 -C "$CFG_LINK" -o "$REASM_BIN" "$REASM_OBJ" 2>>"$REPORT"

# Strip load address if present (compare payload)
python3 - <<'PY'
import sys,hashlib
orig_path=sys.argv[1]; new_path=sys.argv[2]
with open(orig_path,'rb') as f:orig=f.read()
with open(new_path,'rb') as f:new=f.read()
# If binaries include load address (first two bytes little-endian of $0800)
if orig[:2]==b'\x00\x08' and new[:2]==b'\x00\x08':
    orig=orig[2:]; new=new[2:]
hash_info=lambda b:(len(b),hashlib.md5(b).hexdigest(),hashlib.crc32(b).to_bytes(4,'big').hex())
print('ORIG', *hash_info(orig))
print('NEW ', *hash_info(new))
# Basic diff summary
mismatches=sum(1 for i,(a,b) in enumerate(zip(orig,new)) if a!=b)
print('MISMATCH_BYTES', mismatches)
if len(orig)==len(new) and mismatches==0:
    print('BINARY OK (identical)')
else:
    print('BINARY DIFFER -- refine config / source annotations')
PY "$ORIG_BIN" "$REASM_BIN" | tee "$WORK/diff_summary.txt"

echo "Round-trip complete. Edit $CFG (ranges/symbols) and/or $SRC_DIR/basic.asm then re-run." >&2
