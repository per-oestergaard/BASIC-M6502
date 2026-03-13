/// AST Expander: Expand macros, evaluate conditionals, unroll repeats
use crate::ast::*;
use anyhow::Result;
use std::collections::HashMap;

pub struct Expander {
    macros: HashMap<String, (Vec<String>, Vec<String>)>, // name -> (params, body)
    symbols: HashMap<String, i64>,
}

impl Expander {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
            symbols: HashMap::new(),
        }
    }
    
    pub fn expand(&mut self, nodes: Vec<AstNode>) -> Result<Vec<AstNode>> {
        let mut result = Vec::new();
        
        for node in nodes {
            match node {
                AstNode::MacroDef { name, params, body } => {
                    self.macros.insert(name, (params, body));
                }
                AstNode::Conditional { kind, condition, then_block, else_block } => {
                    let cond_val = self.eval_expr(&condition)?;
                    let should_expand = match kind {
                        CondKind::IfEqual => cond_val == 0,
                        CondKind::IfNotEqual => cond_val != 0,
                        CondKind::IfNotDef => {
                            // Check if symbol is defined
                            if let Expr::Symbol(name) = &condition {
                                !self.symbols.contains_key(name)
                            } else {
                                false
                            }
                        }
                    };
                    
                    if should_expand {
                        let expanded = self.expand(then_block)?;
                        result.extend(expanded);
                    } else {
                        let expanded = self.expand(else_block)?;
                        result.extend(expanded);
                    }
                }
                AstNode::Repeat { label, count, body } => {
                    // Emit label once
                    if let Some(lbl) = label {
                        result.push(AstNode::Label { name: lbl });
                    }
                    
                    // Unroll body count times
                    for _ in 0..count {
                        let expanded = self.expand(body.clone())?;
                        result.extend(expanded);
                    }
                }
                AstNode::Equate { name, expr } => {
                    if let Ok(val) = self.eval_expr(&expr) {
                        self.symbols.insert(name.clone(), val);
                    }
                    result.push(AstNode::Equate { name, expr });
                }
                _ => {
                    result.push(node);
                }
            }
        }
        
        Ok(result)
    }
    
    fn eval_expr(&self, expr: &Expr) -> Result<i64> {
        match expr {
            Expr::Number(n) => Ok(*n),
            Expr::Symbol(name) => {
                if let Some(val) = self.symbols.get(name) {
                    Ok(*val)
                } else {
                    // Unknown symbol - return 0 for now
                    Ok(0)
                }
            }
            Expr::BinOp { op, left, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                Ok(match op {
                    BinOp::Add => l + r,
                    BinOp::Sub => l - r,
                    BinOp::Mul => l * r,
                    BinOp::Div => l / r,
                    BinOp::And => l & r,
                    BinOp::Or => l | r,
                    BinOp::Xor => l ^ r,
                })
            }
            Expr::UnaryOp { op, expr } => {
                let val = self.eval_expr(expr)?;
                Ok(match op {
                    UnaryOp::Neg => -val,
                    UnaryOp::Not => !val,
                })
            }
            Expr::LowByte(inner) => {
                let val = self.eval_expr(inner)?;
                Ok(val & 0xFF)
            }
            Expr::HighByte(inner) => {
                let val = self.eval_expr(inner)?;
                Ok((val >> 8) & 0xFF)
            }
        }
    }
}
