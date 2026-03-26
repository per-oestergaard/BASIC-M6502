/// Owned AST types produced by the parser and consumed by the expander.

// ---------------------------------------------------------------------------
// CondKind
// ---------------------------------------------------------------------------

/// Discriminant for IFE / IFN / IFNDEF / IF1 / IF2 conditionals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CondKind {
    /// IFE — branch when expression equals zero.
    IfEq,
    /// IFN — branch when expression is non-zero.
    IfNe,
    /// IFNDEF — branch when symbol is not defined.
    IfNotDef,
    /// IF1 — always take (first-pass semantics; we treat as always-true).
    If1,
    /// IF2 — never take (second-pass semantics; we treat as always-false).
    If2,
}

// ---------------------------------------------------------------------------
// SourceNode — the parse-time (pre-expansion) AST
// ---------------------------------------------------------------------------

/// A single logical statement in the source, as produced by [`crate::parser::parse`].
#[derive(Debug, Clone)]
pub enum SourceNode {
    /// A standalone label definition: `LABEL:` or `LABEL::`.
    Label { name: String },

    /// An equate: `NAME = EXPR` or `NAME == EXPR`.
    Equate { name: String, expr: String },

    /// An ORG directive: `ORG EXPR`.
    Org { expr: String },

    /// A DEFINE macro definition.
    MacroDef {
        name: String,
        params: Vec<String>,
        /// Raw body lines (reparsed on each expansion after arg substitution).
        body: Vec<String>,
    },

    /// A conditional block (IFE / IFN / IFNDEF / IF1 / IF2).
    Conditional {
        kind: CondKind,
        /// The expression string (or symbol name for IFNDEF).
        expr: String,
        then_body: Vec<SourceNode>,
        else_body: Vec<SourceNode>,
    },

    /// A REPEAT block.
    Repeat {
        label: Option<String>,
        count_expr: String,
        body: Vec<SourceNode>,
    },

    /// A macro invocation or unrecognised mnemonic.
    MacroCall {
        label: Option<String>,
        name: String,
        arg: Option<String>,
    },

    /// Raw byte data (DC / DCI / BYTE / DB / …).
    Bytes {
        label: Option<String>,
        args: Vec<String>,
    },

    /// 16-bit word data (WORD / DW / …).
    Word {
        label: Option<String>,
        args: Vec<String>,
    },

    /// Reserved storage (BLKB / BLKW / BLOCK / RES / …).
    Res {
        label: Option<String>,
        count_expr: String,
    },

    /// A 6502 instruction or pseudo-op (expanded by the expander).
    Instr {
        label: Option<String>,
        mnemonic: String,
        operand: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// FlatStmt — the post-expansion, pre-assembly AST
// ---------------------------------------------------------------------------

/// A fully expanded statement, ready for the two-pass assembler.
#[derive(Debug, Clone)]
pub enum FlatStmt {
    /// Label definition.
    Label(String),

    /// Compile-time numeric equate (value already evaluated).
    Equate { name: String, value: i64 },

    /// Origin change.
    Org(u16),

    /// Byte data emission.
    Bytes {
        label: Option<String>,
        values: Vec<String>,
    },

    /// 16-bit word emission (one word per statement).
    Word {
        label: Option<String>,
        expr: String,
    },

    /// Reserved (zero-filled) storage.
    Res {
        label: Option<String>,
        count: u16,
    },

    /// 6502 instruction (real mnemonic + optional operand string).
    Instr {
        label: Option<String>,
        mnemonic: String,
        operand: Option<String>,
    },
}
