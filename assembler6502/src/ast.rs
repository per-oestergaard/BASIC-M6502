/// AST (Abstract Syntax Tree) for 6502 assembly with macro support
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum AstNode {
    /// Label definition
    Label { name: String },
    
    /// Instruction with optional label
    Instruction {
        label: Option<String>,
        mnemonic: String,
        operand: Option<Operand>,
    },
    
    /// Directive (ORG, .byte, .word, .res)
    Directive {
        label: Option<String>,
        directive: Directive,
    },
    
    /// Equate (symbol = value)
    Equate {
        name: String,
        expr: Expr,
    },
    
    /// Macro definition
    MacroDef {
        name: String,
        params: Vec<String>,
        body: Vec<String>, // Raw body lines until we expand
    },
    
    /// Macro call
    MacroCall {
        label: Option<String>,
        name: String,
        args: Vec<String>,
    },
    
    /// Conditional block
    Conditional {
        kind: CondKind,
        condition: Expr,
        then_block: Vec<AstNode>,
        else_block: Vec<AstNode>,
    },
    
    /// Repeat block
    Repeat {
        label: Option<String>,
        count: usize,
        body: Vec<AstNode>,
    },
    
    /// Comment or empty line (can be discarded)
    Comment(String),
}

#[derive(Debug, Clone)]
pub enum Directive {
    Org(Expr),
    Byte(Vec<Expr>),
    Word(Vec<Expr>),
    Reserve(usize),
    Align(usize),
}

#[derive(Debug, Clone)]
pub enum ByteExpr {
    Value(Expr),
    String(String, bool), // string, high_bit_last (DCI)
}

#[derive(Debug, Clone)]
pub enum Operand {
    Immediate(Expr),
    Absolute(Expr),
    AbsoluteX(Expr),
    AbsoluteY(Expr),
    ZeroPage(Expr),
    ZeroPageX(Expr),
    ZeroPageY(Expr),
    Indirect(Expr),
    IndirectX(Expr),
    IndirectY(Expr),
    Relative(Expr),
    Accumulator,
}

#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal number
    Number(i64),
    
    /// Symbol reference
    Symbol(String),
    
    /// Binary operation
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    
    /// Unary operation
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    
    /// Low byte (<symbol)
    LowByte(Box<Expr>),
    
    /// High byte (>symbol)
    HighByte(Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    Xor,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum CondKind {
    IfEqual,    // IFE
    IfNotEqual, // IFN
    IfNotDef,   // IFNDEF
}

/// Context for parsing/preprocessing
pub struct ParseContext {
    pub symbols: HashMap<String, i64>,
    pub macros: HashMap<String, MacroDef>,
    pub config: Config,
}

#[derive(Debug, Clone)]
pub struct MacroDef {
    pub params: Vec<String>,
    pub body: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub realio: i64,
    pub apple: i64,
    pub kimrom: i64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            realio: 4, // Apple II
            apple: 1,
            kimrom: 0,
        }
    }
}

impl ParseContext {
    pub fn new() -> Self {
        let mut ctx = ParseContext {
            symbols: HashMap::new(),
            macros: HashMap::new(),
            config: Config::default(),
        };
        // Seed with config symbols
        ctx.symbols.insert("REALIO".to_string(), ctx.config.realio);
        ctx.symbols.insert("APPLE".to_string(), ctx.config.apple);
        ctx.symbols.insert("KIMROM".to_string(), ctx.config.kimrom);
        ctx
    }
}
