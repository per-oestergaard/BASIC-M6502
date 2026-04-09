use basic_interpreter::interpreter::Interpreter;
use basic_interpreter::parser;
use std::fs;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Rust BASIC Interpreter - Interactive REPL");
    println!("==========================================");
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
                    println!(
                        "Program loaded from {} ({} lines)",
                        filename,
                        program_lines.len()
                    );
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

            let program_text = program_lines.join("\n");

            // Parse and run the program
            let program = match parser::parse(&program_text) {
                Ok(program) => program,
                Err(e) => {
                    eprintln!("Parse error: {}", e);
                    continue;
                }
            };

            let mut interpreter = Interpreter::new();
            match interpreter.run(&program) {
                Ok(output) => {
                    if !output.trim().is_empty() {
                        print!("{}", output);
                    }
                }
                Err(e) => eprintln!("Runtime error: {}", e),
            }
            continue;
        }

        // If line starts with a number, it's a program line
        if line
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            // Extract line number
            let line_no = line
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u32>().ok());

            if let Some(line_no) = line_no {
                // Remove any existing line with this number
                program_lines.retain(|l| {
                    l.split_whitespace()
                        .next()
                        .and_then(|s| s.parse::<u32>().ok())
                        != Some(line_no)
                });

                // Add new line if it's not just a number (deletion)
                if line.split_whitespace().count() > 1 {
                    program_lines.push(line.to_ascii_uppercase());
                }

                // Keep sorted by line number
                program_lines.sort_by_key(|l| {
                    l.split_whitespace()
                        .next()
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(0)
                });
            } else {
                println!("Invalid line number");
            }
            continue;
        }

        // Try to execute as immediate command (wrap in line 10)
        let immediate_program = format!("10 {}", line);
        let program = match parser::parse(&immediate_program) {
            Ok(program) => program,
            Err(e) => {
                eprintln!("Parse error: {}", e);
                continue;
            }
        };

        let mut interpreter = Interpreter::new();
        match interpreter.run(&program) {
            Ok(output) => {
                if !output.trim().is_empty() {
                    print!("{}", output);
                }
            }
            Err(e) => eprintln!("Runtime error: {}", e),
        }
    }

    println!("\nGoodbye!");
    Ok(())
}
