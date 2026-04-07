# Kør Microsoft BASIC Interaktivt 🎮

Du kan nu køre BASIC interaktivt! Jeg har lavet en REPL (Read-Eval-Print Loop) med fil support.

## Start REPL

```bash
cargo run --release -p emu6502 --example interactive
```

## Kommandoer

- `10 PRINT "HI"` - Tilføj program linje (konverteres til UPPERCASE)
- `RUN` - Kør programmet
- `LIST` - Vis programmet
- `NEW` - Ryd programmet
- `SAVE <filename>` - Gem programmet til fil (i UPPERCASE)
- `LOAD <filename>` - Indlæs program fra fil
- `exit` - Afslut

**OBS:** Immediate mode (kommandoer uden linjenumre) virker ikke. Brug altid linjenumre.

## Quick Start Eksempel

```
] 10 PRINT "HELLO WORLD"
OK
] 20 FOR I=1 TO 5
OK
] 30 PRINT I;
OK
] 40 NEXT I
OK
] SAVE myprog.bas
Program saved to myprog.bas
] RUN
HELLO WORLD 1 2 3 4 5
] exit
Goodbye!
```

## Indlæs Eksisterende Programmer

Du kan indlæse alle test-programmerne:

```bash
cargo run --release -p emu6502 --example interactive
] LOAD tests/basic_programs/fibonacci_sequence.bas
Program loaded from tests/basic_programs/fibonacci_sequence.bas (8 lines)
] RUN
 0  1  1  2  3  5  8  13  21  34
```

Se mere i [emu6502/examples/INTERACTIVE.md](emu6502/examples/INTERACTIVE.md)
