use emu6502::BasicHarness;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let binary_path = workspace_root().join("build/original/basic.bin");

    if !binary_path.exists() {
        eprintln!("Error: BASIC interpreter binary not found at:");
        eprintln!("  {}", binary_path.display());
        eprintln!("\nPlease build it first with:");
        eprintln!("  cargo run --release --bin asm");
        std::process::exit(1);
    }

    println!("Microsoft BASIC Interactive REPL");
    println!("=================================");
    println!("Type BASIC programs line by line.");
    println!("Commands: RUN, LIST, NEW, LOAD <file>, SAVE <file>");
    println!("Type 'exit' or Ctrl-D to quit.\n");

    let mut program_lines = Vec::new();

    loop {
        print!("] ");
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF (Ctrl-D)
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                continue;
            }
        }

        let line = input.trim();

        // Exit commands
        if line.eq_ignore_ascii_case("exit") || line.eq_ignore_ascii_case("quit") {
            break;
        }

        // Empty line
        if line.is_empty() {
            continue;
        }

        // Handle special commands
        if line.eq_ignore_ascii_case("new") {
            program_lines.clear();
            println!("OK");
            continue;
        }

        if line.eq_ignore_ascii_case("list") {
            if program_lines.is_empty() {
                println!("(empty program)");
            } else {
                for line in &program_lines {
                    println!("{}", line);
                }
            }
            continue;
        }

        // SAVE command
        if line.to_lowercase().starts_with("save ") {
            let filename = line[5..].trim();
            if filename.is_empty() {
                println!("Usage: SAVE <filename>");
                continue;
            }

            let program = program_lines.join("\n");
            match fs::write(filename, program) {
                Ok(_) => println!("Program saved to {}", filename),
                Err(e) => eprintln!("Error saving file: {}", e),
            }
            continue;
        }

        // LOAD command
        if line.to_lowercase().starts_with("load ") {
            let filename = line[5..].trim();
            if filename.is_empty() {
                println!("Usage: LOAD <filename>");
                continue;
            }

            match fs::read_to_string(filename) {
                Ok(content) => {
                    program_lines.clear();
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            // Convert to uppercase to match BASIC behavior
                            program_lines.push(trimmed.to_ascii_uppercase());
                        }
                    }
                    // Sort lines by line number
                    program_lines.sort_by_key(|l| {
                        l.split_whitespace()
                            .next()
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0)
                    });
                    println!("Program loaded from {} ({} lines)", filename, program_lines.len());
                }
                Err(e) => eprintln!("Error loading file: {}", e),
            }
            continue;
        }

        if line.eq_ignore_ascii_case("run") {
            if program_lines.is_empty() {
                println!("(nothing to run)");
                continue;
            }

            let program = program_lines.join("\n");
            match BasicHarness::run_apple_ii_basic(
                binary_path.to_str().unwrap(),
                &program,
                50_000_000,
            ) {
                Ok(output) => {
                    if !output.trim().is_empty() {
                        println!("{}", output);
                    }
                }
                Err(e) => eprintln!("Execution error: {}", e),
            }
            continue;
        }

        // If line starts with a number, it's a program line
        if line.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            // Extract line number
            let line_num: u32 = line
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            // Remove existing line with same number
            program_lines.retain(|l: &String| {
                !l.starts_with(&format!("{} ", line_num))
                    && !l.starts_with(&format!("{}\t", line_num))
            });

            // Add new line in UPPERCASE (to match BASIC behavior)
            let uppercase_line = line.to_ascii_uppercase();
            program_lines.push(uppercase_line);
            program_lines.sort_by_key(|l| {
                l.split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            });

            println!("OK");
        } else {
            // Immediate mode is not supported - Apple II BASIC requires line numbers
            println!("?SYNTAX ERROR - LINE NUMBER REQUIRED");
            println!("(Immediate mode is not supported. Use line numbers, then RUN)");
        }
    }

    println!("\nGoodbye!");
    Ok(())
}
