/// Result of straight path computation.
pub struct StraightPath {
    pub source: (f32, f32),
    pub target: (f32, f32),
    pub label_x: f32,
    pub label_y: f32,
}

/// Compute a straight edge path between two points.
pub fn get_straight_path(
    source_x: f32,
    source_y: f32,
    target_x: f32,
    target_y: f32,
) -> StraightPath {
    StraightPath {
        source: (source_x, source_y),
        target: (target_x, target_y),
        label_x: (source_x + target_x) / 2.0,
        label_y: (source_y + target_y) / 2.0,
    }
}
