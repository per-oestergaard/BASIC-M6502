//! Interpreter for Microsoft BASIC
//!
//! Executes parsed BASIC programs.

use crate::parser::*;
use std::collections::HashMap;
use std::io;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Number(f64),
    String(String),
}

impl Value {
    fn as_number(&self) -> Result<f64, String> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::String(_) => Err("Type mismatch: expected number".to_string()),
        }
    }

    fn as_string(&self) -> Result<String, String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            Value::Number(_) => Err("Type mismatch: expected string".to_string()),
        }
    }

    fn to_bool(&self) -> bool {
        match self {
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
        }
    }
}

pub struct Interpreter {
    variables: HashMap<String, Value>,
    arrays: HashMap<String, Array>,
    output: String,
    input_queue: Vec<String>,
    input_pos: usize,
    line_map: HashMap<u32, usize>,
    pc: usize,
    current_line: u32,
    for_stack: Vec<ForContext>,
    gosub_stack: Vec<usize>,
    data_values: Vec<DataValue>,
    data_pos: usize,
    user_functions: HashMap<String, (String, Expr)>,
    rnd_seed: f64,
    print_column: usize,
    memory: HashMap<u16, u8>,
}

struct Array {
    dims: Vec<usize>,
    values: Vec<Value>,
}

#[derive(Clone)]
struct ForContext {
    var: String,
    end_value: f64,
    step: f64,
    return_pc: usize,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            arrays: HashMap::new(),
            output: String::new(),
            input_queue: Vec::new(),
            input_pos: 0,
            line_map: HashMap::new(),
            pc: 0,
            current_line: 0,
            for_stack: Vec::new(),
            gosub_stack: Vec::new(),
            data_values: Vec::new(),
            data_pos: 0,
            user_functions: HashMap::new(),
            rnd_seed: 0.0,
            print_column: 0,
            memory: HashMap::new(),
        }
    }

    pub fn queue_input(&mut self, input: &str) {
        self.input_queue.push(input.to_string());
    }

    pub fn run(&mut self, program: &Program) -> Result<String, String> {
        // Build line number map and collect DATA statements
        self.build_line_map(program)?;
        self.collect_data(program)?;

        // Execute program
        self.pc = 0;

        while self.pc < program.lines.len() {
            let line = &program.lines[self.pc];
            self.current_line = line.line_number;

            for stmt in &line.statements {
                self.execute_statement(stmt, program)?;
            }

            self.pc += 1;
        }

        Ok(self.output.clone())
    }

    fn build_line_map(&mut self, program: &Program) -> Result<(), String> {
        for (idx, line) in program.lines.iter().enumerate() {
            self.line_map.insert(line.line_number, idx);
        }
        Ok(())
    }

    fn collect_data(&mut self, program: &Program) -> Result<(), String> {
        for line in &program.lines {
            for stmt in &line.statements {
                if let Statement::Data(data_stmt) = stmt {
                    self.data_values.extend(data_stmt.values.clone());
                }
            }
        }
        Ok(())
    }

    fn execute_statement(&mut self, stmt: &Statement, program: &Program) -> Result<(), String> {
        match stmt {
            Statement::Print(print_stmt) => self.exec_print(print_stmt),
            Statement::Let(let_stmt) => self.exec_let(let_stmt),
            Statement::If(if_stmt) => self.exec_if(if_stmt, program),
            Statement::For(for_stmt) => self.exec_for(for_stmt),
            Statement::Next(next_stmt) => self.exec_next(next_stmt, program),
            Statement::Goto(goto_stmt) => self.exec_goto(goto_stmt),
            Statement::Gosub(gosub_stmt) => self.exec_gosub(gosub_stmt),
            Statement::Return => self.exec_return(),
            Statement::Dim(dim_stmt) => self.exec_dim(dim_stmt),
            Statement::Read(read_stmt) => self.exec_read(read_stmt),
            Statement::Data(_) => Ok(()), // DATA is collected beforehand
            Statement::Restore(restore_stmt) => self.exec_restore(restore_stmt),
            Statement::On(on_stmt) => self.exec_on(on_stmt),
            Statement::Def(def_stmt) => self.exec_def(def_stmt),
            Statement::End => self.exec_end(),
            Statement::Stop => self.exec_stop(),
            Statement::Clear => self.exec_clear(),
            Statement::New => self.exec_new(),
            Statement::Rem(_) => Ok(()), // Comments are ignored
            Statement::Input(input_stmt) => self.exec_input(input_stmt),
            Statement::Poke(poke_stmt) => self.exec_poke(poke_stmt),
        }
    }

    fn exec_print(&mut self, stmt: &PrintStmt) -> Result<(), String> {
        for (i, item) in stmt.items.iter().enumerate() {
            let is_last = i == stmt.items.len() - 1;
            let next_is_semicolon = if i + 1 < stmt.items.len() {
                matches!(stmt.items[i + 1], PrintItem::Semicolon)
            } else {
                false
            };

            match item {
                PrintItem::Expr(expr) => {
                    let value = self.eval_expr(expr)?;
                    let (text, is_number) = match value {
                        Value::Number(n) => {
                            // Microsoft BASIC formatting: space before positive, minus for negative
                            let text = if n >= 0.0 {
                                format!(" {}", n)
                            } else {
                                format!("{}", n)
                            };
                            (text, true)
                        }
                        Value::String(s) => (s, false),
                    };
                    self.output.push_str(&text);
                    self.print_column += text.len();

                    // Add trailing space after numbers if followed by semicolon
                    if is_number && next_is_semicolon {
                        self.output.push(' ');
                        self.print_column += 1;
                    }
                }
                PrintItem::Tab(expr) => {
                    let pos = self.eval_expr(expr)?.as_number()? as usize;
                    while self.print_column < pos {
                        self.output.push(' ');
                        self.print_column += 1;
                    }
                }
                PrintItem::Spc(expr) => {
                    let count = self.eval_expr(expr)?.as_number()? as usize;
                    for _ in 0..count {
                        self.output.push(' ');
                        self.print_column += 1;
                    }
                }
                PrintItem::Semicolon => {
                    // Semicolon just suppresses newline - spacing handled above
                }
                PrintItem::Comma => {
                    // Tab to next 14-character zone
                    let next_zone = ((self.print_column / 14) + 1) * 14;
                    while self.print_column < next_zone {
                        self.output.push(' ');
                        self.print_column += 1;
                    }
                }
            }

            // Add newline if this is the last item and it's not a separator
            if is_last {
                match item {
                    PrintItem::Semicolon | PrintItem::Comma => {
                        // Trailing separator: no newline
                    }
                    _ => {
                        // Trim trailing spaces before adding newline
                        self.output = self.output.trim_end().to_string();
                        self.output.push('\n');
                        self.print_column = 0;
                    }
                }
            }
        }

        // Empty PRINT statement gets just a newline
        if stmt.items.is_empty() {
            // Trim trailing spaces from previous PRINT with trailing comma
            self.output = self.output.trim_end().to_string();
            self.output.push('\n');
            self.print_column = 0;
        }

        Ok(())
    }

    fn exec_let(&mut self, stmt: &LetStmt) -> Result<(), String> {
        let value = self.eval_expr(&stmt.value)?;
        self.set_var(&stmt.var, value)?;
        Ok(())
    }

    fn exec_if(&mut self, stmt: &IfStmt, program: &Program) -> Result<(), String> {
        let condition = self.eval_expr(&stmt.condition)?;

        if condition.to_bool() {
            for s in &stmt.then_stmt {
                self.execute_statement(s, program)?;
            }
        } else if let Some(else_stmts) = &stmt.else_stmt {
            for s in else_stmts {
                self.execute_statement(s, program)?;
            }
        }

        Ok(())
    }

    fn exec_for(&mut self, stmt: &ForStmt) -> Result<(), String> {
        let start = self.eval_expr(&stmt.start)?.as_number()?;
        let end = self.eval_expr(&stmt.end)?.as_number()?;
        let step = if let Some(step_expr) = &stmt.step {
            self.eval_expr(step_expr)?.as_number()?
        } else {
            1.0
        };

        // Initialize loop variable
        self.variables
            .insert(stmt.var.clone(), Value::Number(start));

        // Push FOR context
        self.for_stack.push(ForContext {
            var: stmt.var.clone(),
            end_value: end,
            step,
            return_pc: self.pc,
        });

        Ok(())
    }

    fn exec_next(&mut self, stmt: &NextStmt, _program: &Program) -> Result<(), String> {
        if self.for_stack.is_empty() {
            return Err("NEXT without FOR".to_string());
        }

        let ctx = self.for_stack.last().unwrap().clone();

        // Check variable name if specified
        if let Some(var_name) = &stmt.var {
            if var_name != &ctx.var {
                return Err(format!("NEXT {} doesn't match FOR {}", var_name, ctx.var));
            }
        }

        // Increment loop variable
        let current = self
            .variables
            .get(&ctx.var)
            .ok_or("Loop variable not found")?
            .as_number()?;
        let next_val = current + ctx.step;

        // Check if loop should continue
        let done = if ctx.step >= 0.0 {
            next_val > ctx.end_value
        } else {
            next_val < ctx.end_value
        };

        if done {
            // Exit loop
            self.for_stack.pop();
        } else {
            // Continue loop
            self.variables
                .insert(ctx.var.clone(), Value::Number(next_val));
            self.pc = ctx.return_pc;
        }

        Ok(())
    }

    fn exec_goto(&mut self, stmt: &GotoStmt) -> Result<(), String> {
        let target_pc = self
            .line_map
            .get(&stmt.target)
            .ok_or(format!("Undefined line number: {}", stmt.target))?;
        self.pc = *target_pc;
        // Subtract 1 because pc will be incremented after this statement
        if self.pc > 0 {
            self.pc -= 1;
        }
        Ok(())
    }

    fn exec_gosub(&mut self, stmt: &GosubStmt) -> Result<(), String> {
        self.gosub_stack.push(self.pc);
        self.exec_goto(&GotoStmt {
            target: stmt.target,
        })
    }

    fn exec_return(&mut self) -> Result<(), String> {
        let return_pc = self.gosub_stack.pop().ok_or("RETURN without GOSUB")?;
        self.pc = return_pc;
        Ok(())
    }

    fn exec_dim(&mut self, stmt: &DimStmt) -> Result<(), String> {
        for (name, dim_exprs) in &stmt.arrays {
            let dims: Result<Vec<usize>, String> = dim_exprs
                .iter()
                .map(|e| self.eval_expr(e)?.as_number().map(|n| (n as usize) + 1))
                .collect();

            let dims = dims?;
            let total_size = dims.iter().product();

            let default_value = if name.ends_with('$') {
                Value::String(String::new())
            } else {
                Value::Number(0.0)
            };

            let array = Array {
                dims,
                values: vec![default_value; total_size],
            };

            self.arrays.insert(name.clone(), array);
        }

        Ok(())
    }

    fn exec_read(&mut self, stmt: &ReadStmt) -> Result<(), String> {
        for var_ref in &stmt.vars {
            if self.data_pos >= self.data_values.len() {
                return Err("Out of DATA".to_string());
            }

            let data_val = &self.data_values[self.data_pos];
            self.data_pos += 1;

            let value = match data_val {
                DataValue::Number(n) => Value::Number(*n),
                DataValue::String(s) => Value::String(s.clone()),
            };

            self.set_var(var_ref, value)?;
        }

        Ok(())
    }

    fn exec_restore(&mut self, _stmt: &RestoreStmt) -> Result<(), String> {
        self.data_pos = 0;
        Ok(())
    }

    fn exec_on(&mut self, stmt: &OnStmt) -> Result<(), String> {
        let index = self.eval_expr(&stmt.expr)?.as_number()? as usize;

        if index > 0 && index <= stmt.targets.len() {
            let target = stmt.targets[index - 1];
            match stmt.kind {
                OnKind::Goto => self.exec_goto(&GotoStmt { target }),
                OnKind::Gosub => self.exec_gosub(&GosubStmt { target }),
            }
        } else {
            // Out of range: continue to next statement
            Ok(())
        }
    }

    fn exec_def(&mut self, stmt: &DefStmt) -> Result<(), String> {
        self.user_functions
            .insert(stmt.name.clone(), (stmt.param.clone(), stmt.body.clone()));
        Ok(())
    }

    fn exec_end(&mut self) -> Result<(), String> {
        // Jump to end of program
        self.pc = usize::MAX - 1; // Will be incremented and exit loop
        Ok(())
    }

    fn exec_stop(&mut self) -> Result<(), String> {
        // STOP outputs a message and ends program
        self.output.push_str("\nBREAK IN ");
        self.output.push_str(&format!(" {}", self.current_line));
        self.pc = usize::MAX - 1; // Will be incremented and exit loop
        Ok(())
    }

    fn exec_clear(&mut self) -> Result<(), String> {
        // CLEAR resets all variables and arrays to zero/empty
        self.variables.clear();
        self.arrays.clear();
        Ok(())
    }

    fn exec_new(&mut self) -> Result<(), String> {
        // NEW clears everything and outputs "OK"
        self.variables.clear();
        self.arrays.clear();
        self.for_stack.clear();
        self.gosub_stack.clear();
        self.user_functions.clear();
        self.data_pos = 0;
        self.output.push_str("OK");
        self.pc = usize::MAX - 1; // Exit program
        Ok(())
    }

    fn exec_input(&mut self, stmt: &InputStmt) -> Result<(), String> {
        // Print prompt if present
        if let Some(prompt) = &stmt.prompt {
            self.output.push_str(prompt);
            self.output.push_str("? ");
        } else {
            self.output.push_str("? ");
        }

        // Get input
        let input_line = if self.input_pos < self.input_queue.len() {
            let line = self.input_queue[self.input_pos].clone();
            self.input_pos += 1;
            line
        } else {
            // Read from stdin
            let mut line = String::new();
            io::stdin()
                .read_line(&mut line)
                .map_err(|e| format!("Input error: {}", e))?;
            line.trim().to_string()
        };

        // Parse input values
        let values: Vec<&str> = input_line.split(',').map(|s| s.trim()).collect();

        if values.len() != stmt.vars.len() {
            return Err("Input count mismatch".to_string());
        }

        for (var_ref, val_str) in stmt.vars.iter().zip(values.iter()) {
            let value = match var_ref {
                VarRef::String(_) => Value::String(val_str.to_string()),
                _ => {
                    // Try to parse as number
                    val_str
                        .parse::<f64>()
                        .map(Value::Number)
                        .unwrap_or(Value::Number(0.0))
                }
            };

            self.set_var(var_ref, value)?;
        }

        Ok(())
    }

    fn exec_poke(&mut self, stmt: &PokeStmt) -> Result<(), String> {
        let addr = self.eval_expr(&stmt.addr)?.as_number()? as u16;
        let value = self.eval_expr(&stmt.value)?.as_number()? as u8;
        self.memory.insert(addr, value);
        Ok(())
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Variable(var_ref) => self.get_var(var_ref),
            Expr::FnCall(name, arg) => {
                let (param, body) = self
                    .user_functions
                    .get(name)
                    .ok_or(format!("Undefined function: {}", name))?
                    .clone();

                let arg_value = self.eval_expr(arg)?;

                // Save current value of parameter (if any)
                let old_value = self.variables.get(&param).cloned();

                // Set parameter
                self.variables.insert(param.clone(), arg_value);

                // Evaluate function body
                let result = self.eval_expr(&body)?;

                // Restore old value
                if let Some(old_val) = old_value {
                    self.variables.insert(param, old_val);
                } else {
                    self.variables.remove(&param);
                }

                Ok(result)
            }
            Expr::UnaryOp(op, operand) => {
                let val = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Neg => Ok(Value::Number(-val.as_number()?)),
                    UnaryOp::Not => Ok(Value::Number(if val.to_bool() { 0.0 } else { -1.0 })),
                }
            }
            Expr::BinaryOp(left, op, right) => {
                let left_val = self.eval_expr(left)?;
                let right_val = self.eval_expr(right)?;

                match op {
                    BinaryOp::Add => match (&left_val, &right_val) {
                        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                        (Value::String(a), Value::String(b)) => {
                            Ok(Value::String(format!("{}{}", a, b)))
                        }
                        _ => Err("Type mismatch in addition".to_string()),
                    },
                    BinaryOp::Sub => Ok(Value::Number(
                        left_val.as_number()? - right_val.as_number()?,
                    )),
                    BinaryOp::Mul => Ok(Value::Number(
                        left_val.as_number()? * right_val.as_number()?,
                    )),
                    BinaryOp::Div => Ok(Value::Number(
                        left_val.as_number()? / right_val.as_number()?,
                    )),
                    BinaryOp::Pow => Ok(Value::Number(
                        left_val.as_number()?.powf(right_val.as_number()?),
                    )),
                    BinaryOp::Eq => {
                        let result = left_val == right_val;
                        Ok(Value::Number(if result { -1.0 } else { 0.0 }))
                    }
                    BinaryOp::Ne => {
                        let result = left_val != right_val;
                        Ok(Value::Number(if result { -1.0 } else { 0.0 }))
                    }
                    BinaryOp::Lt => {
                        let result = match (&left_val, &right_val) {
                            (Value::Number(a), Value::Number(b)) => a < b,
                            (Value::String(a), Value::String(b)) => a < b,
                            _ => return Err("Type mismatch in comparison".to_string()),
                        };
                        Ok(Value::Number(if result { -1.0 } else { 0.0 }))
                    }
                    BinaryOp::Gt => {
                        let result = match (&left_val, &right_val) {
                            (Value::Number(a), Value::Number(b)) => a > b,
                            (Value::String(a), Value::String(b)) => a > b,
                            _ => return Err("Type mismatch in comparison".to_string()),
                        };
                        Ok(Value::Number(if result { -1.0 } else { 0.0 }))
                    }
                    BinaryOp::Le => {
                        let result = match (&left_val, &right_val) {
                            (Value::Number(a), Value::Number(b)) => a <= b,
                            (Value::String(a), Value::String(b)) => a <= b,
                            _ => return Err("Type mismatch in comparison".to_string()),
                        };
                        Ok(Value::Number(if result { -1.0 } else { 0.0 }))
                    }
                    BinaryOp::Ge => {
                        let result = match (&left_val, &right_val) {
                            (Value::Number(a), Value::Number(b)) => a >= b,
                            (Value::String(a), Value::String(b)) => a >= b,
                            _ => return Err("Type mismatch in comparison".to_string()),
                        };
                        Ok(Value::Number(if result { -1.0 } else { 0.0 }))
                    }
                    BinaryOp::And => {
                        let a = left_val.as_number()? as i32;
                        let b = right_val.as_number()? as i32;
                        Ok(Value::Number((a & b) as f64))
                    }
                    BinaryOp::Or => {
                        let a = left_val.as_number()? as i32;
                        let b = right_val.as_number()? as i32;
                        Ok(Value::Number((a | b) as f64))
                    }
                }
            }
            Expr::BuiltinFn(func) => self.eval_builtin_fn(func),
        }
    }

    fn eval_builtin_fn(&mut self, func: &BuiltinFn) -> Result<Value, String> {
        match func {
            BuiltinFn::Abs(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                Ok(Value::Number(n.abs()))
            }
            BuiltinFn::Atn(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                Ok(Value::Number(n.atan()))
            }
            BuiltinFn::Cos(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                Ok(Value::Number(n.cos()))
            }
            BuiltinFn::Exp(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                Ok(Value::Number(n.exp()))
            }
            BuiltinFn::Int(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                Ok(Value::Number(n.floor()))
            }
            BuiltinFn::Log(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                Ok(Value::Number(n.ln()))
            }
            BuiltinFn::Rnd(expr) => {
                let _n = self.eval_expr(expr)?.as_number()?;
                // Simple Linear Congruential Generator
                self.rnd_seed = (self.rnd_seed * 16807.0) % 2147483647.0;
                if self.rnd_seed == 0.0 {
                    self.rnd_seed = 1.0;
                }
                Ok(Value::Number(self.rnd_seed / 2147483647.0))
            }
            BuiltinFn::Sgn(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                let result = if n > 0.0 {
                    1.0
                } else if n < 0.0 {
                    -1.0
                } else {
                    0.0
                };
                Ok(Value::Number(result))
            }
            BuiltinFn::Sin(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                Ok(Value::Number(n.sin()))
            }
            BuiltinFn::Sqr(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                Ok(Value::Number(n.sqrt()))
            }
            BuiltinFn::Tan(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                Ok(Value::Number(n.tan()))
            }
            BuiltinFn::Asc(expr) => {
                let s = self.eval_expr(expr)?.as_string()?;
                if s.is_empty() {
                    return Err("ASC of empty string".to_string());
                }
                Ok(Value::Number(s.bytes().next().unwrap() as f64))
            }
            BuiltinFn::Chr(expr) => {
                let n = self.eval_expr(expr)?.as_number()? as u8;
                Ok(Value::String((n as char).to_string()))
            }
            BuiltinFn::Left(str_expr, len_expr) => {
                let s = self.eval_expr(str_expr)?.as_string()?;
                let len = self.eval_expr(len_expr)?.as_number()? as usize;
                Ok(Value::String(s.chars().take(len).collect()))
            }
            BuiltinFn::Len(expr) => {
                let s = self.eval_expr(expr)?.as_string()?;
                Ok(Value::Number(s.len() as f64))
            }
            BuiltinFn::Mid(str_expr, start_expr, len_expr) => {
                let s = self.eval_expr(str_expr)?.as_string()?;
                let start = self.eval_expr(start_expr)?.as_number()? as usize;
                let chars: Vec<char> = s.chars().collect();

                if start < 1 || start > chars.len() {
                    return Ok(Value::String(String::new()));
                }

                let len = if let Some(len_e) = len_expr {
                    self.eval_expr(len_e)?.as_number()? as usize
                } else {
                    chars.len()
                };

                let result: String = chars.iter().skip(start - 1).take(len).collect();

                Ok(Value::String(result))
            }
            BuiltinFn::Right(str_expr, len_expr) => {
                let s = self.eval_expr(str_expr)?.as_string()?;
                let len = self.eval_expr(len_expr)?.as_number()? as usize;
                let chars: Vec<char> = s.chars().collect();
                let start = if len >= chars.len() {
                    0
                } else {
                    chars.len() - len
                };
                Ok(Value::String(chars.iter().skip(start).collect()))
            }
            BuiltinFn::Str(expr) => {
                let n = self.eval_expr(expr)?.as_number()?;
                let s = if n >= 0.0 {
                    format!(" {}", n)
                } else {
                    format!("{}", n)
                };
                Ok(Value::String(s))
            }
            BuiltinFn::Val(expr) => {
                let s = self.eval_expr(expr)?.as_string()?;
                let n = s.trim().parse::<f64>().unwrap_or(0.0);
                Ok(Value::Number(n))
            }
            BuiltinFn::Fre(expr) => {
                let _ = self.eval_expr(expr)?;
                // Calculate free memory based on arrays
                // Base memory is 1475 bytes (matching original BASIC)
                // Each array has 7-byte overhead + 5 bytes per element
                let base_memory: usize = 1475;
                let used_memory: usize = self
                    .arrays
                    .values()
                    .map(|arr| 7 + arr.values.len() * 5)
                    .sum();
                let free = base_memory.saturating_sub(used_memory);
                Ok(Value::Number(free as f64))
            }
            BuiltinFn::Pos(expr) => {
                let _ = self.eval_expr(expr)?;
                // Return current print column position
                Ok(Value::Number(self.print_column as f64))
            }
            BuiltinFn::Peek(expr) => {
                let addr = self.eval_expr(expr)?.as_number()? as u16;
                // Return byte from memory or 0 if not set
                let value = self.memory.get(&addr).copied().unwrap_or(0);
                Ok(Value::Number(value as f64))
            }
        }
    }

    fn get_var(&mut self, var_ref: &VarRef) -> Result<Value, String> {
        match var_ref {
            VarRef::Simple(name) | VarRef::String(name) => Ok(self
                .variables
                .get(name)
                .cloned()
                .unwrap_or(Value::Number(0.0))),
            VarRef::Array(name, indices) => {
                // Check if this is actually a user-defined function call
                // In BASIC, FNA(X) could be either an array or a function
                if indices.len() == 1 && self.user_functions.contains_key(name) {
                    // It's a function call, not an array access
                    let (param, body) = self.user_functions.get(name).unwrap().clone();
                    let arg_value = self.eval_expr(&indices[0])?;

                    // Save current value of parameter (if any)
                    let old_value = self.variables.get(&param).cloned();

                    // Set parameter
                    self.variables.insert(param.clone(), arg_value);

                    // Evaluate function body
                    let result = self.eval_expr(&body)?;

                    // Restore old value
                    if let Some(old_val) = old_value {
                        self.variables.insert(param, old_val);
                    } else {
                        self.variables.remove(&param);
                    }

                    return Ok(result);
                }

                // It's an array access
                // Evaluate indices first (before borrowing array)
                let idx_vals: Result<Vec<usize>, String> = indices
                    .iter()
                    .map(|e| self.eval_expr(e)?.as_number().map(|n| n as usize))
                    .collect();
                let idx_vals = idx_vals?;

                let array = self
                    .arrays
                    .get(name)
                    .ok_or(format!("Undefined array: {}", name))?;

                let linear_idx = self.calc_array_index(&array.dims, &idx_vals)?;

                Ok(array
                    .values
                    .get(linear_idx)
                    .cloned()
                    .unwrap_or(Value::Number(0.0)))
            }
        }
    }

    fn set_var(&mut self, var_ref: &VarRef, value: Value) -> Result<(), String> {
        match var_ref {
            VarRef::Simple(name) | VarRef::String(name) => {
                self.variables.insert(name.clone(), value);
                Ok(())
            }
            VarRef::Array(name, indices) => {
                // Auto-dimension array if not yet defined
                if !self.arrays.contains_key(name) {
                    let default_dims = vec![11; indices.len()]; // 0-10 for each dimension
                    let total_size = default_dims.iter().product();
                    let default_value = if name.ends_with('$') {
                        Value::String(String::new())
                    } else {
                        Value::Number(0.0)
                    };
                    self.arrays.insert(
                        name.clone(),
                        Array {
                            dims: default_dims,
                            values: vec![default_value; total_size],
                        },
                    );
                }

                // Evaluate indices first (before borrowing array)
                let idx_vals: Result<Vec<usize>, String> = indices
                    .iter()
                    .map(|e| self.eval_expr(e)?.as_number().map(|n| n as usize))
                    .collect();
                let idx_vals = idx_vals?;

                // Calculate linear index (using immutable borrow)
                let linear_idx = {
                    let array = self
                        .arrays
                        .get(name)
                        .ok_or(format!("Undefined array: {}", name))?;
                    self.calc_array_index(&array.dims, &idx_vals)?
                };

                // Now mutate the array
                let array = self
                    .arrays
                    .get_mut(name)
                    .ok_or(format!("Undefined array: {}", name))?;

                if linear_idx < array.values.len() {
                    array.values[linear_idx] = value;
                    Ok(())
                } else {
                    Err("Array index out of bounds".to_string())
                }
            }
        }
    }

    fn calc_array_index(&self, dims: &[usize], indices: &[usize]) -> Result<usize, String> {
        if indices.len() != dims.len() {
            return Err("Wrong number of array indices".to_string());
        }

        let mut linear_idx = 0;
        let mut multiplier = 1;

        for i in (0..dims.len()).rev() {
            if indices[i] >= dims[i] {
                return Err("Array index out of bounds".to_string());
            }
            linear_idx += indices[i] * multiplier;
            multiplier *= dims[i];
        }

        Ok(linear_idx)
    }
}
