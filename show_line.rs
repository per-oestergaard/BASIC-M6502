use assembler6502::preprocess_run;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let line_num: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(26);
    
    let src = fs::read_to_string("m6502.asm").unwrap();
    let pre = preprocess_run(&src);
    
    if line_num > 0 && line_num <= pre.lines.len() {
        println!("Line {}: {}", line_num, pre.lines[line_num - 1]);
    }
}
