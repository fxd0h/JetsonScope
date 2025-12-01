use tiny_http::Request;

/// Simple auth for metrics/debug endpoints.
/// If the provided env var is set, require `Authorization: Bearer <token>`.
/// Otherwise allow all.
pub fn authorize_request(req: &Request, env_var: &str) -> bool {
    let expected = match std::env::var(env_var) {
        Ok(t) if !t.is_empty() => t,
        _ => return true,
    };
    for header in req.headers() {
        if header.field.equiv("Authorization") {
            let val = header.value.as_str();
            if let Some(token) = val.strip_prefix("Bearer ").or_else(|| val.strip_prefix("bearer ")) {
                if token == expected {
                    return true;
                }
            }
        }
    }
    false
}
