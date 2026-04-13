/// A simple calculator with history tracking.
pub struct Calculator {
    history: Vec<f64>,
}

impl Calculator {
    /// Create a new calculator.
    pub fn new() -> Self {
        Calculator { history: Vec::new() }
    }

    /// Add two numbers and record the result.
    pub fn add(&mut self, a: f64, b: f64) -> f64 {
        let result = a + b;
        self.history.push(result);
        result
    }

    /// Multiply two numbers and record the result.
    pub fn multiply(&mut self, a: f64, b: f64) -> f64 {
        let result = a * b;
        self.history.push(result);
        result
    }

    /// Get the last N results.
    pub fn recent(&self, n: usize) -> &[f64] {
        let start = self.history.len().saturating_sub(n);
        &self.history[start..]
    }
}

/// Format a result for display.
pub fn format_result(value: f64) -> String {
    format!("{:.2}", value)
}

pub const MAX_HISTORY: usize = 100;
