use chatw_plugin_sdk::{export_chatw_plugin, host_log, PluginRequest, PluginResponse};
use serde_json::json;

/// Safe recursive-descent arithmetic expression parser (zero eval, pure Rust compiled to WASM).
struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { bytes: input.as_bytes(), pos: 0 }
    }
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }
    fn parse_expr(&mut self) -> Result<f64, &'static str> {
        let mut val = self.parse_term()?;
        while let Some(op) = self.peek() {
            if op == b'+' || op == b'-' {
                self.pos += 1;
                let rhs = self.parse_term()?;
                val = if op == b'+' { val + rhs } else { val - rhs };
            } else {
                break;
            }
        }
        Ok(val)
    }
    fn parse_term(&mut self) -> Result<f64, &'static str> {
        let mut val = self.parse_unary()?;
        while let Some(op) = self.peek() {
            if op == b'*' || op == b'/' || op == b'%' {
                self.pos += 1;
                let rhs = self.parse_unary()?;
                val = match op {
                    b'*' => val * rhs,
                    b'/' => {
                        if rhs == 0.0 {
                            return Err("division_by_zero");
                        }
                        val / rhs
                    }
                    b'%' => {
                        if rhs == 0.0 {
                            return Err("division_by_zero");
                        }
                        val % rhs
                    }
                    _ => val,
                };
            } else {
                break;
            }
        }
        Ok(val)
    }
    fn parse_unary(&mut self) -> Result<f64, &'static str> {
        if let Some(b'-') = self.peek() {
            self.pos += 1;
            return Ok(-self.parse_unary()?);
        }
        if let Some(b'+') = self.peek() {
            self.pos += 1;
            return self.parse_unary();
        }
        self.parse_primary()
    }
    fn parse_primary(&mut self) -> Result<f64, &'static str> {
        self.skip_ws();
        if let Some(b'(') = self.peek() {
            self.pos += 1;
            let v = self.parse_expr()?;
            if self.peek() != Some(b')') {
                return Err("missing_closing_paren");
            }
            self.pos += 1;
            return Ok(v);
        }
        let start = self.pos;
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'.')
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err("unexpected_token");
        }
        let num_str = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| "invalid_utf8")?;
        num_str.parse::<f64>().map_err(|_| "invalid_number")
    }
}

fn eval_expression(expr: &str) -> Result<f64, &'static str> {
    let trimmed = expr.trim();
    if trimmed.is_empty() || trimmed.len() > 256 {
        return Err("invalid_expression_length");
    }
    let mut p = Parser::new(trimmed);
    let out = p.parse_expr()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err("trailing_characters");
    }
    if !out.is_finite() {
        return Err("non_finite_result");
    }
    Ok(out)
}

fn handle(req: PluginRequest) -> PluginResponse {
    match req.kind.as_str() {
        "command" => {
            let expr = req.args.as_deref().unwrap_or("").trim();
            match eval_expression(expr) {
                Ok(val) => {
                    let formatted = if (val - val.round()).abs() < 1e-10 {
                        format!("{}", val.round() as i64)
                    } else {
                        format!("{:.6}", val)
                            .trim_end_matches('0')
                            .trim_end_matches('.')
                            .to_string()
                    };
                    host_log(&format!("WASM Calc: {expr} = {formatted}"));
                    PluginResponse::ok_data(
                        format!("WASM Calc: {expr} = {formatted}"),
                        json!({ "expression": expr, "result": val, "formatted": formatted, "engine": "rust-wasm" }),
                    )
                }
                Err(code) => PluginResponse::err(format!("calc_error:{code}")),
            }
        }
        "render" => PluginResponse::ok_data(
            "Rust WASM Calculator ready",
            json!({ "engine": "rust-wasm", "version": "1.0.0" }),
        ),
        _ => PluginResponse::ok_text("calc-wasm-rust noop"),
    }
}

export_chatw_plugin!(handle);
