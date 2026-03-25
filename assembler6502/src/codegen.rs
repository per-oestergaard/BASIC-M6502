/// Code generator — converts FlatStmt list back to assembly text (for debugging).
use crate::ast::FlatStmt;
use anyhow::Result;

pub struct CodeGen {
    output: Vec<String>,
}

impl CodeGen {
    pub fn new() -> Self {
        Self { output: Vec::new() }
    }

    pub fn generate(&mut self, stmts: &[FlatStmt]) -> Result<String> {
        for stmt in stmts {
            match stmt {
                FlatStmt::Label(name) => self.output.push(format!("{}:", name)),
                FlatStmt::Equate { name, value } => {
                    self.output.push(format!("{} = {}", name, value));
                }
                FlatStmt::Org(addr) => self.output.push(format!("ORG ${:04X}", addr)),
                FlatStmt::Res { label, count } => {
                    let prefix = label
                        .as_deref()
                        .map(|l| format!("{}: ", l))
                        .unwrap_or_default();
                    self.output.push(format!("{}.res {}", prefix, count));
                }
                FlatStmt::Bytes { label, values } => {
                    let prefix = label
                        .as_deref()
                        .map(|l| format!("{}: ", l))
                        .unwrap_or_default();
                    self.output
                        .push(format!("{}.byte {}", prefix, values.join(", ")));
                }
                FlatStmt::Word { label, expr } => {
                    let prefix = label
                        .as_deref()
                        .map(|l| format!("{}: ", l))
                        .unwrap_or_default();
                    self.output.push(format!("{}.word {}", prefix, expr));
                }
                FlatStmt::Instr {
                    label,
                    mnemonic,
                    operand,
                } => {
                    let prefix = label
                        .as_deref()
                        .map(|l| format!("{}: ", l))
                        .unwrap_or_default();
                    let op = operand
                        .as_deref()
                        .map(|o| format!(" {}", o))
                        .unwrap_or_default();
                    self.output.push(format!("{}{}{}", prefix, mnemonic, op));
                }
            }
        }
        Ok(self.output.join("\n"))
    }
}

pub struct CodeGen {
    output: Vec<String>,
}

impl CodeGen {
    pub fn new() -> Self {
        Self { output: Vec::new() }
    }

    pub fn generate(&mut self, nodes: Vec<AstNode>) -> Result<String> {
        for node in nodes {
            self.generate_node(node)?;
        }
        Ok(self.output.join("\n"))
    }

    fn generate_node(&mut self, node: AstNode) -> Result<()> {
        match node {
            AstNode::Label { name } => {
                self.output.push(format!("{}:", name));
            }
            AstNode::Instruction {
                label,
                mnemonic,
                operand,
            } => {
                let mut line = String::new();
                if let Some(lbl) = label {
                    line.push_str(&format!("{}: ", lbl));
                }
                line.push_str(&mnemonic);
                if let Some(op) = operand {
                    line.push(' ');
                    line.push_str(&self.format_operand(&op)?);
                }
                self.output.push(line);
            }
            AstNode::Directive { label, directive } => {
                let mut line = String::new();
                if let Some(lbl) = label {
                    line.push_str(&format!("{}: ", lbl));
                }
                line.push_str(&self.format_directive(&directive)?);
                self.output.push(line);
            }
            AstNode::Equate { name, expr } => {
                self.output
                    .push(format!("{} = {}", name, self.format_expr(&expr)?));
            }
            AstNode::MacroCall { .. } => {
                // Should be expanded already
            }
            AstNode::Conditional { .. } => {
                // Should be expanded already
            }
            AstNode::Repeat { .. } => {
                // Should be expanded already
            }
            AstNode::MacroDef { .. } => {
                // Skip macro definitions
            }
            AstNode::Comment(text) => {
                self.output.push(format!("; {}", text));
            }
        }
        Ok(())
    }

    fn format_operand(&self, operand: &Operand) -> Result<String> {
        Ok(match operand {
            Operand::Immediate(expr) => format!("#{}", self.format_expr(expr)?),
            Operand::Absolute(expr) => self.format_expr(expr)?,
            Operand::AbsoluteX(expr) => format!("{},X", self.format_expr(expr)?),
            Operand::AbsoluteY(expr) => format!("{},Y", self.format_expr(expr)?),
            Operand::ZeroPage(expr) => self.format_expr(expr)?,
            Operand::ZeroPageX(expr) => format!("{},X", self.format_expr(expr)?),
            Operand::ZeroPageY(expr) => format!("{},Y", self.format_expr(expr)?),
            Operand::Indirect(expr) => format!("({})", self.format_expr(expr)?),
            Operand::IndirectX(expr) => format!("({},X)", self.format_expr(expr)?),
            Operand::IndirectY(expr) => format!("({}),Y", self.format_expr(expr)?),
            Operand::Relative(expr) => self.format_expr(expr)?,
            Operand::Accumulator => "A".to_string(),
        })
    }

    fn format_directive(&self, directive: &Directive) -> Result<String> {
        Ok(match directive {
            Directive::Org(expr) => format!("ORG {}", self.format_expr(expr)?),
            Directive::Byte(exprs) => {
                let vals: Vec<String> = exprs
                    .iter()
                    .map(|e| self.format_expr(e))
                    .collect::<Result<Vec<_>>>()?;
                format!(".byte {}", vals.join(", "))
            }
            Directive::Word(exprs) => {
                let vals: Vec<String> = exprs
                    .iter()
                    .map(|e| self.format_expr(e))
                    .collect::<Result<Vec<_>>>()?;
                format!(".word {}", vals.join(", "))
            }
            Directive::Reserve(size) => format!(".res {}", size),
            Directive::Align(boundary) => format!(".align {}", boundary),
        })
    }

    fn format_expr(&self, expr: &Expr) -> Result<String> {
        Ok(match expr {
            Expr::Number(n) => {
                if *n < 0 {
                    format!("{}", n)
                } else if *n < 256 {
                    format!("${:02X}", n)
                } else {
                    format!("${:04X}", n)
                }
            }
            Expr::Symbol(name) => name.clone(),
            Expr::BinOp { op, left, right } => {
                let l = self.format_expr(left)?;
                let r = self.format_expr(right)?;
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::And => "&",
                    BinOp::Or => "|",
                    BinOp::Xor => "^",
                };
                format!("{}{}{}", l, op_str, r)
            }
            Expr::UnaryOp { op, expr } => {
                let val = self.format_expr(expr)?;
                match op {
                    UnaryOp::Neg => format!("-{}", val),
                    UnaryOp::Not => format!("!{}", val),
                }
            }
            Expr::LowByte(inner) => format!("<{}", self.format_expr(inner)?),
            Expr::HighByte(inner) => format!(">{}", self.format_expr(inner)?),
        })
    }
}
