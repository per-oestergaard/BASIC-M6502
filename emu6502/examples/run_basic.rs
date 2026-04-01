use emu6502::BasicHarness;
use std::fs;
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn resolve_program_path(arg: &str) -> PathBuf {
    let input = PathBuf::from(arg);
    if input.exists() {
        return input;
    }

    let root = workspace_root();
    let bas_name = if arg.ends_with(".bas") {
        arg.to_string()
    } else {
        format!("{arg}.bas")
    };
    root.join("tests/basic_programs").join(bas_name)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let mut args = std::env::args().skip(1);
    let program_arg = args.next().unwrap_or_else(|| "hello".to_string());
    let program_path = resolve_program_path(&program_arg);
    let binary_path = workspace_root().join("build/original/basic.bin");

    let program = fs::read_to_string(&program_path)?;
    let output = BasicHarness::run_apple_ii_basic(
        binary_path
            .to_str()
            .ok_or("interpreter binary path is not valid UTF-8")?,
        &program,
        50_000_000,
    )
    .map_err(std::io::Error::other)?;

    println!("{output}");
    Ok(())
}