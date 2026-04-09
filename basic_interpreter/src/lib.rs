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
    let program = match parser::parse(source) {
        Ok(p) => p,
        Err(e) => {
            // Parse errors are already formatted as ?SN ERROR if they come from
            // the parser with line context. Return as successful output (like original BASIC).
            if e.starts_with('?') {
                return Ok(e);
            }
            return Err(e);
        }
    };

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
    let program = match parser::parse(source) {
        Ok(p) => p,
        Err(e) => {
            if e.starts_with('?') {
                return Ok(e);
            }
            return Err(e);
        }
    };
    let mut interp = interpreter::Interpreter::new();
    interp.run(&program)
}
