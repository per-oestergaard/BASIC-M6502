//! Test that LOAD command converts file content to uppercase
//!
//! This test verifies that when loading a BASIC program from a file
//! with lowercase letters, it gets converted to uppercase as per
//! Microsoft BASIC behavior.

use std::env;
use std::fs;
use std::process::Command;

#[test]
fn test_load_converts_to_uppercase() {
    // Get workspace root
    let workspace_root = env::var("CARGO_MANIFEST_DIR")
        .map(|p| std::path::PathBuf::from(p).parent().unwrap().to_path_buf())
        .unwrap_or_else(|_| std::path::PathBuf::from(".."));

    let temp_dir = workspace_root.join("temp");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");

    let temp_file = temp_dir.join("test_load_uppercase.bas");

    // Write lowercase BASIC program
    let lowercase_program = "10 print \"hello world\"\n20 for i=1 to 3\n30 print i\n40 next i\n";

    fs::write(&temp_file, lowercase_program).expect("Failed to write test file");

    // Run interactive REPL with commands to LOAD and LIST using shell
    let commands = format!("load temp/test_load_uppercase.bas\nlist\nexit\n");

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", &format!("(echo load temp/test_load_uppercase.bas && echo list && echo exit) | cargo run --release -p emu6502 --example interactive")])
            .current_dir(&workspace_root)
            .output()
            .expect("Failed to execute command")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "printf '{}' | cargo run --release -p emu6502 --example interactive 2>&1",
                commands
            ))
            .current_dir(&workspace_root)
            .output()
            .expect("Failed to execute command")
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify the LIST output contains uppercase
    assert!(
        stdout.contains("10 PRINT \"HELLO WORLD\""),
        "Expected uppercase PRINT statement, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("20 FOR I=1 TO 3"),
        "Expected uppercase FOR statement, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("30 PRINT I"),
        "Expected uppercase PRINT I, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("40 NEXT I"),
        "Expected uppercase NEXT I, got:\n{}",
        stdout
    );

    // Should NOT contain any lowercase program lines
    assert!(
        !stdout.contains("print \"hello"),
        "Found lowercase 'print' in output - LOAD should convert to uppercase:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("for i="),
        "Found lowercase 'for' in output - LOAD should convert to uppercase:\n{}",
        stdout
    );

    println!("✓ LOAD correctly converts lowercase to uppercase");
}

#[test]
fn test_load_and_run_uppercase() {
    // Get workspace root
    let workspace_root = env::var("CARGO_MANIFEST_DIR")
        .map(|p| std::path::PathBuf::from(p).parent().unwrap().to_path_buf())
        .unwrap_or_else(|_| std::path::PathBuf::from(".."));

    let temp_dir = workspace_root.join("temp");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");

    let temp_file = temp_dir.join("test_load_run.bas");

    let mixed_case_program = "10 a$=\"test\"\n20 print a$\n30 print len(a$)\n";

    fs::write(&temp_file, mixed_case_program).expect("Failed to write test file");

    // Run interactive REPL with LOAD, RUN
    let commands = "load temp/test_load_run.bas\nrun\nexit\n";

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", "(echo load temp/test_load_run.bas && echo run && echo exit) | cargo run --release -p emu6502 --example interactive"])
            .current_dir(&workspace_root)
            .output()
            .expect("Failed to execute command")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "printf '{}' | cargo run --release -p emu6502 --example interactive 2>&1",
                commands
            ))
            .current_dir(&workspace_root)
            .output()
            .expect("Failed to execute command")
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify program executes and outputs uppercase string
    assert!(
        stdout.contains("TEST"),
        "Expected 'TEST' in output (uppercase conversion), got:\n{}",
        stdout
    );
    assert!(stdout.contains(" 4"), "Expected length 4, got:\n{}", stdout);

    println!("✓ LOAD and RUN correctly handles uppercase conversion");
}
