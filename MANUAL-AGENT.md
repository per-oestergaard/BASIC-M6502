
Disregard the programming approach. You cannot use programming languages, you cannot run commands in the terminal, only direct manipulation unless instructed so in the individual step. Do the following -

1. For each step below, create a new version of the previous file, giving it a useful name. Place files in ./temp. Pause after each step and ask me about the quality of the work and whether you should continue.
2. strip off block comments
3. strip off end of line comments (semicolon and everything after, there are no quoted semicolons)
4. we'll invent a statement seperator and use § for that. Convert all multi-line statements to single line statements by replacing newlines with §. This will ensure that all blocks (<>) are on a single line, and that all statements are on a single line. This will make it easier to parse the file in the next steps. For this create a new rust program called step3to4. Only handle this step in the program. It should be simple by just counting <> and replacing newlines with § when in a block.
5. Create a new rust program called step4to5 that expands conditional macros e.g either expand or remove content. Repeat process until no replacements are made. Ignore DEFINEs, only the conditional macros matter.
6. repeat next level of macros
7. repeat next level of macros
