#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"
printf '==> Build (cargo build)...\n'
cargo build --quiet --manifest-path assembler6502/Cargo.toml
printf '==> Run assembler on m6502.asm...\n'
# Allow extra args to pass through (e.g., alternate source file)
if [[ $# -gt 0 ]]; then
  SRC="$1"
else
  SRC="m6502.asm"
fi
if [[ ! -f "$SRC" ]]; then
  echo "Source file $SRC not found" >&2
  exit 1
fi
# Run and show only first 80 lines of error/output to keep concise
set +e
OUTPUT="$(target/debug/asm "$SRC" 2>&1)"
STATUS=$?
set -e
if [[ $STATUS -ne 0 ]]; then
  echo "Assembler failed (exit $STATUS). Showing first lines:" >&2
  echo "$OUTPUT" | head -n 60
  exit $STATUS
else
  echo "Assembler succeeded."; fi
