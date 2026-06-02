use anyhow::Result;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::ai::providers::LlmProvider;
use crate::overlay::cursor::OverlayCommand;
use crate::screen::capture::ScreenCapture;
use crate::voice::tts::TtsProvider;

pub const PORT: u16 = 32123;

#[derive(Clone)]
pub struct AppState {
    pub overlay_tx: std::sync::mpsc::Sender<OverlayCommand>,
    pub screen: Arc<tokio::sync::Mutex<Box<dyn ScreenCapture>>>,
    pub tts: Arc<tokio::sync::Mutex<Box<dyn TtsProvider>>>,
    pub llm: Arc<tokio::sync::Mutex<Box<dyn LlmProvider>>>,
    pub auth_token: String,
    pub event_tx: broadcast::Sender<Value>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    tools: Vec<String>,
    version: String,
}

pub async fn start_server(state: AppState) -> Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/cursor", post(cursor))
        .route("/cursors", post(cursors))
        .route("/scribble", post(scribble))
        .route("/highlight", post(highlight))
        .route("/rectangle", post(highlight))
        .route("/caption", post(caption))
        .route("/screenshot", post(screenshot))
        .route("/click", post(click))
        .route("/speak", post(speak))
        .route("/notify", post(notify))
        .route("/clear", post(clear))
        .route("/events", get(events))
        .route("/mcp/tools", get(crate::bridge::mcp::list_tools))
        .route("/mcp/call", post(crate::bridge::mcp::call_tool))
        .route("/mcp/calls", post(crate::bridge::mcp::batch_call))
        .route("/mcp", post(crate::bridge::mcp::jsonrpc_handler))
        .route("/v1/chat/completions", post(inference_proxy_chat))
        .route("/v1/messages", post(inference_proxy_messages))
        .route("/v1/responses", post(inference_proxy_responses))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", PORT)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health(State(_state): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "tools": [
            "cursor", "cursors", "scribble", "highlight", "rectangle",
            "caption", "screenshot", "click", "speak", "notify", "clear"
        ],
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[derive(Deserialize)]
struct CursorRequest {
    x: f64,
    y: f64,
    #[serde(default)]
    label: Option<String>,
    #[serde(default = "default_accent")]
    accent: String,
}

fn default_accent() -> String { "blue".to_string() }

async fn cursor(State(state): State<AppState>, Json(req): Json<CursorRequest>) -> StatusCode {
    let point = crate::overlay::cursor::Point { x: req.x, y: req.y, label: req.label };
    let _ = state.overlay_tx.send(OverlayCommand::ShowCursor(point, req.accent, 0));
    let _ = state.event_tx.send(json!({"type": "cursor", "x": req.x, "y": req.y}));
    StatusCode::OK
}

#[derive(Deserialize)]
struct CursorsRequest {
    cursors: Vec<CursorRequest>,
}

async fn cursors(State(state): State<AppState>, Json(req): Json<CursorsRequest>) -> StatusCode {
    let points: Vec<crate::overlay::cursor::Point> = req.cursors.into_iter().map(|c| {
        crate::overlay::cursor::Point { x: c.x, y: c.y, label: c.label }
    }).collect();
    let accent = points.first().map(|_| "blue").unwrap_or("blue");
    let _ = state.overlay_tx.send(OverlayCommand::ShowCursors(points, accent.to_string(), 0));
    StatusCode::OK
}

#[derive(Deserialize)]
struct ScribbleRequest {
    points: Vec<crate::overlay::cursor::Point>,
    #[serde(default = "default_accent")]
    accent: String,
}

async fn scribble(State(state): State<AppState>, Json(req): Json<ScribbleRequest>) -> StatusCode {
    let _ = state.overlay_tx.send(OverlayCommand::ShowScribble(req.points, req.accent, 0));
    StatusCode::OK
}

#[derive(Deserialize)]
struct HighlightRequest {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    #[serde(default = "default_accent")]
    accent: String,
}

async fn highlight(State(state): State<AppState>, Json(req): Json<HighlightRequest>) -> StatusCode {
    let rect = crate::overlay::cursor::Rect { x: req.x, y: req.y, width: req.width, height: req.height };
    let _ = state.overlay_tx.send(OverlayCommand::ShowHighlight(rect, req.accent, 0));
    StatusCode::OK
}

#[derive(Deserialize)]
struct CaptionRequest {
    text: String,
    x: f64,
    y: f64,
    #[serde(default = "default_accent")]
    accent: String,
}

async fn caption(State(state): State<AppState>, Json(req): Json<CaptionRequest>) -> StatusCode {
    let _ = state.overlay_tx.send(OverlayCommand::ShowCaption(req.text, req.x, req.y, req.accent, 0));
    StatusCode::OK
}

async fn screenshot(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let result = state.screen.lock().await.capture_cursor_screen().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let b64 = base64_encode(&result.image_data);
    Ok(Json(json!({
        "image": b64,
        "width": result.width,
        "height": result.height,
        "is_cursor_screen": result.is_cursor_screen,
        "app_name": result.app_name,
    })))
}

#[derive(Deserialize)]
struct ClickRequest {
    x: i32,
    y: i32,
}

async fn click(Json(req): Json<ClickRequest>) -> StatusCode {
    let _ = std::process::Command::new("xdotool")
        .args(["click", "1", "--delay", "0"])
        .output();
    let _ = std::process::Command::new("xdotool")
        .args(["mousemove", &req.x.to_string(), &req.y.to_string()])
        .output();
    StatusCode::OK
}

#[derive(Deserialize)]
struct SpeakRequest {
    text: String,
}

async fn speak(State(state): State<AppState>, Json(req): Json<SpeakRequest>) -> StatusCode {
    let tts = state.tts.lock().await;
    match tts.synthesize(&req.text).await {
        Ok(audio) => {
            let _ = play_audio_async(&audio).await;
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    }
    StatusCode::OK
}

#[derive(Deserialize)]
struct NotifyRequest {
    title: String,
    body: String,
}

async fn notify(Json(req): Json<NotifyRequest>) -> StatusCode {
    let _ = std::process::Command::new("notify-send")
        .args([&req.title, &req.body])
        .output();
    StatusCode::OK
}

async fn clear(State(state): State<AppState>) -> StatusCode {
    let _ = state.overlay_tx.send(OverlayCommand::Clear);
    StatusCode::OK
}

async fn events(State(state): State<AppState>) -> axum::response::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    let rx = state.event_tx.subscribe();
    let stream = async_stream::stream! {
        let mut rx = rx;
        loop {
            match rx.recv().await {
                Ok(value) => {
                    let event = axum::response::sse::Event::default().data(value.to_string());
                    yield Ok(event);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };
    axum::response::Sse::new(stream)
}

async fn play_audio_async(data: &[u8]) -> Result<()> {
    let data = data.to_vec();
    tokio::task::spawn_blocking(move || {
        let cursor = std::io::Cursor::new(data);
        let source = rodio::Decoder::new(cursor)?;
        let (_stream, stream_handle) = rodio::OutputStream::try_default()?;
        let sink = rodio::Sink::try_new(&stream_handle)?;
        sink.append(source);
        sink.sleep_until_end();
        Ok::<(), anyhow::Error>(())
    }).await??;
    Ok(())
}

fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.write_char(TABLE[((triple >> 18) & 0x3F) as usize] as char).unwrap();
        out.write_char(TABLE[((triple >> 12) & 0x3F) as usize] as char).unwrap();
        if chunk.len() > 1 {
            out.write_char(TABLE[((triple >> 6) & 0x3F) as usize] as char).unwrap();
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.write_char(TABLE[(triple & 0x3F) as usize] as char).unwrap();
        } else {
            out.push('=');
        }
    }
    out
}

async fn inference_proxy_chat(State(state): State<AppState>, body: Json<Value>) -> Result<Json<Value>, StatusCode> {
    let body = body.0;
    let messages = body.get("messages")
        .and_then(|m| m.as_array())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let mut llm_messages = Vec::new();
    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user").to_string();
        let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
        llm_messages.push(crate::ai::providers::LlmMessage { role, content });
    }

    let model = body.get("model").and_then(|m| m.as_str()).map(String::from);
    let max_tokens = body.get("max_tokens").and_then(|m| m.as_u64()).map(|t| t as u32);
    let temperature = body.get("temperature").and_then(|t| t.as_f64()).map(|t| t as f32);

    let req = crate::ai::providers::LlmRequest {
        messages: llm_messages,
        model,
        max_tokens,
        temperature,
    };

    let llm = state.llm.lock().await;
    let resp = llm.complete(req).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "id": format!("chatcmpl-{}", uuid_short()),
        "object": "chat.completion",
        "model": resp.model,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": resp.content},
            "finish_reason": "stop"
        }]
    })))
}

async fn inference_proxy_messages(State(state): State<AppState>, body: Json<Value>) -> Result<Json<Value>, StatusCode> {
    let body = body.0;
    let messages = body.get("messages")
        .and_then(|m| m.as_array())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let mut llm_messages = Vec::new();
    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user").to_string();
        let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
        llm_messages.push(crate::ai::providers::LlmMessage { role, content });
    }

    let model = body.get("model").and_then(|m| m.as_str()).map(String::from);
    let max_tokens = body.get("max_tokens").and_then(|m| m.as_u64()).map(|t| t as u32);

    let req = crate::ai::providers::LlmRequest {
        messages: llm_messages,
        model,
        max_tokens,
        temperature: None,
    };

    let llm = state.llm.lock().await;
    let resp = llm.complete(req).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "id": format!("msg_{}", uuid_short()),
        "type": "message",
        "role": "assistant",
        "model": resp.model,
        "content": [{"type": "text", "text": resp.content}]
    })))
}

async fn inference_proxy_responses(State(state): State<AppState>, body: Json<Value>) -> Result<Json<Value>, StatusCode> {
    let body = body.0;
    let input = body.get("input")
        .and_then(|i| i.as_str())
        .unwrap_or("");

    let req = crate::ai::providers::LlmRequest {
        messages: vec![
            crate::ai::providers::LlmMessage { role: "user".to_string(), content: input.to_string() },
        ],
        model: None,
        max_tokens: None,
        temperature: None,
    };

    let llm = state.llm.lock().await;
    let resp = llm.complete(req).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "id": format!("resp_{}", uuid_short()),
        "object": "response",
        "output": [{"type": "message", "content": [{"type": "output_text", "text": resp.content}]}]
    })))
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("{:x}", ts)
}
