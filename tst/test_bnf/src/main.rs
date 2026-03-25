// Test what the bnf crate supports for parsing MACRO-10 assembly.

use bnf::Grammar;

fn test(name: &str, grammar_str: &str, input: &str) {
    let grammar: Grammar = match grammar_str.parse() {
        Ok(g) => g,
        Err(e) => {
            println!("[FAIL] {name}: grammar parse error: {e}");
            return;
        }
    };
    let parser = match grammar.build_parser() {
        Ok(p) => p,
        Err(e) => {
            println!("[FAIL] {name}: build_parser error: {e}");
            return;
        }
    };
    let results: Vec<_> = parser.parse_input(input).collect();
    if results.is_empty() {
        println!("[FAIL] {name}: no parse for {:?}", input);
    } else {
        println!("[OK]   {name}: {} parse(s) for {:?}", results.len(), input);
    }
}

fn main() {
    // ── Test 1: left-recursive expr ───────────────────────────────────────
    test(
        "left-recursive expr 1+2+3",
        r#"<expr> ::= <expr> "+" <digit> | <digit>
<digit> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9""#,
        "1+2+3",
    );

    // ── Test 2: angle brackets as terminal literals ───────────────────────
    test(
        "angle bracket terminal <x>",
        r#"<group>  ::= "<" <letter> ">"
<letter> ::= "a" | "b" | "c" | "x" | "y" | "z""#,
        "<x>",
    );

    // ── Test 3: equate ────────────────────────────────────────────────────
    test(
        "equate REALIO=4",
        r#"<equate>  ::= <symbol> "=" <digits>
<symbol>  ::= <letter> | <letter> <symbol>
<digits>  ::= <digit> | <digit> <digits>
<letter>  ::= "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z"
<digit>   ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9""#,
        "REALIO=4",
    );

    // ── Test 4: conditional with angle-bracket body ───────────────────────
    test(
        "conditional IFE 0,<X=1>",
        r#"<conditional> ::= "IFE" " " <digits> "," "<" <body> ">"
<body>    ::= <simple> | <simple> <body>
<simple>  ::= <letter> | <digit> | "=" | " "
<digits>  ::= <digit> | <digit> <digits>
<letter>  ::= "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z"
<digit>   ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9""#,
        "IFE 0,<X=1>",
    );

    // ── Test 5: nested angle-bracket groups ───────────────────────────────
    test(
        "nested groups <<a>>",
        r#"<outer> ::= "<" <inner> ">"
<inner>  ::= "<" <letter> ">"
<letter> ::= "a" | "b" | "c""#,
        "<<a>>",
    );

    // ── Test 6: ambiguity — instruction vs macro call ─────────────────────
    {
        let grammar: Grammar = r#"<line>   ::= <instr> | <call>
<instr>  ::= <mnem> " " <digits>
<call>   ::= <symbol> " " <digits>
<mnem>   ::= "LDA" | "LDX" | "LDY" | "STA"
<symbol> ::= <letter> | <letter> <symbol>
<digits> ::= <digit> | <digit> <digits>
<letter> ::= "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z"
<digit>  ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9""#
            .parse()
            .unwrap();
        let parser = grammar.build_parser().unwrap();
        let n = parser.parse_input("LDA 100").count();
        println!("[INFO] ambiguous 'LDA 100': {n} parse(s) (expect 2: instr + call)");
    }

    // ── Test 7: performance ───────────────────────────────────────────────
    {
        let grammar: Grammar = r#"<stmt>   ::= <symbol> " " <digits>
<symbol> ::= <letter> | <letter> <symbol>
<digits> ::= <digit> | <digit> <digits>
<letter> ::= "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z"
<digit>  ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9""#
            .parse()
            .unwrap();
        let parser = grammar.build_parser().unwrap();
        let start = std::time::Instant::now();
        let n = parser.parse_input("STKEND 507").count();
        println!(
            "[INFO] perf 'STKEND 507': {n} parse(s) in {:?}",
            start.elapsed()
        );
    }

    // ── Test 8: tab escape in terminal ────────────────────────────────────
    {
        let g: Result<Grammar, _> = r#"<ws> ::= " " | "\t""#.parse();
        match g {
            Ok(grammar) => {
                let parser = grammar.build_parser().unwrap();
                let n = parser.parse_input("\t").count();
                println!("[INFO] tab terminal: {n} parse(s)");
            }
            Err(e) => println!("[INFO] tab terminal grammar error: {e}"),
        }
    }

    // ── Test 9: § separator (step4 line joining inside groups) ───────────
    test(
        "§ separator <A=1§B=2>",
        r#"<group>   ::= "<" <content> ">"
<content> ::= <stmt> | <stmt> "§" <content>
<stmt>    ::= <letter> "=" <digit>
<letter>  ::= "A" | "B" | "C" | "X" | "Y" | "Z"
<digit>   ::= "0" | "1" | "2" | "3" | "4""#,
        "<A=1§B=2>",
    );
}
