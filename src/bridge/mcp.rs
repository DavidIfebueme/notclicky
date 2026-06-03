use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use super::server::AppState;

pub async fn list_tools() -> Json<Value> {
    Json(json!({
        "tools": [
            {
                "name": "cursor",
                "description": "Show a cursor at a specific position on screen",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "x": {"type": "number", "description": "X coordinate"},
                        "y": {"type": "number", "description": "Y coordinate"},
                        "label": {"type": "string", "description": "Label for the cursor"},
                        "accent": {"type": "string", "description": "Color accent", "default": "blue"}
                    },
                    "required": ["x", "y"]
                }
            },
            {
                "name": "cursors",
                "description": "Show multiple cursors on screen",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "cursors": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "x": {"type": "number"},
                                    "y": {"type": "number"},
                                    "label": {"type": "string"}
                                },
                                "required": ["x", "y"]
                            }
                        }
                    },
                    "required": ["cursors"]
                }
            },
            {
                "name": "scribble",
                "description": "Draw a freehand path on screen",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "points": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "x": {"type": "number"},
                                    "y": {"type": "number"}
                                },
                                "required": ["x", "y"]
                            }
                        },
                        "accent": {"type": "string", "default": "blue"}
                    },
                    "required": ["points"]
                }
            },
            {
                "name": "highlight",
                "description": "Highlight a rectangular region on screen",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "x": {"type": "number"},
                        "y": {"type": "number"},
                        "width": {"type": "number"},
                        "height": {"type": "number"},
                        "accent": {"type": "string", "default": "blue"}
                    },
                    "required": ["x", "y", "width", "height"]
                }
            },
            {
                "name": "caption",
                "description": "Show a text caption at a position",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "text": {"type": "string"},
                        "x": {"type": "number"},
                        "y": {"type": "number"},
                        "accent": {"type": "string", "default": "blue"}
                    },
                    "required": ["text", "x", "y"]
                }
            },
            {
                "name": "screenshot",
                "description": "Capture a screenshot",
                "parameters": {"type": "object", "properties": {}}
            },
            {
                "name": "click",
                "description": "Click at a coordinate",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "x": {"type": "integer"},
                        "y": {"type": "integer"}
                    },
                    "required": ["x", "y"]
                }
            },
            {
                "name": "speak",
                "description": "Speak text aloud using TTS",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "text": {"type": "string"}
                    },
                    "required": ["text"]
                }
            },
            {
                "name": "notify",
                "description": "Show a desktop notification",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "body": {"type": "string"}
                    },
                    "required": ["title", "body"]
                }
            },
            {
                "name": "clear",
                "description": "Clear all overlays",
                "parameters": {"type": "object", "properties": {}}
            }
        ]
    }))
}

#[derive(Deserialize)]
pub struct McpCallRequest {
    pub tool: String,
    pub parameters: Option<Value>,
}

pub async fn call_tool(State(state): State<AppState>, Json(req): Json<McpCallRequest>) -> Result<Json<Value>, StatusCode> {
    let params = req.parameters.unwrap_or(json!({}));
    match req.tool.as_str() {
        "cursor" => {
            let x = params["x"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
            let y = params["y"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
            let label = params["label"].as_str().map(String::from);
            let accent = params["accent"].as_str().unwrap_or("blue").to_string();
            let point = crate::overlay::cursor::Point { x, y, label };
            let _ = state.overlay_tx.send(crate::overlay::cursor::OverlayCommand::ShowCursor(point, accent, 0));
            Ok(Json(json!({"success": true})))
        }
        "highlight" => {
            let x = params["x"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
            let y = params["y"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
            let width = params["width"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
            let height = params["height"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
            let accent = params["accent"].as_str().unwrap_or("blue").to_string();
            let rect = crate::overlay::cursor::Rect { x, y, width, height };
            let _ = state.overlay_tx.send(crate::overlay::cursor::OverlayCommand::ShowHighlight(rect, accent, 0));
            Ok(Json(json!({"success": true})))
        }
        "caption" => {
            let text = params["text"].as_str().ok_or(StatusCode::BAD_REQUEST)?.to_string();
            let x = params["x"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
            let y = params["y"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
            let accent = params["accent"].as_str().unwrap_or("blue").to_string();
            let _ = state.overlay_tx.send(crate::overlay::cursor::OverlayCommand::ShowCaption(text, x, y, accent, 0));
            Ok(Json(json!({"success": true})))
        }
        "screenshot" => {
            let result = state.screen.lock().await.capture_cursor_screen().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(json!({
                "success": true,
                "width": result.width,
                "height": result.height,
                "app_name": result.app_name,
            })))
        }
        "click" => {
            let x = params["x"].as_i64().ok_or(StatusCode::BAD_REQUEST)?;
            let y = params["y"].as_i64().ok_or(StatusCode::BAD_REQUEST)?;
            let _ = std::process::Command::new("xdotool")
                .args(["mousemove", &x.to_string(), &y.to_string(), "click", "1"])
                .output();
            Ok(Json(json!({"success": true})))
        }
        "speak" => {
            let text = params["text"].as_str().ok_or(StatusCode::BAD_REQUEST)?.to_string();
            let tts = state.tts.lock().await;
            match tts.synthesize(&text).await {
                Ok(_) => Ok(Json(json!({"success": true}))),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        "notify" => {
            let title = params["title"].as_str().unwrap_or("").to_string();
            let body = params["body"].as_str().unwrap_or("").to_string();
            let _ = std::process::Command::new("notify-send").args([&title, &body]).output();
            Ok(Json(json!({"success": true})))
        }
        "clear" => {
            let _ = state.overlay_tx.send(crate::overlay::cursor::OverlayCommand::Clear);
            Ok(Json(json!({"success": true})))
        }
        _ => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize)]
pub struct McpBatchRequest {
    pub calls: Vec<McpCallRequest>,
    #[serde(default = "default_delay")]
    pub delay_ms: u64,
}

fn default_delay() -> u64 { 100 }

pub async fn batch_call(State(state): State<AppState>, Json(req): Json<McpBatchRequest>) -> Json<Value> {
    let mut results = Vec::new();
    for call in req.calls {
        if !results.is_empty() && req.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(req.delay_ms)).await;
        }
        let params = call.parameters.unwrap_or(json!({}));
        let result = execute_tool(&state, &call.tool, &params).await;
        results.push(result);
    }
    Json(json!({"results": results}))
}

async fn execute_tool(state: &AppState, tool: &str, params: &Value) -> Value {
    match tool {
        "cursor" => {
            let x = params["x"].as_f64().unwrap_or(0.0);
            let y = params["y"].as_f64().unwrap_or(0.0);
            let label = params["label"].as_str().map(String::from);
            let accent = params["accent"].as_str().unwrap_or("blue").to_string();
            let point = crate::overlay::cursor::Point { x, y, label };
            let _ = state.overlay_tx.send(crate::overlay::cursor::OverlayCommand::ShowCursor(point, accent, 0));
            json!({"tool": tool, "success": true})
        }
        "clear" => {
            let _ = state.overlay_tx.send(crate::overlay::cursor::OverlayCommand::Clear);
            json!({"tool": tool, "success": true})
        }
        _ => json!({"tool": tool, "success": false, "error": "unknown tool"}),
    }
}

#[derive(Deserialize)]
pub struct JsonRpcRequest {
    #[serde(rename = "jsonrpc")]
    pub _jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

pub async fn jsonrpc_handler(State(state): State<AppState>, Json(req): Json<JsonRpcRequest>) -> Json<Value> {
    match req.method.as_str() {
        "tools/list" => {
            let tools = list_tools().await;
            let tools_value = tools.0;
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": tools_value
            }))
        }
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let result = execute_tool(&state, tool_name, &arguments).await;
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": result
            }))
        }
        "initialize" => {
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "notclicky-bridge",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            }))
        }
        _ => {
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "error": {"code": -32601, "message": "Method not found"}
            }))
        }
    }
}
