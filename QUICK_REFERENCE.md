# BASIC REPL - Quick Reference

## Start

```bash
cargo run --release -p emu6502 --example interactive
```

## Kommandoer

| Kommando | Beskrivelse | Eksempel |
|----------|-------------|----------|
| `10 PRINT "HI"` | Tilføj linje til program (→ UPPERCASE) | `10 PRINT "HELLO"` |
| `RUN` | Kør programmet | `RUN` |
| `LIST` | Vis programmet (i UPPERCASE) | `LIST` |
| `NEW` | Ryd programmet | `NEW` |
| `SAVE <fil>` | Gem program til fil (i UPPERCASE) | `SAVE myprog.bas` |
| `LOAD <fil>` | Indlæs program fra fil | `LOAD myprog.bas` |
| `exit` eller `quit` | Afslut REPL | `exit` |
| Ctrl-D | Afslut REPL | |

**Note:** Immediate mode (uden linjenumre) virker ikke. Brug altid linjenumre.

## Hurtig Demo

```bash
# Start REPL
cargo run --release -p emu6502 --example interactive

# Skriv et program
] 10 PRINT "HELLO WORLD"
] 20 FOR I=1 TO 5
] 30 PRINT I
] 40 NEXT I

# Gem det
] SAVE hello.bas

# Kør det
] RUN

# Indlæs et eksisterende program
] LOAD tests/basic_programs/fibonacci_sequence.bas
] RUN
```

## Alle Test-Programmer

Du kan indlæse enhver af de 76 test-programmer:

```bash
] LOAD tests/basic_programs/hello.bas
] LOAD tests/basic_programs/for_loop.bas
] LOAD tests/basic_programs/fibonacci_sequence.bas
] LOAD tests/basic_programs/string_sort.bas
] LOAD tests/basic_programs/nested_for.bas
# ... osv
```

Liste alle tilgængelige:
```bash
ls tests/basic_programs/*.bas
```

## Tips

- **Redigér linje**: Skriv samme linjenummer igen
- **Performance**: Brug `--release` for hurtig kørsel
- **Line editing**: Installer `rlwrap` for historik og editing:
  ```bash
  rlwrap cargo run --release -p emu6502 --example interactive
  ```

## Flere Eksempler

Se [emu6502/examples/INTERACTIVE.md](../emu6502/examples/INTERACTIVE.md) for:
- Fibonacci eksempel
- String manipulation
- Komplette program eksempler
- Fejlfinding tips
