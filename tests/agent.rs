use notclicky::agent::session::{AgentSession, AgentStatus};

#[test]
fn session_new_has_starting_status() {
    let session = AgentSession::new("test prompt".to_string(), "/tmp".to_string(), None);
    assert_eq!(session.status, AgentStatus::Starting);
    assert!(session.id.starts_with("agent-"));
    assert_eq!(session.prompt, "test prompt");
    assert_eq!(session.working_dir, "/tmp");
    assert!(session.model.is_none());
    assert!(session.output.is_empty());
}

#[test]
fn session_with_model() {
    let session = AgentSession::new("hello".to_string(), "/home".to_string(), Some("gpt-4".to_string()));
    assert_eq!(session.model.as_deref(), Some("gpt-4"));
}

#[test]
fn session_id_is_unique() {
    let a = AgentSession::new("a".to_string(), "/tmp".to_string(), None);
    let b = AgentSession::new("b".to_string(), "/tmp".to_string(), None);
    assert_ne!(a.id, b.id);
}

#[test]
fn session_status_serde_roundtrip() {
    let statuses = vec![AgentStatus::Starting, AgentStatus::Running, AgentStatus::Done, AgentStatus::Failed];
    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let parsed: AgentStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, parsed);
    }
}

#[test]
fn session_serde_roundtrip() {
    let session = AgentSession::new("test".to_string(), "/tmp".to_string(), Some("claude-3".to_string()));
    let json = serde_json::to_string(&session).unwrap();
    let parsed: AgentSession = serde_json::from_str(&json).unwrap();
    assert_eq!(session.id, parsed.id);
    assert_eq!(session.prompt, parsed.prompt);
    assert_eq!(session.working_dir, parsed.working_dir);
    assert_eq!(session.model, parsed.model);
}

#[test]
fn session_output_accumulates() {
    let mut session = AgentSession::new("test".to_string(), "/tmp".to_string(), None);
    session.output.push_str("hello ");
    session.output.push_str("world");
    assert_eq!(session.output, "hello world");
}

#[test]
fn agent_routing_detects_agent_keyword() {
    assert!(notclicky::voice::assistant::is_agent_request("agent build a webpage"));
    assert!(notclicky::voice::assistant::is_agent_request("Agent do something"));
    assert!(notclicky::voice::assistant::is_agent_request("clicky agent fix the bug"));
    assert!(!notclicky::voice::assistant::is_agent_request("what is the weather"));
    assert!(!notclicky::voice::assistant::is_agent_request("tell me a joke"));
}

#[test]
fn agent_routing_strips_keyword() {
    assert_eq!(notclicky::voice::assistant::strip_agent_keyword("agent build a webpage"), "build a webpage");
    assert_eq!(notclicky::voice::assistant::strip_agent_keyword("clicky agent fix the bug"), "fix the bug");
    assert_eq!(notclicky::voice::assistant::strip_agent_keyword("hello world"), "hello world");
}
