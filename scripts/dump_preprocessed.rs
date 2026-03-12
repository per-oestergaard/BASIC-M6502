use assembler6502::{Assembler,AsmOptions,preprocess};
use std::fs;
fn main(){
 let src=fs::read_to_string("m6502.asm").unwrap();
 let pre=preprocess::run(&src);
 for (i,l) in pre.lines.iter().enumerate(){
   if l.starts_with("IFN") || l.starts_with("IFE") { println!("{}: {}", i+1, l); }
 }
}
