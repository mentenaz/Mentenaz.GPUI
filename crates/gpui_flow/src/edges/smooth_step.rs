use crate::types::HandlePosition;

/// Result of smooth-step path computation.
pub struct SmoothStepPath {
    /// Sequence of points forming the path (including source and target).
    pub points: Vec<(f32, f32)>,
    pub label_x: f32,
    pub label_y: f32,
}

/// Compute a smooth-step (orthogonal) edge path between two handle positions.
///
/// Ported from xyflow's `getSmoothStepPath`.
pub fn get_smooth_step_path(
    source_x: f32,
    source_y: f32,
    source_position: HandlePosition,
    target_x: f32,
    target_y: f32,
    target_position: HandlePosition,
    offset: f32,
) -> SmoothStepPath {
    // Offset points away from the handles
    let (sx, sy) = offset_point(source_x, source_y, source_position, offset);
    let (tx, ty) = offset_point(target_x, target_y, target_position, offset);

    let mut points = vec![(source_x, source_y), (sx, sy)];

    // Determine the routing based on handle directions
    let source_horizontal = matches!(
        source_position,
        HandlePosition::Left | HandlePosition::Right
    );
    let target_horizontal = matches!(
        target_position,
        HandlePosition::Left | HandlePosition::Right
    );

    if source_horizontal && target_horizontal {
        // Both horizontal: route via mid-x
        let mid_x = (sx + tx) / 2.0;
        points.push((mid_x, sy));
        points.push((mid_x, ty));
    } else if !source_horizontal && !target_horizontal {
        // Both vertical: route via mid-y
        let mid_y = (sy + ty) / 2.0;
        points.push((sx, mid_y));
        points.push((tx, mid_y));
    } else if source_horizontal {
        // Source horizontal, target vertical: L-shape
        points.push((tx, sy));
    } else {
        // Source vertical, target horizontal: L-shape
        points.push((sx, ty));
    }

    points.push((tx, ty));
    points.push((target_x, target_y));

    let label_x = (source_x + target_x) / 2.0;
    let label_y = (source_y + target_y) / 2.0;

    SmoothStepPath {
        points,
        label_x,
        label_y,
    }
}

/// Offset a point away from a handle in the handle's direction.
fn offset_point(x: f32, y: f32, position: HandlePosition, offset: f32) -> (f32, f32) {
    match position {
        HandlePosition::Left => (x - offset, y),
        HandlePosition::Right => (x + offset, y),
        HandlePosition::Top => (x, y - offset),
        HandlePosition::Bottom => (x, y + offset),
    }
}
