#[test]
fn parse_single_point() {
    let points = notclicky::ai::point_parser::parse_points("Click [POINT:100,200:Settings menu]");
    assert_eq!(points.len(), 1);
    let p = &points[0];
    assert_eq!(p.point_type, notclicky::ai::point_parser::PointType::Point);
    assert_eq!(p.x, 100);
    assert_eq!(p.y, 200);
    assert_eq!(p.label, "Settings menu");
}

#[test]
fn parse_type_point() {
    let points = notclicky::ai::point_parser::parse_points("[TYPE:50,75:Submit button]");
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].point_type, notclicky::ai::point_parser::PointType::Type);
    assert_eq!(points[0].label, "Submit button");
}

#[test]
fn parse_multiple_points() {
    let text = "First [POINT:10,20:A] then [POINT:30,40:B]";
    let points = notclicky::ai::point_parser::parse_points(text);
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].label, "A");
    assert_eq!(points[1].label, "B");
}

#[test]
fn parse_no_points() {
    let points = notclicky::ai::point_parser::parse_points("No points here");
    assert!(points.is_empty());
}

#[test]
fn parse_mixed_text_and_points() {
    let text = "Click on [POINT:500,300:File menu] then go to [TYPE:100,200:OK button]";
    let points = notclicky::ai::point_parser::parse_points(text);
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].point_type, notclicky::ai::point_parser::PointType::Point);
    assert_eq!(points[1].point_type, notclicky::ai::point_parser::PointType::Type);
}

#[test]
fn parse_invalid_brackets_ignored() {
    let points = notclicky::ai::point_parser::parse_points("[INVALID:data] and [POINT:1,2:test]");
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].label, "test");
}

#[test]
fn parse_whitespace_in_coords() {
    let points = notclicky::ai::point_parser::parse_points("[POINT: 100 , 200 : test]");
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].x, 100);
    assert_eq!(points[0].y, 200);
}

#[test]
fn sentence_splitter_single_sentence() {
    let mut splitter = notclicky::ai::sentence_splitter::SentenceSplitter::new();
    let sentences = splitter.push("Hello world. ");
    assert_eq!(sentences, vec!["Hello world."]);
}

#[test]
fn sentence_splitter_multiple_sentences() {
    let mut splitter = notclicky::ai::sentence_splitter::SentenceSplitter::new();
    let mut all = Vec::new();
    all.extend(splitter.push("Hello. "));
    all.extend(splitter.push("World! "));
    all.extend(splitter.push("How are you? "));
    assert!(all.len() >= 3);
}

#[test]
fn sentence_splitter_flush_remaining() {
    let mut splitter = notclicky::ai::sentence_splitter::SentenceSplitter::new();
    splitter.push("Hello world");
    let remaining = splitter.flush();
    assert_eq!(remaining, Some("Hello world".to_string()));
}

#[test]
fn sentence_splitter_flush_empty() {
    let mut splitter = notclicky::ai::sentence_splitter::SentenceSplitter::new();
    assert!(splitter.flush().is_none());
}

#[test]
fn prefire_divergence_identical() {
    let div = notclicky::ai::prefire::compute_divergence("hello world", "hello world");
    assert!(div < 0.01);
}

#[test]
fn prefire_divergence_completely_different() {
    let div = notclicky::ai::prefire::compute_divergence("hello world", "foo bar baz");
    assert!(div > 0.5);
}

#[test]
fn prefire_divergence_partial_match() {
    let div = notclicky::ai::prefire::compute_divergence("what time is it", "what time is it now");
    assert!(div < 0.15);
}

#[test]
fn prefire_divergence_empty_interim() {
    let div = notclicky::ai::prefire::compute_divergence("", "hello");
    assert_eq!(div, 1.0);
}
