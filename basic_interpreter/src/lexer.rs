//! Lexer for Microsoft BASIC
//!
//! Tokenizes BASIC source code into a stream of tokens.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Number(f64),
    String(String),

    // Identifiers
    Identifier(String), // Variable names: A, X1, COUNT
    StringVar(String),  // String variables: A$, NAME$
    ArrayVar(String),   // Array variables: A(, B%(

    // Keywords
    Print,
    Let,
    If,
    Then,
    Else,
    For,
    To,
    Step,
    Next,
    Goto,
    Gosub,
    Return,
    Dim,
    Read,
    Data,
    Restore,
    On,
    Def,
    Fn,
    End,
    Stop,
    Clear,
    New,
    Rem,
    Input,

    // Functions
    Abs,
    Atn,
    Cos,
    Exp,
    Int,
    Log,
    Rnd,
    Sgn,
    Sin,
    Sqr,
    Tan,
    Asc,
    Chr,
    Left,
    Len,
    Mid,
    Right,
    Str,
    Val,
    Tab,
    Spc,
    Fre,
    Pos,
    Peek,
    Poke,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    Not,

    // Punctuation
    LeftParen,
    RightParen,
    Comma,
    Semicolon,
    Colon,

    // Special
    Newline,
    Eof,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();

            if self.is_at_end() {
                tokens.push(Token::Eof);
                break;
            }

            let token = self.next_token()?;
            tokens.push(token.clone());

            if token == Token::Eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && self.current().is_whitespace() && self.current() != '\n' {
            self.advance();
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn current(&self) -> char {
        self.input[self.pos]
    }

    fn advance(&mut self) -> char {
        let ch = self.current();
        self.pos += 1;
        ch
    }

    fn peek(&self) -> Option<char> {
        if self.pos + 1 < self.input.len() {
            Some(self.input[self.pos + 1])
        } else {
            None
        }
    }

    fn next_token(&mut self) -> Result<Token, String> {
        if self.is_at_end() {
            return Ok(Token::Eof);
        }

        let ch = self.current();

        // Newline
        if ch == '\n' {
            self.advance();
            return Ok(Token::Newline);
        }

        // Numbers
        if ch.is_ascii_digit()
            || (ch == '.' && self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false))
        {
            return self.read_number();
        }

        // Strings
        if ch == '"' {
            return self.read_string();
        }

        // Identifiers and keywords
        if ch.is_alphabetic() {
            return self.read_identifier_or_keyword();
        }

        // Operators and punctuation
        self.advance();
        match ch {
            '+' => Ok(Token::Plus),
            '-' => Ok(Token::Minus),
            '*' => Ok(Token::Star),
            '/' => Ok(Token::Slash),
            '^' => Ok(Token::Caret),
            '=' => Ok(Token::Equal),
            '<' => {
                if !self.is_at_end() && self.current() == '>' {
                    self.advance();
                    Ok(Token::NotEqual)
                } else if !self.is_at_end() && self.current() == '=' {
                    self.advance();
                    Ok(Token::LessEqual)
                } else {
                    Ok(Token::Less)
                }
            }
            '>' => {
                if !self.is_at_end() && self.current() == '=' {
                    self.advance();
                    Ok(Token::GreaterEqual)
                } else {
                    Ok(Token::Greater)
                }
            }
            '(' => Ok(Token::LeftParen),
            ')' => Ok(Token::RightParen),
            ',' => Ok(Token::Comma),
            ';' => Ok(Token::Semicolon),
            ':' => Ok(Token::Colon),
            _ => Err(format!("Unexpected character: '{}'", ch)),
        }
    }

    fn read_number(&mut self) -> Result<Token, String> {
        let mut num_str = String::new();

        while !self.is_at_end() && (self.current().is_ascii_digit() || self.current() == '.') {
            num_str.push(self.advance());
        }

        // Handle scientific notation (e.g., 1.5E10)
        if !self.is_at_end() && (self.current() == 'E' || self.current() == 'e') {
            num_str.push(self.advance());
            if !self.is_at_end() && (self.current() == '+' || self.current() == '-') {
                num_str.push(self.advance());
            }
            while !self.is_at_end() && self.current().is_ascii_digit() {
                num_str.push(self.advance());
            }
        }

        num_str
            .parse::<f64>()
            .map(Token::Number)
            .map_err(|e| format!("Invalid number: {}", e))
    }

    fn read_string(&mut self) -> Result<Token, String> {
        self.advance(); // Skip opening quote
        let mut s = String::new();

        while !self.is_at_end() && self.current() != '"' {
            s.push(self.advance());
        }

        if self.is_at_end() {
            return Err("Unterminated string".to_string());
        }

        self.advance(); // Skip closing quote
        Ok(Token::String(s))
    }

    fn read_identifier_or_keyword(&mut self) -> Result<Token, String> {
        let mut name = String::new();

        while !self.is_at_end() && (self.current().is_alphanumeric() || self.current() == '_') {
            name.push(self.advance().to_ascii_uppercase());
        }

        // Check if it's a keyword FIRST
        let token = match name.as_str() {
            "PRINT" => Token::Print,
            "LET" => Token::Let,
            "IF" => Token::If,
            "THEN" => Token::Then,
            "ELSE" => Token::Else,
            "FOR" => Token::For,
            "TO" => Token::To,
            "STEP" => Token::Step,
            "NEXT" => Token::Next,
            "GOTO" | "GO" => Token::Goto,
            "GOSUB" => Token::Gosub,
            "RETURN" => Token::Return,
            "DIM" => Token::Dim,
            "READ" => Token::Read,
            "DATA" => Token::Data,
            "RESTORE" => Token::Restore,
            "ON" => Token::On,
            "DEF" => Token::Def,
            "FN" => Token::Fn,
            "END" => Token::End,
            "STOP" => Token::Stop,
            "CLEAR" => Token::Clear,
            "NEW" => Token::New,
            "REM" => Token::Rem,
            "INPUT" => Token::Input,
            "ABS" => Token::Abs,
            "ATN" => Token::Atn,
            "COS" => Token::Cos,
            "EXP" => Token::Exp,
            "INT" => Token::Int,
            "LOG" => Token::Log,
            "RND" => Token::Rnd,
            "SGN" => Token::Sgn,
            "SIN" => Token::Sin,
            "SQR" => Token::Sqr,
            "TAN" => Token::Tan,
            "ASC" => Token::Asc,
            "CHR" => {
                // CHR$ requires the $
                if !self.is_at_end() && self.current() == '$' {
                    self.advance();
                }
                Token::Chr
            }
            "LEFT" => {
                // LEFT$ requires the $
                if !self.is_at_end() && self.current() == '$' {
                    self.advance();
                }
                Token::Left
            }
            "LEN" => Token::Len,
            "MID" => {
                // MID$ requires the $
                if !self.is_at_end() && self.current() == '$' {
                    self.advance();
                }
                Token::Mid
            }
            "RIGHT" => {
                // RIGHT$ requires the $
                if !self.is_at_end() && self.current() == '$' {
                    self.advance();
                }
                Token::Right
            }
            "STR" => {
                // STR$ requires the $
                if !self.is_at_end() && self.current() == '$' {
                    self.advance();
                }
                Token::Str
            }
            "VAL" => Token::Val,
            "TAB" => Token::Tab,
            "SPC" => Token::Spc,
            "FRE" => Token::Fre,
            "POS" => Token::Pos,
            "PEEK" => Token::Peek,
            "POKE" => Token::Poke,
            "AND" => Token::And,
            "OR" => Token::Or,
            "NOT" => Token::Not,
            _ => {
                // Not a keyword - check for string variable or array
                if !self.is_at_end() && self.current() == '$' {
                    self.advance();
                    return Ok(Token::StringVar(name));
                } else if !self.is_at_end() && self.current() == '(' {
                    return Ok(Token::ArrayVar(name));
                } else {
                    return Ok(Token::Identifier(name));
                }
            }
        };

        Ok(token)
    }
}
