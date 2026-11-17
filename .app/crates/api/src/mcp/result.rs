//! Формирование JSON-RPC 2.0 ответов инструментов MCP.

use serde_json::{json, Value};

pub fn mcp_tool_result(text: &str) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": false
    })
}

pub fn mcp_tool_error(text: &str) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": true
    })
}
