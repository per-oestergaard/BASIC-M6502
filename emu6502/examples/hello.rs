use emu6502::BasicHarness;

fn main(){
    let mut h = BasicHarness::new();
    let program = [0xA9,b'H',0x8D,0x02,0x00,0xA9,b'I',0x8D,0x02,0x00,0x00];
    h.load_and_run(0x0800,&program,10_000).unwrap();
    println!("Captured: {}", h.output_string());
}