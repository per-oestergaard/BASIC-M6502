#!/usr/bin/env bash
set -euo pipefail

## Purpose: Attempt step 2 (build original interpreter) by
## 1. Running a lightweight translation stub to produce a ca65-ish file
## 2. Invoking ca65 + ld65 to gauge current incompatibilities
## 3. Capturing diagnostics for future incremental translation work

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ASM_SRC="$ROOT_DIR/m6502.asm"
BUILD_DIR="$ROOT_DIR/build/original"
TRANS_DIR="$BUILD_DIR/translated"
REPORT_DIR="$BUILD_DIR/report"
OUT_OBJ="$BUILD_DIR/m6502.o"
OUT_LIST="$BUILD_DIR/m6502.lst"
OUT_ERR="$REPORT_DIR/assemble_errors.txt"
META_JSON="$REPORT_DIR/translation_summary.json"

mkdir -p "$BUILD_DIR" "$TRANS_DIR" "$REPORT_DIR"

echo "[step2] Minimal ca65 translator running..." >&2
if python3 "$ROOT_DIR/scripts/translate_min_ca65.py" \
	--input "$ASM_SRC" \
	--output "$TRANS_DIR/m6502_ca65.asm" \
	--summary "$META_JSON"; then
	:
else
	echo "[step2] minimal translator failed; abort" >&2
	exit 2
fi

echo "[step2] Invoking ca65 (expect errors until full translation implemented)..." >&2 || true
set +e
ca65 -l "$OUT_LIST" -o "$OUT_OBJ" "$TRANS_DIR/m6502_ca65.asm" 2>"$OUT_ERR"
STATUS=$?
set -e

if [[ $STATUS -ne 0 ]]; then
	echo "[step2] Assembly failed (expected). See $OUT_ERR for details." >&2
	echo "[step2] Summary of first 10 error lines:" >&2
	grep -i "error" "$OUT_ERR" | head -n 10 >&2 || true
	exit 3
else
	echo "[step2] Assembly succeeded unexpectedly; review output object at $OUT_OBJ" >&2
fi

exit 0
