//! HTTP-прокси запросов в запущенный polygram-api.exe.

use serde_json::Value;
use std::error::Error;

/// POST JSON-команды в /api/commands запущенного polygram.exe.
pub fn proxy_post_command(base: &str, cmd_json: &str) -> Result<String, Box<dyn Error>> {
    serde_json::from_str::<Value>(cmd_json).map_err(|e| format!("неверный JSON команды: {e}"))?;
    let resp = ureq::post(&format!("{base}/api/commands"))
        .set("Content-Type", "application/json")
        .send_bytes(cmd_json.as_bytes())?;
    Ok(resp.into_string()?.trim().to_owned())
}

/// GET из HTTP API polygram.exe.
pub fn proxy_get(base: &str, path: &str) -> Result<Value, Box<dyn Error>> {
    let resp = ureq::get(&format!("{base}{path}")).call()?;
    let body: Value = serde_json::from_str(&resp.into_string()?)?;
    Ok(body)
}

/// POST JSON в произвольный путь HTTP API polygram.exe.
pub fn proxy_post_json(base: &str, path: &str, body: &str) -> Result<String, Box<dyn Error>> {
    let resp = ureq::post(&format!("{base}{path}"))
        .set("Content-Type", "application/json")
        .send_string(body)?;
    Ok(resp.into_string()?.trim().to_owned())
}
