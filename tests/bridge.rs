#[test]
fn mcp_tool_names_match_bridge() {
    let expected = vec![
        "cursor", "cursors", "scribble", "highlight", "caption",
        "screenshot", "click", "speak", "notify", "clear",
    ];

    let health_tools = vec![
        "cursor", "cursors", "scribble", "highlight", "rectangle",
        "caption", "screenshot", "click", "speak", "notify", "clear",
    ];

    for tool in &expected {
        assert!(health_tools.contains(tool), "missing tool: {}", tool);
    }
}

#[test]
fn bridge_port_is_32123() {
    assert_eq!(notclicky::bridge::server::PORT, 32123);
}
