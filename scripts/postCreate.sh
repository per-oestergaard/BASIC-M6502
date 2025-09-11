#!/usr/bin/env bash
set -euo pipefail
echo "[postCreate] Environment info:" >&2
uname -a || true
echo "cc65 version:" >&2
cc65 --version || true
echo "Rust version:" >&2
rustc --version || true
echo "cargo version:" >&2
cargo --version || true
echo "Done."
