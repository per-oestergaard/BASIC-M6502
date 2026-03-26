use tracing::trace;

/// Strip multi-line COMMENT blocks BEFORE any other processing.
///
/// COMMENT blocks are fully opaque — their body may contain ";", "<", ">" and
/// every other special character so they MUST be removed before comment
/// stripping or angle-bracket counting.
///
/// Syntax:
///   COMMENT <delim>  [rest of opening line — ignored]
///   ... any number of body lines (opaque) ...
///   <delim>          ← same delimiter character ALONE on its own line
///
/// <delim> = first non-whitespace character following "COMMENT".
pub fn strip_block_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut lines = src.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        let upper = trimmed.to_ascii_uppercase();
        if upper.starts_with("COMMENT") {
            let after = trimmed["COMMENT".len()..].trim_start();
            if after.is_empty() { continue; }
            let delim = after.chars().next().map(|c| c.to_string()).unwrap_or_default();
            trace!(target: "assembler6502::preprocess", "COMMENT block delim={:?}", delim);
            for body in lines.by_ref() {
                if body.trim() == delim {
                    trace!(target: "assembler6502::preprocess", "COMMENT block end");
                    break;
                }
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Strip inline `;` comments — everything from `;` to end-of-line.
///
/// There are no quoted semicolons in m6502.asm, so `;` always starts a comment.
/// Must be called AFTER strip_block_comments (COMMENT bodies can contain `;`).
pub fn strip_inline_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        let end = line.find(';').unwrap_or(line.len());
        out.push_str(&line[..end]);
        out.push('\n');
    }
    out
}

/// Run both pre-parse normalisation steps and return the result.
///
/// The output is suitable to feed directly to the bnf-based parser.
/// No `§` joining or conditional evaluation happens here — that is the
/// parser's job.
pub fn normalize(src: &str) -> String {
    let s1 = strip_block_comments(src);
    trace!(target: "assembler6502::preprocess",
        "after strip_block_comments: {} lines", s1.lines().count());
    let s2 = strip_inline_comments(&s1);
    trace!(target: "assembler6502::preprocess",
        "after strip_inline_comments: {} lines", s2.lines().count());
    s2
}
