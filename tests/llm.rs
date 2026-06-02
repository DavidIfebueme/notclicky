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
