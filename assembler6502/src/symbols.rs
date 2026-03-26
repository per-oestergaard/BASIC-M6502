use anyhow::{Result, anyhow};
use std::collections::HashMap;
use tracing::trace;

/// Symbol table with arithmetic expression evaluation.
///
/// Arithmetic operators: `+` `-` `*` `/` `&` (AND) `!` (OR) `-` (unary)
/// Numeric literals: decimal, `^Oooo` (octal), `$hh` (hex), `"c"` (char)
/// Grouping: `<expr>` (angle brackets)
#[derive(Debug, Default)]
pub struct SymTable {
    values: HashMap<String, i64>,
}

impl SymTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, name: &str, val: i64) {
        trace!(target: "assembler6502::symbols", "set {} = {}", name, val);
        self.values.insert(name.to_ascii_uppercase(), val);
    }

    pub fn get(&self, name: &str) -> Option<i64> {
        self.values.get(&name.to_ascii_uppercase()).copied()
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.values.contains_key(&name.to_ascii_uppercase())
    }

    pub fn eval(&self, expr: &str) -> Result<i64> {
        let tokens = tokenize(expr.trim())?;
        if tokens.is_empty() {
            return Err(anyhow!("empty expression"));
        }
        let mut pos = 0;
        let val = parse_or(&tokens, &mut pos, self)?;
        if pos != tokens.len() {
            return Err(anyhow!("unexpected tokens after expression in {:?}", expr));
        }
        Ok(val)
    }
}

// ── Recursive-descent expression parser ──────────────────────────────────────

fn parse_or(t: &[Token], p: &mut usize, s: &SymTable) -> Result<i64> {
    let mut lhs = parse_and(t, p, s)?;
    while *p < t.len() {
        if matches!(t[*p], Token::Op('!')) {
            *p += 1;
            lhs |= parse_and(t, p, s)?;
        } else {
            break;
        }
    }
    Ok(lhs)
}

fn parse_and(t: &[Token], p: &mut usize, s: &SymTable) -> Result<i64> {
    let mut lhs = parse_add(t, p, s)?;
    while *p < t.len() {
        if matches!(t[*p], Token::Op('&')) {
            *p += 1;
            lhs &= parse_add(t, p, s)?;
        } else {
            break;
        }
    }
    Ok(lhs)
}

fn parse_add(t: &[Token], p: &mut usize, s: &SymTable) -> Result<i64> {
    let mut lhs = parse_mul(t, p, s)?;
    while *p < t.len() {
        match t[*p] {
            Token::Op('+') => {
                *p += 1;
                lhs += parse_mul(t, p, s)?;
            }
            Token::Op('-') => {
                *p += 1;
                lhs -= parse_mul(t, p, s)?;
            }
            _ => break,
        }
    }
    Ok(lhs)
}

fn parse_mul(t: &[Token], p: &mut usize, s: &SymTable) -> Result<i64> {
    let mut lhs = parse_unary(t, p, s)?;
    while *p < t.len() {
        match t[*p] {
            Token::Op('*') => {
                *p += 1;
                lhs *= parse_unary(t, p, s)?;
            }
            Token::Op('/') => {
                *p += 1;
                let rhs = parse_unary(t, p, s)?;
                if rhs == 0 {
                    return Err(anyhow!("division by zero"));
                }
                lhs /= rhs;
            }
            _ => break,
        }
    }
    Ok(lhs)
}

fn parse_unary(t: &[Token], p: &mut usize, s: &SymTable) -> Result<i64> {
    if *p < t.len() && matches!(t[*p], Token::Op('-')) {
        *p += 1;
        return Ok(-parse_unary(t, p, s)?);
    }
    parse_atom(t, p, s)
}

fn parse_atom(t: &[Token], p: &mut usize, s: &SymTable) -> Result<i64> {
    if *p >= t.len() {
        return Err(anyhow!("unexpected end of expression"));
    }
    match t[*p].clone() {
        Token::Num(n) => {
            *p += 1;
            Ok(n)
        }
        Token::Sym(name) => {
            *p += 1;
            s.get(&name)
                .ok_or_else(|| anyhow!("undefined symbol: {name}"))
        }
        Token::LAngle => {
            // <expr> grouping
            *p += 1;
            let val = parse_or(t, p, s)?;
            if *p < t.len() && matches!(t[*p], Token::RAngle) {
                *p += 1;
            }
            Ok(val)
        }
        tok => Err(anyhow!("unexpected token {tok:?}")),
    }
}

// ── Tokeniser ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Token {
    Num(i64),
    Sym(String),
    Op(char),
    LAngle,
    RAngle,
}

fn tokenize(expr: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = expr.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => i += 1,
            '+' | '-' | '*' | '/' | '&' | '!' => {
                tokens.push(Token::Op(c));
                i += 1;
            }
            '<' => {
                tokens.push(Token::LAngle);
                i += 1;
            }
            '>' => {
                tokens.push(Token::RAngle);
                i += 1;
            }
            '^' if i + 1 < chars.len() && (chars[i + 1] == 'O' || chars[i + 1] == 'o') => {
                i += 2;
                let start = i;
                while i < chars.len() && matches!(chars[i], '0'..='7') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Num(
                    i64::from_str_radix(&s, 8).map_err(|_| anyhow!("invalid octal: ^O{s}"))?,
                ));
            }
            '$' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i].is_ascii_hexdigit() {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Num(
                    i64::from_str_radix(&s, 16).map_err(|_| anyhow!("invalid hex: ${s}"))?,
                ));
            }
            '"' => {
                // character literal: "A" → 65
                i += 1;
                if i < chars.len() {
                    tokens.push(Token::Num(chars[i] as i64));
                    i += 1;
                    if i < chars.len() && chars[i] == '"' {
                        i += 1;
                    }
                }
            }
            '0'..='9' => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Num(
                    s.parse::<i64>()
                        .map_err(|_| anyhow!("invalid number: {s}"))?,
                ));
            }
            c if c.is_ascii_alphabetic() || c == '_' || c == '.' || c == '%' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric()
                        || chars[i] == '_'
                        || chars[i] == '.'
                        || chars[i] == '$')
                {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Sym(s));
            }
            other => return Err(anyhow!("unexpected char {other:?} in expression {expr:?}")),
        }
    }
    Ok(tokens)
}
