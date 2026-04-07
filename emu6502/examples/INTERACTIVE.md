# Interactive Microsoft BASIC REPL

Kør den interaktive BASIC REPL:

```bash
cargo run --release -p emu6502 --example interactive
```

## Kommandoer

### Program Mode
- Skriv BASIC linjer med linjenumre: `10 PRINT "HELLO"`
- `RUN` - Kør programmet
- `LIST` - Vis programmet
- `NEW` - Ryd programmet
- `SAVE <filename>` - Gem programmet til fil
- `LOAD <filename>` - Indlæs program fra fil

### Immediate Mode
- ~~Skriv kommandoer uden linjenumre for direkte udførelse~~
- **VIRKER IKKE**: Immediate mode er ikke understøttet. Apple II BASIC kræver linjenumre.
- Brug i stedet: `10 PRINT 2+2` efterfulgt af `RUN`

### Afslut
- `exit` eller `quit` - Afslut REPL
- Ctrl-D - Afslut REPL (Unix/Linux/Mac)

## Eksempel Session 1: Simple Program

```
$ cargo run --release -p emu6502 --example interactive
Microsoft BASIC Interactive REPL
=================================
Type BASIC programs line by line.
Type 'RUN' to execute, 'LIST' to see code, 'NEW' to clear.
Type 'exit' or Ctrl-D to quit.

] 10 PRINT "HELLO WORLD"
OK
] 20 FOR I=1 TO 5
OK
] 30 PRINT I
OK
] 40 NEXT I
OK
] LIST
10 PRINT "HELLO WORLD"
20 FOR I=1 TO 5
30 PRINT I
40 NEXT I
] RUN
HELLO WORLD
 1
 2
 3
 4
 5
] exit
Goodbye!
```

## Eksempel Session 2: Fibonacci

```
] 10 A=0:B=1
OK
] 20 FOR I=1 TO 10
OK
] 30 PRINT A;
OK
] 40 C=A+B:A=B:B=C
OK
] 50 NEXT I
OK
] RUN
 0  1  1  2  3  5  8  13  21  34
] NEW
OK
] exit
Goodbye!
```

## Eksempel Session 3: String Manipulation

```
] 10 A$="HELLO"
OK
] 20 B$=" WORLD"
OK
] 30 PRINT A$+B$
OK
] 40 PRINT LEFT$(A$,3)
OK
] 50 PRINT LEN(A$+B$)
OK
] RUN
HELLO WORLD
HEL
 11
] exit
Goodbye!
```

## Eksempel Session 4: Gem og Indlæs

```
] 10 PRINT "MY PROGRAM"
OK
] 20 FOR I=1 TO 3
OK
] 30 PRINT "STEP";I
OK
] 40 NEXT I
OK
] SAVE myprogram.bas
Program saved to myprogram.bas
] NEW
OK
] LIST
(empty program)
] LOAD myprogram.bas
Program loaded from myprogram.bas (4 lines)
] LIST
10 PRINT "MY PROGRAM"
20 FOR I=1 TO 3
30 PRINT "STEP";I
40 NEXT I
] RUN
MY PROGRAM
STEP 1
STEP 2
STEP 3
```

## Tips

1. **Redigér linjer**: Bare skriv linje med samme nummer igen
   ```
   ] 10 PRINT "WRONG"
   OK
   ] 10 PRINT "CORRECT"
   OK
   ```

2. **Gem dit arbejde**: Brug SAVE for at gemme programmer
   ```
   ] SAVE fibonacci.bas
   Program saved to fibonacci.bas
   ```

3. **Indlæs fra fil**: Brug LOAD for at indlæse gemt program
   ```
   ] LOAD fibonacci.bas
   Program loaded from fibonacci.bas (5 lines)
   ```

4. **Fejlhåndtering**: Hvis der er fejl, vil programmet stoppe og vise fejlen

5. **Performance**: Release build er meget hurtigere end debug build!

## Begrænsninger

- **Immediate mode virker ikke**: Apple II BASIC kræver linjenumre. Brug `10 PRINT X` ikke bare `PRINT X`.
- Input via INPUT statement virker ikke i interaktiv mode
- Programmet startes forfra ved hver RUN
- Ingen historik/line editing (brug rlwrap for det)
- Program-linjer konverteres til UPPERCASE (autentisk BASIC opførsel)

## Avanceret: Med Line Editing

For bedre kommandolinje erfaring:

```bash
rlwrap cargo run --release -p emu6502 --example interactive
```

Dette giver dig:
- Pil op/ned for historik
- Ctrl-R for søgning
- Line editing med emacs/vi bindings
