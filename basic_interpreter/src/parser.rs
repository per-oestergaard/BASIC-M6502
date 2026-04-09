//! Parser for Microsoft BASIC
//!
//! Parses tokens into an Abstract Syntax Tree (AST).

use crate::lexer::{Lexer, Token};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub lines: Vec<ProgramLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgramLine {
    pub line_number: u32,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Print(PrintStmt),
    Let(LetStmt),
    If(IfStmt),
    For(ForStmt),
    Next(NextStmt),
    Goto(GotoStmt),
    Gosub(GosubStmt),
    Return,
    Dim(DimStmt),
    Read(ReadStmt),
    Data(DataStmt),
    Restore(RestoreStmt),
    On(OnStmt),
    Def(DefStmt),
    End,
    Stop,
    Clear,
    New,
    Rem(String),
    Input(InputStmt),
    Poke(PokeStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt {
    pub items: Vec<PrintItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrintItem {
    Expr(Expr),
    Tab(Expr),
    Spc(Expr),
    Semicolon,
    Comma,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetStmt {
    pub var: VarRef,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_stmt: Vec<Statement>,
    pub else_stmt: Option<Vec<Statement>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub var: String,
    pub start: Expr,
    pub end: Expr,
    pub step: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NextStmt {
    pub var: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GotoStmt {
    pub target: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GosubStmt {
    pub target: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DimStmt {
    pub arrays: Vec<(String, Vec<Expr>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadStmt {
    pub vars: Vec<VarRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataStmt {
    pub values: Vec<DataValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RestoreStmt {
    pub line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnStmt {
    pub expr: Expr,
    pub kind: OnKind,
    pub targets: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnKind {
    Goto,
    Gosub,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DefStmt {
    pub name: String,
    pub param: String,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputStmt {
    pub prompt: Option<String>,
    pub vars: Vec<VarRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PokeStmt {
    pub addr: Expr,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VarRef {
    Simple(String),
    String(String),
    Array(String, Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    String(String),
    Variable(VarRef),
    FnCall(String, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    BinaryOp(Box<Expr>, BinaryOp, Box<Expr>),
    BuiltinFn(BuiltinFn),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinFn {
    Abs(Box<Expr>),
    Atn(Box<Expr>),
    Cos(Box<Expr>),
    Exp(Box<Expr>),
    Int(Box<Expr>),
    Log(Box<Expr>),
    Rnd(Box<Expr>),
    Sgn(Box<Expr>),
    Sin(Box<Expr>),
    Sqr(Box<Expr>),
    Tan(Box<Expr>),
    Asc(Box<Expr>),
    Chr(Box<Expr>),
    Left(Box<Expr>, Box<Expr>),
    Len(Box<Expr>),
    Mid(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Right(Box<Expr>, Box<Expr>),
    Str(Box<Expr>),
    Val(Box<Expr>),
    Fre(Box<Expr>),
    Pos(Box<Expr>),
    Peek(Box<Expr>),
}

pub fn parse(source: &str) -> Result<Program, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<Program, String> {
        let mut lines = Vec::new();

        while !self.is_at_end() && self.current() != &Token::Eof {
            // Skip empty lines
            if self.current() == &Token::Newline {
                self.advance();
                continue;
            }

            lines.push(self.parse_line()?);
        }

        // Sort lines by line number
        lines.sort_by_key(|l| l.line_number);

        Ok(Program { lines })
    }

    fn parse_line(&mut self) -> Result<ProgramLine, String> {
        // Line must start with a number
        let line_number = match self.current() {
            Token::Number(n) => {
                let num = *n as u32;
                self.advance();
                num
            }
            _ => return Err(format!("Expected line number, got {:?}", self.current())),
        };

        let mut statements = Vec::new();

        // Parse statements separated by colons
        loop {
            statements.push(self.parse_statement()?);

            if self.current() == &Token::Colon {
                self.advance();
            } else {
                break;
            }
        }

        // Expect newline or EOF
        if self.current() == &Token::Newline {
            self.advance();
        } else if self.current() != &Token::Eof {
            return Err(format!("Expected newline, got {:?}", self.current()));
        }

        Ok(ProgramLine {
            line_number,
            statements,
        })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        match self.current() {
            Token::Print => self.parse_print(),
            Token::Let => self.parse_let(),
            Token::If => self.parse_if(),
            Token::For => self.parse_for(),
            Token::Next => self.parse_next(),
            Token::Goto => self.parse_goto(),
            Token::Gosub => self.parse_gosub(),
            Token::Return => {
                self.advance();
                Ok(Statement::Return)
            }
            Token::Dim => self.parse_dim(),
            Token::Read => self.parse_read(),
            Token::Data => self.parse_data(),
            Token::Restore => self.parse_restore(),
            Token::On => self.parse_on(),
            Token::Def => self.parse_def(),
            Token::End => {
                self.advance();
                Ok(Statement::End)
            }
            Token::Stop => {
                self.advance();
                Ok(Statement::Stop)
            }
            Token::Clear => {
                self.advance();
                Ok(Statement::Clear)
            }
            Token::New => {
                self.advance();
                Ok(Statement::New)
            }
            Token::Rem => self.parse_rem(),
            Token::Input => self.parse_input(),
            Token::Poke => self.parse_poke(),
            Token::Identifier(_) | Token::StringVar(_) | Token::ArrayVar(_) => {
                // Implicit LET
                self.parse_let_implicit()
            }
            _ => Err(format!(
                "Unexpected token in statement: {:?}",
                self.current()
            )),
        }
    }

    fn parse_print(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip PRINT

        let mut items = Vec::new();

        // PRINT with no arguments
        if self.current() == &Token::Newline
            || self.current() == &Token::Colon
            || self.current() == &Token::Eof
        {
            return Ok(Statement::Print(PrintStmt { items }));
        }

        loop {
            // Check for separators
            if self.current() == &Token::Semicolon {
                items.push(PrintItem::Semicolon);
                self.advance();
            } else if self.current() == &Token::Comma {
                items.push(PrintItem::Comma);
                self.advance();
            } else if self.current() == &Token::Tab {
                self.advance();
                self.expect(&Token::LeftParen)?;
                let expr = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                items.push(PrintItem::Tab(expr));
            } else if self.current() == &Token::Spc {
                self.advance();
                self.expect(&Token::LeftParen)?;
                let expr = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                items.push(PrintItem::Spc(expr));
            } else {
                // Parse expression
                items.push(PrintItem::Expr(self.parse_expression()?));
            }

            // Check if we're done
            if self.current() == &Token::Newline
                || self.current() == &Token::Colon
                || self.current() == &Token::Eof
            {
                break;
            }
        }

        Ok(Statement::Print(PrintStmt { items }))
    }

    fn parse_let(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip LET
        self.parse_let_implicit()
    }

    fn parse_let_implicit(&mut self) -> Result<Statement, String> {
        let var = self.parse_var_ref()?;
        self.expect(&Token::Equal)?;
        let value = self.parse_expression()?;
        Ok(Statement::Let(LetStmt { var, value }))
    }

    fn parse_if(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip IF
        let condition = self.parse_expression()?;
        self.expect(&Token::Then)?;

        let mut then_stmt = Vec::new();
        let mut else_stmt = None;

        // Check if THEN is followed by a line number (GOTO)
        if let Token::Number(_) = self.current() {
            let target = self.parse_line_number()?;
            then_stmt.push(Statement::Goto(GotoStmt { target }));
        } else {
            // Parse THEN statements
            loop {
                then_stmt.push(self.parse_statement()?);

                if self.current() == &Token::Colon && self.peek() != Some(&Token::Else) {
                    self.advance();
                } else {
                    break;
                }
            }

            // Check for ELSE
            if self.current() == &Token::Colon {
                self.advance();
            }

            if self.current() == &Token::Else {
                self.advance();

                // Check if ELSE is followed by a line number
                if let Token::Number(_) = self.current() {
                    let target = self.parse_line_number()?;
                    else_stmt = Some(vec![Statement::Goto(GotoStmt { target })]);
                } else {
                    let mut stmts = Vec::new();
                    loop {
                        stmts.push(self.parse_statement()?);

                        if self.current() == &Token::Colon {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    else_stmt = Some(stmts);
                }
            }
        }

        Ok(Statement::If(IfStmt {
            condition,
            then_stmt,
            else_stmt,
        }))
    }

    fn parse_for(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip FOR

        let var = match self.current() {
            Token::Identifier(name) => {
                let v = name.clone();
                self.advance();
                v
            }
            _ => return Err("Expected variable name after FOR".to_string()),
        };

        self.expect(&Token::Equal)?;
        let start = self.parse_expression()?;
        self.expect(&Token::To)?;
        let end = self.parse_expression()?;

        let step = if self.current() == &Token::Step {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::For(ForStmt {
            var,
            start,
            end,
            step,
        }))
    }

    fn parse_next(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip NEXT

        let var = if let Token::Identifier(name) = self.current() {
            let v = name.clone();
            self.advance();
            Some(v)
        } else {
            None
        };

        Ok(Statement::Next(NextStmt { var }))
    }

    fn parse_goto(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip GOTO

        // Handle "GO TO" as two tokens
        if self.current() == &Token::To {
            self.advance();
        }

        let target = self.parse_line_number()?;
        Ok(Statement::Goto(GotoStmt { target }))
    }

    fn parse_gosub(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip GOSUB
        let target = self.parse_line_number()?;
        Ok(Statement::Gosub(GosubStmt { target }))
    }

    fn parse_dim(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip DIM

        let mut arrays = Vec::new();

        loop {
            let name = match self.current() {
                Token::ArrayVar(n) | Token::Identifier(n) => {
                    let name = n.clone();
                    self.advance();
                    name
                }
                _ => return Err("Expected array name in DIM".to_string()),
            };

            self.expect(&Token::LeftParen)?;

            let mut dims = Vec::new();
            loop {
                dims.push(self.parse_expression()?);

                if self.current() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }

            self.expect(&Token::RightParen)?;
            arrays.push((name, dims));

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Statement::Dim(DimStmt { arrays }))
    }

    fn parse_read(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip READ

        let mut vars = Vec::new();

        loop {
            vars.push(self.parse_var_ref()?);

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Statement::Read(ReadStmt { vars }))
    }

    fn parse_data(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip DATA

        let mut values = Vec::new();

        loop {
            match self.current() {
                Token::Number(n) => {
                    values.push(DataValue::Number(*n));
                    self.advance();
                }
                Token::String(s) => {
                    values.push(DataValue::String(s.clone()));
                    self.advance();
                }
                Token::Minus => {
                    self.advance();
                    if let Token::Number(n) = self.current() {
                        values.push(DataValue::Number(-*n));
                        self.advance();
                    } else {
                        return Err("Expected number after minus in DATA".to_string());
                    }
                }
                _ => {
                    // Try to parse as identifier/string (unquoted string in DATA)
                    if let Token::Identifier(s) = self.current() {
                        values.push(DataValue::String(s.clone()));
                        self.advance();
                    } else {
                        return Err(format!("Expected value in DATA, got {:?}", self.current()));
                    }
                }
            }

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Statement::Data(DataStmt { values }))
    }

    fn parse_restore(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip RESTORE

        let line = if let Token::Number(_) = self.current() {
            Some(self.parse_line_number()?)
        } else {
            None
        };

        Ok(Statement::Restore(RestoreStmt { line }))
    }

    fn parse_on(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip ON

        let expr = self.parse_expression()?;

        let kind = match self.current() {
            Token::Goto => {
                self.advance();
                OnKind::Goto
            }
            Token::Gosub => {
                self.advance();
                OnKind::Gosub
            }
            _ => return Err("Expected GOTO or GOSUB after ON".to_string()),
        };

        let mut targets = Vec::new();

        loop {
            targets.push(self.parse_line_number()?);

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Statement::On(OnStmt {
            expr,
            kind,
            targets,
        }))
    }

    fn parse_def(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip DEF

        // In Microsoft BASIC, syntax is: DEF FN<name>(<param>) = <expr>
        // The function name can be FNA, FNX, etc. - the FN is part of the name
        // So we need to handle both "FN A" and "FNA" patterns

        let name = if self.current() == &Token::Fn {
            // Pattern: DEF FN <identifier>
            self.advance();
            match self.current() {
                Token::Identifier(n) | Token::ArrayVar(n) => {
                    let name = n.clone();
                    self.advance();
                    name
                }
                _ => return Err("Expected function name after DEF FN".to_string()),
            }
        } else {
            // Pattern: DEF FN<identifier> (e.g., DEFNA parsed as one token)
            // Or function name starting with FN
            match self.current() {
                Token::Identifier(n) | Token::ArrayVar(n) => {
                    let name = n.clone();
                    self.advance();
                    name
                }
                _ => return Err("Expected FN or function name after DEF".to_string()),
            }
        };

        self.expect(&Token::LeftParen)?;

        let param = match self.current() {
            Token::Identifier(n) => {
                let param = n.clone();
                self.advance();
                param
            }
            _ => return Err("Expected parameter name in DEF FN".to_string()),
        };

        self.expect(&Token::RightParen)?;
        self.expect(&Token::Equal)?;

        let body = self.parse_expression()?;

        Ok(Statement::Def(DefStmt { name, param, body }))
    }

    fn parse_rem(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip REM

        // REM eats everything until end of line
        let comment = String::new();
        while self.current() != &Token::Newline && self.current() != &Token::Eof {
            // Reconstruct the comment (this is a simplification)
            self.advance();
        }

        Ok(Statement::Rem(comment))
    }

    fn parse_input(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip INPUT

        // Check for optional prompt string
        let prompt = if let Token::String(s) = self.current() {
            let p = s.clone();
            self.advance();

            // Expect semicolon or comma after prompt
            if self.current() == &Token::Semicolon || self.current() == &Token::Comma {
                self.advance();
            }

            Some(p)
        } else {
            None
        };

        let mut vars = Vec::new();

        loop {
            vars.push(self.parse_var_ref()?);

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Statement::Input(InputStmt { prompt, vars }))
    }

    fn parse_poke(&mut self) -> Result<Statement, String> {
        self.advance(); // Skip POKE

        let addr = self.parse_expression()?;
        self.expect(&Token::Comma)?;
        let value = self.parse_expression()?;

        Ok(Statement::Poke(PokeStmt { addr, value }))
    }

    fn parse_var_ref(&mut self) -> Result<VarRef, String> {
        match self.current() {
            Token::Identifier(name) => {
                let n = name.clone();
                self.advance();
                Ok(VarRef::Simple(n))
            }
            Token::StringVar(name) => {
                let n = name.clone();
                self.advance();
                Ok(VarRef::String(n))
            }
            Token::ArrayVar(name) => {
                let n = name.clone();
                self.advance();
                self.expect(&Token::LeftParen)?;

                let mut indices = Vec::new();
                loop {
                    indices.push(self.parse_expression()?);

                    if self.current() == &Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }

                self.expect(&Token::RightParen)?;
                Ok(VarRef::Array(n, indices))
            }
            _ => Err(format!(
                "Expected variable reference, got {:?}",
                self.current()
            )),
        }
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;

        while self.current() == &Token::Or {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp(Box::new(left), BinaryOp::Or, Box::new(right));
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_not()?;

        while self.current() == &Token::And {
            self.advance();
            let right = self.parse_not()?;
            left = Expr::BinaryOp(Box::new(left), BinaryOp::And, Box::new(right));
        }

        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, String> {
        if self.current() == &Token::Not {
            self.advance();
            let expr = self.parse_not()?;
            Ok(Expr::UnaryOp(UnaryOp::Not, Box::new(expr)))
        } else {
            self.parse_relational()
        }
    }

    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.current() {
                Token::Equal => BinaryOp::Eq,
                Token::NotEqual => BinaryOp::Ne,
                Token::Less => BinaryOp::Lt,
                Token::Greater => BinaryOp::Gt,
                Token::LessEqual => BinaryOp::Le,
                Token::GreaterEqual => BinaryOp::Ge,
                _ => break,
            };

            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.current() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };

            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_power()?;

        loop {
            let op = match self.current() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => break,
            };

            self.advance();
            let right = self.parse_power()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;

        if self.current() == &Token::Caret {
            self.advance();
            let right = self.parse_power()?; // Right associative
            left = Expr::BinaryOp(Box::new(left), BinaryOp::Pow, Box::new(right));
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.current() == &Token::Minus {
            self.advance();
            let expr = self.parse_unary()?;
            Ok(Expr::UnaryOp(UnaryOp::Neg, Box::new(expr)))
        } else if self.current() == &Token::Plus {
            self.advance();
            self.parse_unary()
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.current().clone() {
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expr::String(s))
            }
            Token::Identifier(_) | Token::StringVar(_) | Token::ArrayVar(_) => {
                let var = self.parse_var_ref()?;
                Ok(Expr::Variable(var))
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                Ok(expr)
            }
            Token::Fn => {
                self.advance();
                let name = match self.current() {
                    Token::Identifier(n) | Token::ArrayVar(n) => {
                        let name = n.clone();
                        self.advance();
                        name
                    }
                    _ => return Err("Expected function name after FN".to_string()),
                };
                self.expect(&Token::LeftParen)?;
                let arg = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                Ok(Expr::FnCall(name, Box::new(arg)))
            }
            // Builtin functions
            Token::Abs => self.parse_builtin_fn1(|e| BuiltinFn::Abs(e)),
            Token::Atn => self.parse_builtin_fn1(|e| BuiltinFn::Atn(e)),
            Token::Cos => self.parse_builtin_fn1(|e| BuiltinFn::Cos(e)),
            Token::Exp => self.parse_builtin_fn1(|e| BuiltinFn::Exp(e)),
            Token::Int => self.parse_builtin_fn1(|e| BuiltinFn::Int(e)),
            Token::Log => self.parse_builtin_fn1(|e| BuiltinFn::Log(e)),
            Token::Rnd => self.parse_builtin_fn1(|e| BuiltinFn::Rnd(e)),
            Token::Sgn => self.parse_builtin_fn1(|e| BuiltinFn::Sgn(e)),
            Token::Sin => self.parse_builtin_fn1(|e| BuiltinFn::Sin(e)),
            Token::Sqr => self.parse_builtin_fn1(|e| BuiltinFn::Sqr(e)),
            Token::Tan => self.parse_builtin_fn1(|e| BuiltinFn::Tan(e)),
            Token::Asc => self.parse_builtin_fn1(|e| BuiltinFn::Asc(e)),
            Token::Chr => self.parse_builtin_fn1(|e| BuiltinFn::Chr(e)),
            Token::Len => self.parse_builtin_fn1(|e| BuiltinFn::Len(e)),
            Token::Str => self.parse_builtin_fn1(|e| BuiltinFn::Str(e)),
            Token::Val => self.parse_builtin_fn1(|e| BuiltinFn::Val(e)),
            Token::Fre => self.parse_builtin_fn1(|e| BuiltinFn::Fre(e)),
            Token::Pos => self.parse_builtin_fn1(|e| BuiltinFn::Pos(e)),
            Token::Peek => self.parse_builtin_fn1(|e| BuiltinFn::Peek(e)),
            Token::Left => self.parse_left(),
            Token::Right => self.parse_right(),
            Token::Mid => self.parse_mid(),
            _ => Err(format!(
                "Unexpected token in expression: {:?}",
                self.current()
            )),
        }
    }

    fn parse_builtin_fn1<F>(&mut self, constructor: F) -> Result<Expr, String>
    where
        F: FnOnce(Box<Expr>) -> BuiltinFn,
    {
        self.advance();
        self.expect(&Token::LeftParen)?;
        let arg = self.parse_expression()?;
        self.expect(&Token::RightParen)?;
        Ok(Expr::BuiltinFn(constructor(Box::new(arg))))
    }

    fn parse_left(&mut self) -> Result<Expr, String> {
        self.advance();
        self.expect(&Token::LeftParen)?;
        let str_expr = self.parse_expression()?;
        self.expect(&Token::Comma)?;
        let len_expr = self.parse_expression()?;
        self.expect(&Token::RightParen)?;
        Ok(Expr::BuiltinFn(BuiltinFn::Left(
            Box::new(str_expr),
            Box::new(len_expr),
        )))
    }

    fn parse_right(&mut self) -> Result<Expr, String> {
        self.advance();
        self.expect(&Token::LeftParen)?;
        let str_expr = self.parse_expression()?;
        self.expect(&Token::Comma)?;
        let len_expr = self.parse_expression()?;
        self.expect(&Token::RightParen)?;
        Ok(Expr::BuiltinFn(BuiltinFn::Right(
            Box::new(str_expr),
            Box::new(len_expr),
        )))
    }

    fn parse_mid(&mut self) -> Result<Expr, String> {
        self.advance();
        self.expect(&Token::LeftParen)?;
        let str_expr = self.parse_expression()?;
        self.expect(&Token::Comma)?;
        let start_expr = self.parse_expression()?;

        let len_expr = if self.current() == &Token::Comma {
            self.advance();
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.expect(&Token::RightParen)?;
        Ok(Expr::BuiltinFn(BuiltinFn::Mid(
            Box::new(str_expr),
            Box::new(start_expr),
            len_expr,
        )))
    }

    fn parse_line_number(&mut self) -> Result<u32, String> {
        match self.current() {
            Token::Number(n) => {
                let num = *n as u32;
                self.advance();
                Ok(num)
            }
            _ => Err(format!("Expected line number, got {:?}", self.current())),
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek(&self) -> Option<&Token> {
        if self.pos + 1 < self.tokens.len() {
            Some(&self.tokens[self.pos + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        if self.current() == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, self.current()))
        }
    }
}
