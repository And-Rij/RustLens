//! Physical-pixel crop validation and result-window placement.

/// Coordinates are physical pixels relative to the captured monitor.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn validate(self, frame_width: u32, frame_height: u32) -> Result<Self, String> {
        if self.width < 2 || self.height < 2 {
            return Err("Виділи більший фрагмент екрана.".into());
        }
        let right = self.x.checked_add(self.width);
        let bottom = self.y.checked_add(self.height);
        if right.is_none_or(|v| v > frame_width) || bottom.is_none_or(|v| v > frame_height) {
            return Err("Виділення виходить за межі захопленого кадру.".into());
        }
        Ok(self)
    }
}

/// Prefer the right of the crop, fall back to the left, then clamp to the monitor.
/// Monitor origins can be negative in a multi-monitor desktop layout.
pub fn popup_position(
    origin: (i32, i32),
    frame: (u32, u32),
    rect: Rect,
    popup: (u32, u32),
) -> (i32, i32) {
    let (ox, oy) = (i64::from(origin.0), i64::from(origin.1));
    let max_x = ox + i64::from(frame.0.saturating_sub(popup.0));
    let max_y = oy + i64::from(frame.1.saturating_sub(popup.1));
    let right = ox + i64::from(rect.x) + i64::from(rect.width) + 12;
    let x = if right <= max_x {
        right
    } else {
        ox + i64::from(rect.x) - i64::from(popup.0) - 12
    };
    (
        x.clamp(ox, max_x) as i32,
        (oy + i64::from(rect.y)).clamp(oy, max_y) as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_can_touch_frame_edges() {
        assert!(
            Rect {
                x: 90,
                y: 80,
                width: 10,
                height: 20
            }
            .validate(100, 100)
            .is_ok()
        );
    }

    #[test]
    fn rejects_out_of_bounds_and_integer_overflow() {
        assert!(
            Rect {
                x: 99,
                y: 0,
                width: 2,
                height: 10
            }
            .validate(100, 100)
            .is_err()
        );
        assert!(
            Rect {
                x: u32::MAX,
                y: 0,
                width: 10,
                height: 10
            }
            .validate(100, 100)
            .is_err()
        );
    }

    #[test]
    fn popup_stays_inside_monitor_with_negative_origin() {
        let rect = Rect {
            x: 1850,
            y: 1000,
            width: 70,
            height: 80,
        };
        assert_eq!(
            popup_position((-1920, 0), (1920, 1080), rect, (420, 340)),
            (-502, 740)
        );
    }

    #[test]
    fn oversized_popup_is_anchored_at_monitor_origin() {
        let rect = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        assert_eq!(popup_position((0, 0), (320, 240), rect, (420, 340)), (0, 0));
    }
}
