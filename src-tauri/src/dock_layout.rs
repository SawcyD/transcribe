//! Where the floating dock sits, shared by the recording dock, the answer
//! bubble and the voice-agent dock so they line up. Sizes are in *logical*
//! pixels and converted with the monitor's scale factor: the old code mixed
//! physical positions with logical window sizes, so on a 125–150% display
//! the docks drifted right and off-centre.

/// The recording dock, in logical pixels (matches tauri.conf.json).
pub const DOCK_WIDTH: f64 = 360.0;
pub const DOCK_HEIGHT: f64 = 48.0;
/// Gap between the dock and the screen edge (clears the taskbar).
pub const DOCK_MARGIN: f64 = 56.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

impl MonitorArea {
    pub fn of(monitor: &tauri::Monitor) -> Self {
        Self {
            x: monitor.position().x,
            y: monitor.position().y,
            width: monitor.size().width,
            height: monitor.size().height,
            scale: monitor.scale_factor().max(0.5),
        }
    }
}

/// The dock's rectangle (physical px): x, y, width, height.
pub fn dock_rect(area: MonitorArea, position: &str) -> (i32, i32, i32, i32) {
    let width = (DOCK_WIDTH * area.scale).round() as i32;
    let height = (DOCK_HEIGHT * area.scale).round() as i32;
    let margin = (DOCK_MARGIN * area.scale).round() as i32;
    let x = if position.ends_with("right") {
        area.x + area.width as i32 - width - margin / 2
    } else {
        area.x + (area.width as i32 - width) / 2
    };
    let y = if position.starts_with("top") {
        area.y + margin
    } else {
        area.y + area.height as i32 - margin - height
    };
    (x, y, width, height)
}

/// Top-left for a window of `size` (physical) centred on the dock's column,
/// either sitting directly on the dock's spot (`over = true`, e.g. the
/// voice-agent dock whose bottom pill replaces it) or just beyond it on the
/// screen-centre side (the answer bubble). Kept on-screen.
pub fn beside_dock(area: MonitorArea, position: &str, size: (u32, u32), over: bool) -> (i32, i32, bool) {
    let (dock_x, dock_y, dock_w, dock_h) = dock_rect(area, position);
    let (w, h) = (size.0 as i32, size.1 as i32);
    let gap = (10.0 * area.scale).round() as i32;
    let x = (dock_x + dock_w / 2 - w / 2).clamp(area.x, area.x + area.width as i32 - w);
    let top = position.starts_with("top");
    let y = match (top, over) {
        (false, true) => dock_y + dock_h - h,
        (true, true) => dock_y,
        (false, false) => dock_y - gap - h,
        (true, false) => dock_y + dock_h + gap,
    };
    (x, y.clamp(area.y, area.y + area.height as i32 - h), top)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD_150: MonitorArea = MonitorArea { x: 0, y: 0, width: 1920, height: 1080, scale: 1.5 };

    #[test]
    fn dock_is_centred_at_any_scale() {
        let (x, _, w, h) = dock_rect(FHD_150, "bottom_center");
        assert_eq!((w, h), (540, 72));
        assert_eq!(x + w / 2, 960);
        let (x, _, w, _) = dock_rect(MonitorArea { scale: 1.0, ..FHD_150 }, "bottom_center");
        assert_eq!(x + w / 2, 960);
    }

    #[test]
    fn dock_sits_above_the_bottom_margin() {
        let (_, y, _, h) = dock_rect(FHD_150, "bottom_center");
        assert_eq!(y + h, 1080 - 84);
    }

    #[test]
    fn bubble_goes_above_a_bottom_dock_and_below_a_top_one() {
        let (_, dock_y, _, dock_h) = dock_rect(FHD_150, "bottom_center");
        let (_, y, top) = beside_dock(FHD_150, "bottom_center", (645, 630), false);
        assert!(!top);
        assert_eq!(y + 630 + 15, dock_y);
        let (_, dock_y, _, _) = dock_rect(FHD_150, "top_center");
        let (_, y, top) = beside_dock(FHD_150, "top_center", (645, 630), false);
        assert!(top);
        assert_eq!(y, dock_y + dock_h + 15);
    }

    #[test]
    fn agent_dock_bottom_lines_up_with_the_dock() {
        let (_, dock_y, _, dock_h) = dock_rect(FHD_150, "bottom_center");
        let (x, y, _) = beside_dock(FHD_150, "bottom_center", (570, 198), true);
        assert_eq!(y + 198, dock_y + dock_h);
        assert_eq!(x + 285, 960);
    }

    #[test]
    fn stays_on_screen_on_a_secondary_monitor() {
        let left = MonitorArea { x: -1280, y: 0, width: 1280, height: 720, scale: 1.0 };
        let (x, y, _) = beside_dock(left, "bottom_right", (430, 420), false);
        assert!(x >= -1280 && x + 430 <= 0);
        assert!(y >= 0);
    }
}
