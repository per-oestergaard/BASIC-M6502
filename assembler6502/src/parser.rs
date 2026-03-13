/// Parser: Convert raw source text to AST
use crate::ast::*;
use anyhow::{bail, Result};
use regex::Regex;

pub struct Parser {
    lines: Vec<String>,
    pos: usize,
    #[allow(dead_code)]
    ctx: ParseContext,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        Parser {
            lines,
            pos: 0,
            ctx: ParseContext::new(),
        }
    }

    pub fn parse(&mut self) -> Result<Vec<AstNode>> {
        let mut nodes = Vec::new();

        // Parse all lines
        while self.pos < self.lines.len() {
            if let Some(node) = self.parse_line()? {
                nodes.push(node);
            }
            self.pos += 1;
        }

        Ok(nodes)
    }

    fn current_line(&self) -> String {
        if self.pos < self.lines.len() {
            self.lines[self.pos].clone()
        } else {
            String::new()
        }
    }

    fn parse_line(&mut self) -> Result<Option<AstNode>> {
        let raw = self.current_line();
        let line = strip_comment(&raw);

        // Skip empty, directives, comments
        if line.trim().is_empty() {
            return Ok(None);
        }

        if self.is_skip_directive(&line) {
            return Ok(None);
        }

        // Try parsing different constructs
        if let Some(node) = self.try_parse_conditional(&line)? {
            return Ok(Some(node));
        }

        if let Some(node) = self.try_parse_macro_def(&line)? {
            return Ok(Some(node));
        }

        if let Some(node) = self.try_parse_repeat(&line)? {
            return Ok(Some(node));
        }

        if let Some(node) = self.try_parse_equate(&line)? {
            return Ok(Some(node));
        }

        if let Some(node) = self.try_parse_directive(&line)? {
            return Ok(Some(node));
        }

        if let Some(node) = self.try_parse_instruction(&line)? {
            return Ok(Some(node));
        }

        // If nothing matches and it looks like code, it's probably prose
        if !line.contains(':') && !line.contains('=') {
            return Ok(None); // Skip prose
        }

        Ok(None)
    }

    fn is_skip_directive(&self, line: &str) -> bool {
        let patterns = [
            "TITLE", "SEARCH", "SALL", "RADIX", "SUBTTL", "PAGE", "PRINTX", "XLIST", ".XCREF",
            ".CREF", "LIST", "PURGE",
        ];
        patterns.iter().any(|p| line.trim_start().starts_with(p))
    }

    fn try_parse_equate(&mut self, line: &str) -> Result<Option<AstNode>> {
        lazy_static::lazy_static! {
            static ref EQU_RE: Regex = Regex::new(r"^([A-Za-z_.$][\w.$]*)\s*==?\s*(.+)").unwrap();
        }

        if let Some(cap) = EQU_RE.captures(line.trim()) {
            let name = cap[1].to_string();
            let expr_str = cap[2].trim();
            let expr = self.parse_expr(expr_str)?;
            return Ok(Some(AstNode::Equate { name, expr }));
        }
        Ok(None)
    }

    fn try_parse_conditional(&mut self, line: &str) -> Result<Option<AstNode>> {
        lazy_static::lazy_static! {
            static ref IF_RE: Regex = Regex::new(r"^(IFE|IFN|IFNDEF)\s+([^,]+),<(.*)$").unwrap();
        }

        if let Some(cap) = IF_RE.captures(line.trim()) {
            let kind_str = &cap[1];
            let cond_str = cap[2].trim();
            let kind = match kind_str {
                "IFE" => CondKind::IfEqual,
                "IFN" => CondKind::IfNotEqual,
                "IFNDEF" => CondKind::IfNotDef,
                _ => bail!("Unknown conditional: {}", kind_str),
            };

            let condition = self.parse_expr(cond_str)?;

            // Parse body until closing >
            let mut then_block = Vec::new();
            self.pos += 1;
            while self.pos < self.lines.len() {
                let body_line = self.current_line();
                if body_line.trim() == ">" || body_line.trim().ends_with(">>") {
                    break;
                }
                if let Some(node) = self.parse_line()? {
                    then_block.push(node);
                }
                self.pos += 1;
            }

            return Ok(Some(AstNode::Conditional {
                kind,
                condition,
                then_block,
                else_block: vec![],
            }));
        }
        Ok(None)
    }

    fn try_parse_macro_def(&mut self, line: &str) -> Result<Option<AstNode>> {
        if !line.trim_start().starts_with("DEFINE") {
            return Ok(None);
        }

        // Parse DEFINE NAME (PARAMS),< ... >
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(None);
        }

        let name = parts[1].to_string();
        let params = vec![]; // TODO: parse params from (PARAM1, PARAM2)

        // Collect body
        let mut body = Vec::new();
        self.pos += 1;
        while self.pos < self.lines.len() {
            let body_line = self.lines[self.pos].clone();
            if body_line.trim_end().ends_with('>') {
                break;
            }
            body.push(body_line);
            self.pos += 1;
        }

        Ok(Some(AstNode::MacroDef { name, params, body }))
    }

    fn try_parse_repeat(&mut self, line: &str) -> Result<Option<AstNode>> {
        let (label, rest) = self.split_label(line);

        if !rest.trim_start().starts_with("REPEAT") {
            return Ok(None);
        }

        // Parse "REPEAT N,<body>"
        let after_repeat = rest.trim_start().strip_prefix("REPEAT").unwrap().trim();

        // Split on comma to separate count from body
        if let Some(comma_pos) = after_repeat.find(',') {
            let count_str = after_repeat[..comma_pos].trim();
            let count = count_str.parse::<usize>().unwrap_or(1);
            let rest_after_comma = &after_repeat[comma_pos + 1..];

            // Check for single-line REPEAT with <body>
            if let Some(start) = rest_after_comma.find('<') {
                if let Some(end) = rest_after_comma.rfind('>') {
                    let body_str = rest_after_comma[start + 1..end].trim();
                    // Parse body as a single instruction
                    let body_node = self.try_parse_instruction(body_str)?;
                    let body = if let Some(n) = body_node {
                        vec![n]
                    } else {
                        vec![]
                    };
                    return Ok(Some(AstNode::Repeat { label, count, body }));
                }
            }
        }

        // Multi-line REPEAT
        let count = 1; // TODO: parse count for multi-line
        let mut body = Vec::new();
        self.pos += 1;
        while self.pos < self.lines.len() {
            let body_line = self.current_line();
            if body_line.trim() == ">" || body_line.trim().ends_with(">>") {
                break;
            }
            if let Some(node) = self.parse_line()? {
                body.push(node);
            }
            self.pos += 1;
        }

        Ok(Some(AstNode::Repeat { label, count, body }))
    }

    fn try_parse_directive(&mut self, line: &str) -> Result<Option<AstNode>> {
        let (label, rest) = self.split_label(line);
        let trimmed = rest.trim();

        // ORG
        if trimmed.starts_with("ORG") {
            let expr_str = trimmed.strip_prefix("ORG").unwrap().trim();
            let expr = self.parse_expr(expr_str)?;
            return Ok(Some(AstNode::Directive {
                label,
                directive: Directive::Org(expr),
            }));
        }

        // .byte, .word, .res
        if trimmed.starts_with(".byte") {
            // TODO: parse byte list
            return Ok(Some(AstNode::Directive {
                label,
                directive: Directive::Byte(vec![]),
            }));
        }

        Ok(None)
    }

    fn try_parse_instruction(&mut self, line: &str) -> Result<Option<AstNode>> {
        let (label, rest) = self.split_label(line);
        let parts: Vec<&str> = rest.trim().split_whitespace().collect();

        if parts.is_empty() {
            // Just a label
            if label.is_some() {
                return Ok(Some(AstNode::Label {
                    name: label.unwrap(),
                }));
            }
            return Ok(None);
        }

        let mnemonic = parts[0].to_string();
        let operand = if parts.len() > 1 {
            let op_str = parts[1..].join(" ");
            Some(self.parse_operand(&op_str)?)
        } else {
            None
        };

        Ok(Some(AstNode::Instruction {
            label,
            mnemonic,
            operand,
        }))
    }

    fn split_label(&self, line: &str) -> (Option<String>, String) {
        // Handle double colon (::) MACRO-10 syntax
        if let Some(double_colon_pos) = line.find("::") {
            let label = line[..double_colon_pos].trim().to_string();
            let rest = line[double_colon_pos + 2..].to_string();
            return (Some(label), rest);
        }

        // Handle single colon
        if let Some(colon_pos) = line.find(':') {
            let label = line[..colon_pos].trim().to_string();
            let rest = line[colon_pos + 1..].to_string();
            (Some(label), rest)
        } else {
            (None, line.to_string())
        }
    }

    fn parse_expr(&self, s: &str) -> Result<Expr> {
        let s = s.trim();

        // Number literal
        if s.starts_with('$') {
            if let Ok(val) = i64::from_str_radix(&s[1..], 16) {
                return Ok(Expr::Number(val));
            }
        }

        if s.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(val) = s.parse::<i64>() {
                return Ok(Expr::Number(val));
            }
        }

        // Low/high byte
        if s.starts_with('<') && s.len() > 1 {
            let inner = self.parse_expr(&s[1..])?;
            return Ok(Expr::LowByte(Box::new(inner)));
        }

        if s.starts_with('>') && s.len() > 1 {
            let inner = self.parse_expr(&s[1..])?;
            return Ok(Expr::HighByte(Box::new(inner)));
        }

        // Binary operations
        for op_str in ["+", "-", "*", "/"] {
            if let Some(pos) = s.rfind(op_str) {
                let left = self.parse_expr(&s[..pos])?;
                let right = self.parse_expr(&s[pos + 1..])?;
                let op = match op_str {
                    "+" => BinOp::Add,
                    "-" => BinOp::Sub,
                    "*" => BinOp::Mul,
                    "/" => BinOp::Div,
                    _ => unreachable!(),
                };
                return Ok(Expr::BinOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                });
            }
        }

        // Symbol
        Ok(Expr::Symbol(s.to_string()))
    }

    fn parse_operand(&self, s: &str) -> Result<Operand> {
        let s = s.trim();

        if s.starts_with('#') {
            let expr = self.parse_expr(&s[1..])?;
            return Ok(Operand::Immediate(expr));
        }

        if s.ends_with(",X") {
            let base = self.parse_expr(s.trim_end_matches(",X"))?;
            return Ok(Operand::AbsoluteX(base));
        }

        if s.ends_with(",Y") {
            let base = self.parse_expr(s.trim_end_matches(",Y"))?;
            return Ok(Operand::AbsoluteY(base));
        }

        // Accumulator
        if s.is_empty() || s == "A" {
            return Ok(Operand::Accumulator);
        }

        // Absolute/ZP (determine later)
        let expr = self.parse_expr(s)?;
        Ok(Operand::Absolute(expr))
    }
}

fn strip_comment(s: &str) -> String {
    if let Some(pos) = s.find(';') {
        s[..pos].trim_end().to_string()
    } else {
        s.to_string()
    }
}
