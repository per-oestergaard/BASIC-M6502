/// Parse: gather logical statements from the normalized source.
///
/// Each "logical statement" is one or more consecutive lines forming a
/// single syntactic unit — specifically a conditional, macro definition,
/// or any statement whose body is enclosed in `<…>`.
///
/// The parser tracks `<>`-depth to know when a statement is complete.
/// The result is a list of complete-statement strings fed to the bnf grammar.

pub struct StatementIter<'a> {
    remaining: &'a str,
}

impl<'a> StatementIter<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { remaining: src }
    }
}

impl<'a> Iterator for StatementIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        // Skip blank lines
        self.remaining = self.remaining.trim_start_matches('\n');
        if self.remaining.is_empty() {
            return None;
        }

        let mut depth: i32 = 0;
        let mut end = 0;

        for (i, c) in self.remaining.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => {
                    if depth > 0 { depth -= 1; }
                }
                '\n' => {
                    if depth == 0 {
                        // Statement ends at this newline
                        let stmt = &self.remaining[..i];
                        self.remaining = &self.remaining[i + 1..];
                        let stmt = stmt.trim();
                        if stmt.is_empty() {
                            // blank line inside iteration — recurse
                            return self.next();
                        }
                        return Some(stmt);
                    }
                    // depth > 0: newline is inside a block — continue
                }
                _ => {}
            }
            end = i + c.len_utf8();
        }

        // Reached end of input without a final newline
        let stmt = self.remaining[..end].trim();
        self.remaining = "";
        if stmt.is_empty() { None } else { Some(stmt) }
    }
}

/// A complete parsed statement — the raw text plus the keyword.
#[derive(Debug, Clone)]
pub struct Statement<'a> {
    pub text: &'a str,
    pub keyword: &'a str,
}

impl<'a> Statement<'a> {
    pub fn from(text: &'a str) -> Self {
        let keyword = text.split_ascii_whitespace().next().unwrap_or("");
        // Strip trailing ':' for label-lines
        let keyword = keyword.trim_end_matches(':');
        Self { text, keyword }
    }
}
