#!/bin/bash
# Start the Rust BASIC interpreter (interactive REPL)
exec cargo run -p basic_interpreter --example interactive -- "$@"
