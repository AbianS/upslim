// ---------------------------------------------------------------------------
// Evaluation context — raw response data
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct EvalContext {
    pub status: Option<u16>,
    pub response_time_ms: u64,
    pub body: Option<String>,
    pub connected: Option<bool>,
}

// ---------------------------------------------------------------------------
// Condition evaluator
// ---------------------------------------------------------------------------

/// Evaluates all conditions with AND semantics.
/// Returns `(success, failure_reason)`.
pub fn evaluate(conditions: &[String], ctx: &EvalContext) -> (bool, Option<String>) {
    for cond in conditions {
        if let Some(reason) = check_condition(cond.trim(), ctx) {
            return (false, Some(reason));
        }
    }
    (true, None)
}

/// Returns `Some(failure_reason)` if the condition fails, `None` if it passes.
fn check_condition(cond: &str, ctx: &EvalContext) -> Option<String> {
    // Parse operator (order matters: >= before >, <= before <, != before ==)
    let operators = [">=", "<=", "!=", "==", ">", "<"];
    let (lhs, op, rhs) = 'parse: {
        for &op in &operators {
            if let Some(pos) = cond.find(op) {
                let lhs = cond[..pos].trim();
                let rhs = cond[pos + op.len()..].trim();
                break 'parse (lhs, op, rhs);
            }
        }
        return Some(format!("invalid condition (no operator found): '{cond}'"));
    };

    // Resolve variable
    let resolved = match resolve_variable(lhs, ctx) {
        Ok(v) => v,
        Err(e) => return Some(e),
    };

    // Compare
    match compare(&resolved, op, rhs) {
        Ok(true) => None, // condition passes
        Ok(false) => Some(format!("{lhs} {op} {rhs}: got '{resolved}'")),
        Err(e) => Some(format!("{lhs} {op} {rhs}: {e}")),
    }
}

/// Resolves a variable from the context.
fn resolve_variable(var: &str, ctx: &EvalContext) -> Result<String, String> {
    if var == "[STATUS]" {
        return ctx
            .status
            .map(|s| s.to_string())
            .ok_or_else(|| "[STATUS] not available (non-HTTP check)".to_owned());
    }

    if var == "[RESPONSE_TIME]" {
        return Ok(ctx.response_time_ms.to_string());
    }

    if var == "[CONNECTED]" {
        return ctx
            .connected
            .map(|c| c.to_string())
            .ok_or_else(|| "[CONNECTED] not available (non-TCP check)".to_owned());
    }

    if var == "[BODY]" {
        return ctx
            .body
            .clone()
            .ok_or_else(|| "[BODY] not available".to_owned());
    }

    // JSON dot-path: [BODY].some.path.here
    if let Some(path) = var.strip_prefix("[BODY].") {
        let body = ctx
            .body
            .as_deref()
            .ok_or_else(|| "[BODY] not available".to_owned())?;
        return resolve_json_path(body, path);
    }

    Err(format!("unknown variable '{var}'"))
}

/// Resolves a JSON dot-path: "status" → "/status", "data.items" → "/data/items"
fn resolve_json_path(body: &str, path: &str) -> Result<String, String> {
    let pointer = format!("/{}", path.replace('.', "/"));
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| format!("[BODY] is not valid JSON (path: .{path})"))?;
    match v.pointer(&pointer) {
        None => Err(format!("[BODY].{path} not found in JSON response")),
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        Some(serde_json::Value::Bool(b)) => Ok(b.to_string()),
        Some(serde_json::Value::Number(n)) => Ok(n.to_string()),
        Some(serde_json::Value::Null) => Ok("null".to_owned()),
        Some(other) => Ok(other.to_string()),
    }
}

/// Compares a resolved value against a literal using the given operator.
fn compare(resolved: &str, op: &str, rhs: &str) -> Result<bool, String> {
    // Numeric comparison
    if let (Ok(l), Ok(r)) = (resolved.parse::<f64>(), rhs.parse::<f64>()) {
        return Ok(match op {
            "==" => (l - r).abs() < f64::EPSILON,
            "!=" => (l - r).abs() >= f64::EPSILON,
            ">" => l > r,
            ">=" => l >= r,
            "<" => l < r,
            "<=" => l <= r,
            _ => return Err(format!("unknown operator '{op}'")),
        });
    }

    // String comparison
    Ok(match op {
        "==" => resolved == rhs,
        "!=" => resolved != rhs,
        ">" => resolved > rhs,
        ">=" => resolved >= rhs,
        "<" => resolved < rhs,
        "<=" => resolved <= rhs,
        _ => return Err(format!("unknown operator '{op}'")),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_http(status: u16, rt_ms: u64, body: &str) -> EvalContext {
        EvalContext {
            status: Some(status),
            response_time_ms: rt_ms,
            body: Some(body.to_owned()),
            connected: None,
        }
    }

    #[test]
    fn status_eq_passes() {
        let ctx = ctx_http(200, 100, "");
        let (ok, reason) = evaluate(&["[STATUS] == 200".to_owned()], &ctx);
        assert!(ok, "reason: {:?}", reason);
    }

    #[test]
    fn status_eq_fails() {
        let ctx = ctx_http(503, 100, "");
        let (ok, reason) = evaluate(&["[STATUS] == 200".to_owned()], &ctx);
        assert!(!ok);
        assert!(reason.unwrap().contains("503"));
    }

    #[test]
    fn response_time_lt_passes() {
        let ctx = ctx_http(200, 100, "");
        let (ok, _) = evaluate(&["[RESPONSE_TIME] < 500".to_owned()], &ctx);
        assert!(ok);
    }

    #[test]
    fn response_time_lt_fails() {
        let ctx = ctx_http(200, 600, "");
        let (ok, reason) = evaluate(&["[RESPONSE_TIME] < 500".to_owned()], &ctx);
        assert!(!ok);
        assert!(reason.is_some());
    }

    #[test]
    fn body_json_path_passes() {
        let ctx = ctx_http(200, 50, r#"{"status": "healthy"}"#);
        let (ok, reason) = evaluate(&["[BODY].status == healthy".to_owned()], &ctx);
        assert!(ok, "reason: {:?}", reason);
    }

    #[test]
    fn body_json_path_fails() {
        let ctx = ctx_http(200, 50, r#"{"status": "degraded"}"#);
        let (ok, _) = evaluate(&["[BODY].status == healthy".to_owned()], &ctx);
        assert!(!ok);
    }

    #[test]
    fn body_json_bool() {
        let ctx = ctx_http(200, 50, r#"{"healthy": true}"#);
        let (ok, reason) = evaluate(&["[BODY].healthy == true".to_owned()], &ctx);
        assert!(ok, "reason: {:?}", reason);
    }

    #[test]
    fn connected_true_passes() {
        let ctx = EvalContext {
            connected: Some(true),
            ..Default::default()
        };
        let (ok, _) = evaluate(&["[CONNECTED] == true".to_owned()], &ctx);
        assert!(ok);
    }

    #[test]
    fn multiple_conditions_all_pass() {
        let ctx = ctx_http(200, 100, r#"{"status": "up"}"#);
        let conditions = vec![
            "[STATUS] == 200".to_owned(),
            "[RESPONSE_TIME] < 500".to_owned(),
            "[BODY].status == up".to_owned(),
        ];
        let (ok, reason) = evaluate(&conditions, &ctx);
        assert!(ok, "reason: {:?}", reason);
    }

    #[test]
    fn multiple_conditions_first_fails() {
        let ctx = ctx_http(503, 100, r#"{"status": "up"}"#);
        let conditions = vec![
            "[STATUS] == 200".to_owned(),
            "[RESPONSE_TIME] < 500".to_owned(),
        ];
        let (ok, reason) = evaluate(&conditions, &ctx);
        assert!(!ok);
        assert!(reason.unwrap().contains("503"));
    }
}
