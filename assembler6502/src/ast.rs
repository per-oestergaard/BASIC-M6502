/// AST types for Stage 1 (parser output) and Stage 2 (expander output).

// ---------------------------------------------------------------------------
// Stage 1 — SourceNode
// Produced by parser.rs. Purely structural: no expression evaluation,
// no symbol lookups. Nested constructs own their child nodes.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SourceNode {
    /// A bare label on a line (or with nothing else following).
    Label { name: String },

    /// A numeric equate: `SYM = expr` or `SYM == expr`.
    Equate { name: String, expr: String },

    /// ORG directive: `ORG expr`.
    Org { expr: String },

    /// IFE / IFN / IFNDEF / IF1 / IF2 conditional block.
    Conditional {
        kind: CondKind,
        /// The raw expression text (unevaluated).
        expr: String,
        then_body: Vec<SourceNode>,
        else_body: Vec<SourceNode>,
    },

    /// DEFINE macro definition.
    /// Body is stored as raw text lines because parameter substitution must
    /// happen before the body can be parsed.
    MacroDef {
        name: String,
        params: Vec<String>,
        /// Raw body lines (not yet parsed into SourceNodes).
        body: Vec<String>,
    },

    /// A macro invocation (opcode field matched a known or forward-declared macro name).
    MacroCall {
        label: Option<String>,
        name: String,
        /// Single raw argument string (may be empty).
        arg: Option<String>,
    },

    /// REPEAT n,<body> — inline unrolling.
    Repeat {
        label: Option<String>,
        count_expr: String,
        body: Vec<SourceNode>,
    },

    /// A real 6502 instruction or pseudo-op.
    Instr {
        label: Option<String>,
        /// Always upper-cased.
        mnemonic: String,
        /// Everything after the mnemonic on the same line, trimmed.
        operand: Option<String>,
    },

    /// `.byte` directive.
    Bytes { label: Option<String>, args: Vec<String> },

    /// `.word` / `ADR` / `XWD` directive.
    Word { label: Option<String>, args: Vec<String> },

    /// `.res` / `BLOCK` directive.
    Res { label: Option<String>, count_expr: String },
}

// ---------------------------------------------------------------------------
// Stage 2 — FlatStmt
// Produced by expander.rs. Completely flat: no conditionals, no macro defs,
// no REPEAT constructs. All pseudo-ops are expanded. Ready for two-pass
// assembly.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum FlatStmt {
    Label(String),

    Instr {
        label: Option<String>,
        mnemonic: String,
        operand: Option<String>,
    },

    Org(u16),

    /// Byte values are kept as expression strings so the assembler can resolve
    /// symbol references in pass 2.
    Bytes { label: Option<String>, values: Vec<String> },

    /// Word value (2-byte LE). Kept as expression string.
    Word { label: Option<String>, expr: String },

    /// Reserve `count` bytes.
    Res { label: Option<String>, count: u16 },

    /// Numeric equate already evaluated.
    Equate { name: String, value: i64 },
}

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondKind {
    /// IFE — assemble body when expression == 0.
    IfEq,
    /// IFN — assemble body when expression != 0.
    IfNe,
    /// IFNDEF — assemble body when symbol is not defined.
    IfNotDef,
    /// IF1 — first pass only (treated as always-true).
    If1,
    /// IF2 — second pass only (treated as always-false / skip).
    If2,
}
