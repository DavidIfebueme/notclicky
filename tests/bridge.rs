use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

struct StubScreenCapture;

#[async_trait::async_trait]
impl notclicky::screen::capture::ScreenCapture for StubScreenCapture {
    async fn capture_all(&self) -> anyhow::Result<Vec<notclicky::screen::capture::CaptureResult>> {
        Ok(vec![])
    }
    async fn capture_cursor_screen(&self) -> anyhow::Result<notclicky::screen::capture::CaptureResult> {
        anyhow::bail!("stub")
    }
    async fn capture_focused_window(&self) -> anyhow::Result<notclicky::screen::capture::CaptureResult> {
        anyhow::bail!("stub")
    }
}

struct StubTts;

#[async_trait::async_trait]
impl notclicky::voice::tts::TtsProvider for StubTts {
    async fn synthesize(&self, _text: &str) -> anyhow::Result<notclicky::voice::tts::AudioChunk> {
        Ok(vec![])
    }
    async fn synthesize_stream(
        &self,
        _text_stream: std::pin::Pin<Box<dyn futures::Stream<Item = String> + Send>>,
    ) -> anyhow::Result<notclicky::voice::tts::AudioStream> {
        anyhow::bail!("stub")
    }
}

struct StubLlm;

#[async_trait::async_trait]
impl notclicky::ai::providers::LlmProvider for StubLlm {
    async fn complete(&self, _req: notclicky::ai::providers::LlmRequest) -> anyhow::Result<notclicky::ai::providers::LlmResponse> {
        Ok(notclicky::ai::providers::LlmResponse {
            content: "stub".to_string(),
            model: "stub".to_string(),
        })
    }
    async fn stream(&self, _req: notclicky::ai::providers::LlmRequest) -> anyhow::Result<notclicky::ai::providers::LlmStream> {
        anyhow::bail!("stub")
    }
}

fn create_test_app() -> axum::Router {
    let (overlay_tx, _rx) = std::sync::mpsc::channel();
    let (event_tx, _) = tokio::sync::broadcast::channel(64);

    let state = notclicky::bridge::server::AppState {
        overlay_tx,
        screen: std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(StubScreenCapture))),
        tts: std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(StubTts))),
        llm: std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(StubLlm))),
        auth_token: "test-token".to_string(),
        event_tx,
    };

    notclicky::bridge::server::build_router(state)
}

async fn send(app: axum::Router, method: Method, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(path);
    let req = if let Some(b) = body {
        builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&b).unwrap()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    (status, val)
}

#[tokio::test]
async fn test_health() {
    let (status, body) = send(create_test_app(), Method::GET, "/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert!(body["tools"].is_array());
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn test_cursor() {
    let (status, _) = send(create_test_app(), Method::POST, "/cursor", Some(json!({
        "x": 100.0, "y": 200.0, "label": "Test", "accent": "blue"
    }))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_cursors() {
    let (status, _) = send(create_test_app(), Method::POST, "/cursors", Some(json!({
        "cursors": [{"x": 10.0, "y": 20.0, "label": "A"}, {"x": 30.0, "y": 40.0, "label": "B"}]
    }))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_scribble() {
    let (status, _) = send(create_test_app(), Method::POST, "/scribble", Some(json!({
        "points": [{"x": 1.0, "y": 2.0}, {"x": 3.0, "y": 4.0}], "accent": "orange"
    }))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_highlight() {
    let (status, _) = send(create_test_app(), Method::POST, "/highlight", Some(json!({
        "x": 50.0, "y": 60.0, "width": 200.0, "height": 100.0, "accent": "green"
    }))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_rectangle_alias() {
    let (status, _) = send(create_test_app(), Method::POST, "/rectangle", Some(json!({
        "x": 50.0, "y": 60.0, "width": 200.0, "height": 100.0
    }))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_caption() {
    let (status, _) = send(create_test_app(), Method::POST, "/caption", Some(json!({
        "text": "Hello", "x": 100.0, "y": 200.0, "accent": "purple"
    }))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_clear() {
    let (status, _) = send(create_test_app(), Method::POST, "/clear", None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_click() {
    let (status, _) = send(create_test_app(), Method::POST, "/click", Some(json!({
        "x": 500, "y": 300
    }))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_notify() {
    let (status, _) = send(create_test_app(), Method::POST, "/notify", Some(json!({
        "title": "Test", "body": "Hello world"
    }))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_mcp_tools() {
    let (status, body) = send(create_test_app(), Method::GET, "/mcp/tools", None).await;
    assert_eq!(status, StatusCode::OK);
    let tools = body["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"cursor"));
    assert!(names.contains(&"highlight"));
    assert!(names.contains(&"screenshot"));
    assert!(names.contains(&"speak"));
    assert!(names.contains(&"clear"));
}

#[tokio::test]
async fn test_mcp_call_cursor() {
    let (status, body) = send(create_test_app(), Method::POST, "/mcp/call", Some(json!({
        "tool": "cursor", "parameters": {"x": 100.0, "y": 200.0}
    }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn test_mcp_call_clear() {
    let (status, body) = send(create_test_app(), Method::POST, "/mcp/call", Some(json!({
        "tool": "clear"
    }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn test_mcp_call_unknown_tool() {
    let (status, _) = send(create_test_app(), Method::POST, "/mcp/call", Some(json!({
        "tool": "nonexistent"
    }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_mcp_batch() {
    let (status, body) = send(create_test_app(), Method::POST, "/mcp/calls", Some(json!({
        "calls": [
            {"tool": "cursor", "parameters": {"x": 10.0, "y": 20.0}},
            {"tool": "clear"}
        ],
        "delay_ms": 0
    }))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["results"].is_array());
}

#[tokio::test]
async fn test_mcp_jsonrpc_initialize() {
    let (status, body) = send(create_test_app(), Method::POST, "/mcp", Some(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize"
    }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert_eq!(body["result"]["serverInfo"]["name"], "notclicky-bridge");
}

#[tokio::test]
async fn test_mcp_jsonrpc_tools_list() {
    let (status, body) = send(create_test_app(), Method::POST, "/mcp", Some(json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list"
    }))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["result"]["tools"].is_array());
}

#[tokio::test]
async fn test_mcp_jsonrpc_tools_call() {
    let (status, body) = send(create_test_app(), Method::POST, "/mcp", Some(json!({
        "jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": {"name": "highlight", "arguments": {"x": 10.0, "y": 20.0, "width": 100.0, "height": 50.0}}
    }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
}

#[tokio::test]
async fn test_mcp_jsonrpc_unknown_method() {
    let (status, body) = send(create_test_app(), Method::POST, "/mcp", Some(json!({
        "jsonrpc": "2.0", "id": 4, "method": "nonexistent"
    }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["error"]["code"], -32601);
}

#[test]
fn bridge_port_is_32123() {
    assert_eq!(notclicky::bridge::server::PORT, 32123);
}
