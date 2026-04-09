//! Microsoft BASIC Interpreter implemented in Rust
//!
//! This crate implements a BASIC interpreter compatible with Microsoft BASIC
//! for the 6502 processor, circa 1976-1978.

pub mod interpreter;
pub mod lexer;
pub mod parser;

/// Run a BASIC program with optional queued input
pub fn run_program(source: &str, inputs: &[String]) -> Result<String, String> {
    // Parse the program
    let program = parser::parse(source)?;

    // Create interpreter with queued inputs
    let mut interp = interpreter::Interpreter::new();
    for input in inputs {
        interp.queue_input(input);
    }

    // Execute
    interp.run(&program)
}

/// Run a BASIC program reading from stdin
pub fn run_program_interactive(source: &str) -> Result<String, String> {
    let program = parser::parse(source)?;
    let mut interp = interpreter::Interpreter::new();
    interp.run(&program)
}
