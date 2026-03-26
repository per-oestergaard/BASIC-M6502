/// Owned AST types: SourceNode (pre-expansion) and FlatStmt (post-expansion).
pub mod ast;

/// Preprocessing: strip opaque COMMENT blocks and `;` inline comments.
/// This is the ONLY pre-parse step — no `§` joining, no manual expansion.
pub mod preprocess;

/// Statement reader: collect multi-line `<>` blocks into single logical units.
pub mod reader;

/// Symbol table and arithmetic expression evaluator.
pub mod symbols;

/// bnf-grammar-based parser: parses logical statements into parse trees.
pub mod parser;

/// Conditional evaluator and macro expander: walks parse trees.
pub mod expander;

/// Two-pass 6502 assembler: resolves labels and emits bytes.
pub mod assemble;

/// 6502 opcode table.
pub mod opcode;
