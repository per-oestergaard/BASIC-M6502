[0m[1m[33mwarning[0m[0m[1m: function `parse_word_token` is never used[0m
[0m  [0m[0m[1m[38;5;12m--> [0m[0massembler6502/src/parse.rs:90:4[0m
[0m   [0m[0m[1m[38;5;12m|[0m
[0m[1m[38;5;12m90[0m[0m [0m[0m[1m[38;5;12m|[0m[0m [0m[0mfn parse_word_token(s:&str)->Option<u16>{[0m
[0m   [0m[0m[1m[38;5;12m|[0m[0m    [0m[0m[1m[33m^^^^^^^^^^^^^^^^[0m
[0m   [0m[0m[1m[38;5;12m|[0m
[0m   [0m[0m[1m[38;5;12m= [0m[0m[1mnote[0m[0m: `#[warn(dead_code)]` on by default[0m

[0m[1m[33mwarning[0m[0m[1m: field `ctx` is never read[0m
[0m [0m[0m[1m[38;5;12m--> [0m[0massembler6502/src/parser.rs:9:5[0m
[0m  [0m[0m[1m[38;5;12m|[0m
[0m[1m[38;5;12m6[0m[0m [0m[0m[1m[38;5;12m|[0m[0m [0m[0mpub struct Parser {[0m
[0m  [0m[0m[1m[38;5;12m|[0m[0m            [0m[0m[1m[38;5;12m------[0m[0m [0m[0m[1m[38;5;12mfield in this struct[0m
[0m[1m[38;5;12m...[0m
[0m[1m[38;5;12m9[0m[0m [0m[0m[1m[38;5;12m|[0m[0m [0m[0m    ctx: ParseContext,[0m
[0m  [0m[0m[1m[38;5;12m|[0m[0m     [0m[0m[1m[33m^^^[0m

[1m[33mwarning[0m[1m:[0m `assembler6502` (lib) generated 2 warnings
[1m[32m   Compiling[0m assembler6502 v0.1.0 (/workspaces/BASIC-M6502/assembler6502)
[1m[32m    Finished[0m ]8;;https://doc.rust-lang.org/cargo/reference/profiles.html#default-profiles\`dev` profile [unoptimized + debuginfo]]8;;\ target(s) in 18.43s
[1m[32m     Running[0m `target/debug/dump_pre`
