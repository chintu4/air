use super::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct CalculatorTool;

impl CalculatorTool {
    pub fn new() -> Self {
        Self
    }
    
    fn evaluate_expression(&self, expr: &str) -> Result<f64> {
        // Simple expression evaluator - supports basic math operations
        let expr_clean = expr.replace(" ", "");
        
        // Handle percentage operations like "15% of 200" or "15%of200"
        if let Some(result) = self.try_parse_percentage_expression(expr) {
            return Ok(result);
        }
        
        // Handle basic operations
        if let Some(result) = self.try_parse_simple_expression(&expr_clean) {
            return Ok(result);
        }
        
        Err(anyhow!("Complex expressions not yet supported: {}", expr))
    }
    
    fn try_parse_percentage_expression(&self, expr: &str) -> Option<f64> {
        let expr_lower = expr.to_lowercase();
        
        // Handle "X% of Y" pattern
        if expr_lower.contains("% of") {
            let parts: Vec<&str> = expr_lower.split("% of").collect();
            if parts.len() == 2 {
                if let (Ok(percentage), Ok(total)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                    return Some((percentage / 100.0) * total);
                }
            }
        }
        
        // Handle "X%of Y" pattern (no space)
        if expr_lower.contains("%of") {
            let parts: Vec<&str> = expr_lower.split("%of").collect();
            if parts.len() == 2 {
                if let (Ok(percentage), Ok(total)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                    return Some((percentage / 100.0) * total);
                }
            }
        }
        
        None
    }
    
    fn try_parse_simple_expression(&self, expr: &str) -> Option<f64> {
        // Handle single numbers
        if let Ok(num) = expr.parse::<f64>() {
            return Some(num);
        }
        
        // Handle basic binary operations
        for op in ["+", "-", "*", "/", "^"].iter() {
            if let Some(pos) = expr.rfind(op) {
                let left = &expr[..pos];
                let right = &expr[pos + 1..];
                
                if let (Ok(l), Ok(r)) = (left.parse::<f64>(), right.parse::<f64>()) {
                    return match *op {
                        "+" => Some(l + r),
                        "-" => Some(l - r),
                        "*" => Some(l * r),
                        "/" => if r != 0.0 { Some(l / r) } else { None },
                        "^" => Some(l.powf(r)),
                        _ => None,
                    };
                }
            }
        }
        
        None
    }
    
    fn calculate_statistics(&self, numbers: &[f64]) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        
        if numbers.is_empty() {
            return stats;
        }
        
        let sum: f64 = numbers.iter().sum();
        let count = numbers.len() as f64;
        let mean = sum / count;
        
        stats.insert("sum".to_string(), sum);
        stats.insert("count".to_string(), count);
        stats.insert("mean".to_string(), mean);
        stats.insert("min".to_string(), numbers.iter().fold(f64::INFINITY, |a, &b| a.min(b)));
        stats.insert("max".to_string(), numbers.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)));
        
        // Calculate variance and standard deviation
        let variance = numbers.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / count;
        stats.insert("variance".to_string(), variance);
        stats.insert("std_dev".to_string(), variance.sqrt());
        
        // Calculate median
        let mut sorted = numbers.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = if sorted.len() % 2 == 0 {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };
        stats.insert("median".to_string(), median);
        
        stats
    }
}

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }
    
    fn description(&self) -> &str {
        "Mathematical calculations: arithmetic, statistics, conversions"
    }
    
    fn available_functions(&self) -> Vec<String> {
        vec![
            "calculate".to_string(),
            "statistics".to_string(),
            "convert_units".to_string(),
            "factorial".to_string(),
            "percentage".to_string(),
        ]
    }
    
    async fn execute(&self, function: &str, args: Value) -> Result<ToolResult> {
        match function {
            "calculate" => {
                let expression = args["expression"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'expression' parameter"))?;
                
                match self.evaluate_expression(expression) {
                    Ok(result) => Ok(ToolResult {
                        success: true,
                        result: format!("{} = {}", expression, result),
                        metadata: Some(json!({
                            "expression": expression,
                            "result": result
                        })),
                    }),
                    Err(e) => Ok(ToolResult {
                        success: false,
                        result: format!("Calculation error: {}", e),
                        metadata: None,
                    })
                }
            }
            
            "statistics" => {
                let numbers: Vec<f64> = args["numbers"].as_array()
                    .ok_or_else(|| anyhow!("Missing 'numbers' array parameter"))?
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0))
                    .collect();
                
                let stats = self.calculate_statistics(&numbers);
                
                let result = format!(
                    "Statistics for {} numbers:\n\
                     Sum: {:.2}\n\
                     Mean: {:.2}\n\
                     Median: {:.2}\n\
                     Min: {:.2}\n\
                     Max: {:.2}\n\
                     Std Dev: {:.2}",
                    stats.get("count").unwrap_or(&0.0),
                    stats.get("sum").unwrap_or(&0.0),
                    stats.get("mean").unwrap_or(&0.0),
                    stats.get("median").unwrap_or(&0.0),
                    stats.get("min").unwrap_or(&0.0),
                    stats.get("max").unwrap_or(&0.0),
                    stats.get("std_dev").unwrap_or(&0.0)
                );
                
                Ok(ToolResult {
                    success: true,
                    result,
                    metadata: Some(json!(stats)),
                })
            }
            
            "factorial" => {
                let n = args["number"].as_u64()
                    .ok_or_else(|| anyhow!("Missing 'number' parameter"))?;
                
                if n > 20 {
                    return Ok(ToolResult {
                        success: false,
                        result: "Factorial too large (max 20)".to_string(),
                        metadata: None,
                    });
                }
                
                let mut result = 1u64;
                for i in 1..=n {
                    result *= i;
                }
                
                Ok(ToolResult {
                    success: true,
                    result: format!("{}! = {}", n, result),
                    metadata: Some(json!({"input": n, "result": result})),
                })
            }
            
            "percentage" => {
                let value = args["value"].as_f64()
                    .ok_or_else(|| anyhow!("Missing 'value' parameter"))?;
                let total = args["total"].as_f64()
                    .ok_or_else(|| anyhow!("Missing 'total' parameter"))?;
                
                if total == 0.0 {
                    return Ok(ToolResult {
                        success: false,
                        result: "Cannot calculate percentage of zero".to_string(),
                        metadata: None,
                    });
                }
                
                let percentage = (value / total) * 100.0;
                
                Ok(ToolResult {
                    success: true,
                    result: format!("{} is {:.2}% of {}", value, percentage, total),
                    metadata: Some(json!({
                        "value": value,
                        "total": total,
                        "percentage": percentage
                    })),
                })
            }
            
            "convert_units" => {
                let value = args["value"].as_f64()
                    .ok_or_else(|| anyhow!("Missing 'value' parameter"))?;
                let from_unit = args["from"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'from' parameter"))?;
                let to_unit = args["to"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'to' parameter"))?;
                
                // Simple unit conversions
                let result = match (from_unit, to_unit) {
                    ("celsius", "fahrenheit") => value * 9.0 / 5.0 + 32.0,
                    ("fahrenheit", "celsius") => (value - 32.0) * 5.0 / 9.0,
                    ("meters", "feet") => value * 3.28084,
                    ("feet", "meters") => value / 3.28084,
                    ("kg", "pounds") => value * 2.20462,
                    ("pounds", "kg") => value / 2.20462,
                    _ => return Ok(ToolResult {
                        success: false,
                        result: format!("Conversion from {} to {} not supported", from_unit, to_unit),
                        metadata: None,
                    })
                };
                
                Ok(ToolResult {
                    success: true,
                    result: format!("{} {} = {:.4} {}", value, from_unit, result, to_unit),
                    metadata: Some(json!({
                        "input_value": value,
                        "from_unit": from_unit,
                        "to_unit": to_unit,
                        "result": result
                    })),
                })
            }
            
            _ => Err(anyhow!("Unknown function: {}", function))
        }
    }
}
