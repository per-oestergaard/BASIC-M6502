#!/usr/bin/env bash
set -euo pipefail

echo "[postCreate] Environment info:" >&2
uname -a || true

echo "" >&2
echo "[postCreate] 6502 toolchain (cc65):" >&2
ca65 --version || true
ld65 --version || true
python3 --version || true

echo "" >&2
echo "[postCreate] Rust toolchain:" >&2
rustc --version || true
cargo --version || true

echo "" >&2
echo "[postCreate] Building Rust workspace to verify environment..." >&2
cargo build --workspace || true

echo "" >&2
echo "[postCreate] Done."
