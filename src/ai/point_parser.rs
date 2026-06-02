#[derive(Debug, Clone, PartialEq)]
pub enum PointType {
    Point,
    Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPoint {
    pub point_type: PointType,
    pub x: i32,
    pub y: i32,
    pub label: String,
}

pub fn parse_points(text: &str) -> Vec<ParsedPoint> {
    let mut results = Vec::new();
    let mut pos = 0;

    while pos < text.len() {
        if text[pos..].strip_prefix('[').is_some() {
            let after_bracket = &text[pos + 1..];
            if let Some(end) = after_bracket.find(']') {
                let content = &after_bracket[..end];
                if let Some(point) = parse_point_content(content) {
                    results.push(point);
                }
                pos += 1 + end + 1;
                continue;
            }
        }
        pos += 1;
    }

    results
}

fn parse_point_content(content: &str) -> Option<ParsedPoint> {
    let (type_str, rest) = content.split_once(':')?;
    let point_type = match type_str.to_uppercase().as_str() {
        "POINT" => PointType::Point,
        "TYPE" => PointType::Type,
        _ => return None,
    };

    let (coords, label) = rest.split_once(':')?;
    let (x_str, y_str) = coords.split_once(',')?;

    let x = x_str.trim().parse::<i32>().ok()?;
    let y = y_str.trim().parse::<i32>().ok()?;

    Some(ParsedPoint {
        point_type,
        x,
        y,
        label: label.trim().to_string(),
    })
}
