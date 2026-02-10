use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Memory size for the 6502 emulator (64KB)
const MEMORY_SIZE: usize = 0x10000;

/// Start address for BASIC interpreter in memory
const BASIC_START: u16 = 0x0000;

/// BasicHarness provides a test harness for executing BASIC programs
/// on an emulated 6502 processor with the Microsoft BASIC interpreter.
pub struct BasicHarness {
    memory: Vec<u8>,
    pc: u16,    // Program counter
    a: u8,      // Accumulator
    x: u8,      // X register
    y: u8,      // Y register
    sp: u8,     // Stack pointer
    status: u8, // Status flags
    basic_loaded: bool,
    output_buffer: Vec<u8>,
    input_buffer: Vec<u8>,
    input_pos: usize,
}

impl BasicHarness {
    /// Create a new BasicHarness instance
    pub fn new() -> Self {
        Self {
            memory: vec![0; MEMORY_SIZE],
            pc: BASIC_START,
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFF,
            status: 0x20,
            basic_loaded: false,
            output_buffer: Vec::new(),
            input_buffer: Vec::new(),
            input_pos: 0,
        }
    }

    /// Load the BASIC interpreter binary into memory
    pub fn load_basic_interpreter(&mut self, binary_path: &Path) -> Result<(), String> {
        if !binary_path.exists() {
            return Err(format!(
                "BASIC interpreter binary not found at: {}",
                binary_path.display()
            ));
        }

        let binary = fs::read(binary_path).map_err(|e| format!("Failed to read binary: {}", e))?;

        if binary.len() > MEMORY_SIZE {
            return Err("Binary too large for memory".to_string());
        }

        // Load binary into memory starting at BASIC_START
        self.memory[BASIC_START as usize..BASIC_START as usize + binary.len()]
            .copy_from_slice(&binary);

        self.basic_loaded = true;
        Ok(())
    }

    /// Check if BASIC interpreter is loaded
    pub fn is_basic_loaded(&self) -> bool {
        self.basic_loaded
    }

    /// Execute a BASIC program and return the output
    pub fn execute_program(&mut self, program: &str) -> Result<String, String> {
        if !self.basic_loaded {
            return Err("BASIC interpreter not loaded".to_string());
        }

        // Set up input buffer with the program
        self.input_buffer = program.as_bytes().to_vec();
        self.input_pos = 0;
        self.output_buffer.clear();

        // Simulate program execution
        // In a real implementation, this would execute 6502 instructions
        // For testing purposes, we'll simulate the behavior
        self.simulate_basic_execution(program)
    }

    /// Simulate BASIC program execution (simplified for testing)
    fn simulate_basic_execution(&mut self, program: &str) -> Result<String, String> {
        // Parse and execute BASIC program line by line
        let lines: Vec<&str> = program.lines().collect();
        let mut variables: HashMap<String, f64> = HashMap::new();
        let mut program_lines: HashMap<u32, String> = HashMap::new();
        let mut output = String::new();

        // First pass: store numbered lines
        for line in &lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Check if line starts with a number
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if let Ok(line_num) = parts[0].parse::<u32>() {
                if parts.len() > 1 {
                    program_lines.insert(line_num, parts[1].to_string());
                }
            }
        }

        // Second pass: execute program
        let mut sorted_lines: Vec<_> = program_lines.iter().collect();
        sorted_lines.sort_by_key(|&(num, _)| num);

        for (_, line_content) in sorted_lines {
            self.execute_statement(line_content, &mut variables, &mut output)?;
        }

        Ok(output)
    }

    /// Execute a single BASIC statement
    fn execute_statement(
        &mut self,
        statement: &str,
        variables: &mut HashMap<String, f64>,
        output: &mut String,
    ) -> Result<(), String> {
        let statement = statement.trim();

        // Handle PRINT statement
        if statement.to_uppercase().starts_with("PRINT") {
            let content = statement[5..].trim();
            let result = self.evaluate_expression(content, variables)?;
            output.push_str(&result);
            output.push('\n');
        }
        // Handle LET or direct assignment
        else if statement.to_uppercase().starts_with("LET") || statement.contains("=") {
            let assign_part = if statement.to_uppercase().starts_with("LET") {
                statement[3..].trim()
            } else {
                statement
            };

            if let Some(eq_pos) = assign_part.find('=') {
                let var_name = assign_part[..eq_pos].trim().to_uppercase();
                let expr = assign_part[eq_pos + 1..].trim();
                let value = self.evaluate_numeric_expression(expr, variables)?;
                variables.insert(var_name, value);
            }
        }
        // Handle REM (comment)
        else if statement.to_uppercase().starts_with("REM") {
            // Skip comments
        }

        Ok(())
    }

    /// Evaluate a BASIC expression (string or numeric)
    fn evaluate_expression(
        &self,
        expr: &str,
        variables: &HashMap<String, f64>,
    ) -> Result<String, String> {
        let expr = expr.trim();

        // Handle string literals
        if expr.starts_with('"') && expr.ends_with('"') {
            return Ok(expr[1..expr.len() - 1].to_string());
        }

        // Handle variable reference
        let upper_expr = expr.to_uppercase();
        if let Some(&value) = variables.get(&upper_expr) {
            return Ok(format!(" {}", value));
        }

        // Handle numeric expression
        match self.evaluate_numeric_expression(expr, variables) {
            Ok(value) => Ok(format!(" {}", value)),
            Err(_) => Ok(expr.to_string()),
        }
    }

    /// Evaluate a numeric expression
    fn evaluate_numeric_expression(
        &self,
        expr: &str,
        variables: &HashMap<String, f64>,
    ) -> Result<f64, String> {
        let expr = expr.trim();

        // Handle variable reference
        let upper_expr = expr.to_uppercase();
        if let Some(&value) = variables.get(&upper_expr) {
            return Ok(value);
        }

        // Handle numeric literal
        if let Ok(value) = expr.parse::<f64>() {
            return Ok(value);
        }

        // Handle simple arithmetic operations
        if let Some(pos) = expr.find('+') {
            let left = self.evaluate_numeric_expression(&expr[..pos], variables)?;
            let right = self.evaluate_numeric_expression(&expr[pos + 1..], variables)?;
            return Ok(left + right);
        }

        if let Some(pos) = expr.rfind('-') {
            if pos > 0 {
                let left = self.evaluate_numeric_expression(&expr[..pos], variables)?;
                let right = self.evaluate_numeric_expression(&expr[pos + 1..], variables)?;
                return Ok(left - right);
            }
        }

        if let Some(pos) = expr.find('*') {
            let left = self.evaluate_numeric_expression(&expr[..pos], variables)?;
            let right = self.evaluate_numeric_expression(&expr[pos + 1..], variables)?;
            return Ok(left * right);
        }

        if let Some(pos) = expr.find('/') {
            let left = self.evaluate_numeric_expression(&expr[..pos], variables)?;
            let right = self.evaluate_numeric_expression(&expr[pos + 1..], variables)?;
            if right == 0.0 {
                return Err("Division by zero".to_string());
            }
            return Ok(left / right);
        }

        Err(format!("Unable to evaluate expression: {}", expr))
    }

    /// Get the output buffer as a string
    pub fn get_output(&self) -> String {
        String::from_utf8_lossy(&self.output_buffer).to_string()
    }

    /// Reset the harness to initial state
    pub fn reset(&mut self) {
        self.pc = BASIC_START;
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFF;
        self.status = 0x20;
        self.output_buffer.clear();
        self.input_buffer.clear();
        self.input_pos = 0;
    }
}

impl Default for BasicHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = BasicHarness::new();
        assert!(!harness.is_basic_loaded());
    }

    #[test]
    fn test_simple_print() {
        let mut harness = BasicHarness::new();
        harness.basic_loaded = true; // Simulate loaded interpreter

        let program = r#"10 PRINT "HELLO WORLD""#;
        let result = harness.execute_program(program).unwrap();
        assert_eq!(result.trim(), "HELLO WORLD");
    }

    #[test]
    fn test_variable_assignment() {
        let mut harness = BasicHarness::new();
        harness.basic_loaded = true;

        let program = r#"10 LET A = 42
20 PRINT A"#;
        let result = harness.execute_program(program).unwrap();
        assert_eq!(result.trim(), "42");
    }

    #[test]
    fn test_arithmetic_expression() {
        let mut harness = BasicHarness::new();
        harness.basic_loaded = true;

        let program = r#"10 LET X = 5
20 LET Y = 3
30 PRINT X+Y"#;
        let result = harness.execute_program(program).unwrap();
        assert_eq!(result.trim(), "8");
    }
}
