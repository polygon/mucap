pub struct ZoomOptions {
    pub snap_range: f32,
    pub speed: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

#[derive(PartialEq)]
pub enum LastOperation {
    Pan,
    Zoom,
}

pub struct ZoomControl {
    available_time: (f32, f32),
    cur_range: (f32, f32),
    target_range: (f32, f32),
    options: ZoomOptions,
    last_operation: LastOperation,
}

impl Default for ZoomOptions {
    fn default() -> Self {
        ZoomOptions {
            snap_range: 0.995,
            speed: 10.0,
            min_zoom: 10.,
            max_zoom: 600.,
        }
    }
}

impl Default for ZoomControl {
    fn default() -> Self {
        ZoomControl {
            available_time: (0.0, 30.0),
            cur_range: (0.0, 30.0),
            target_range: (0.0, 30.0),
            options: ZoomOptions::default(),
            last_operation: LastOperation::Pan,
        }
    }
}

impl ZoomControl {
    pub fn current_range(&self) -> (f32, f32) {
        self.cur_range
    }

    pub fn update_time(&mut self, available_time: (f32, f32)) {
        self.available_time = available_time;

        // Clamp target_range to stay within available_time bounds
        if self.target_range.0 < self.available_time.0 {
            let offset = self.available_time.0 - self.target_range.0;
            self.target_range.0 = self.available_time.0;
            self.target_range.1 += offset;
        }

        if self.target_range.1 > self.available_time.1 {
            let offset = self.target_range.1 - self.available_time.1;
            self.target_range.1 = self.available_time.1;
            self.target_range.0 -= offset;
        }

        // Ensure target_range doesn't exceed available bounds
        self.target_range.0 = self.target_range.0.max(self.available_time.0);
        self.target_range.1 = self.target_range.1.min(self.available_time.1);
    }

    pub fn update(&mut self, dt: f32) {
        let current_range = self.cur_range.1 - self.cur_range.0;
        let position_offset = (self.cur_range.0 - self.target_range.0).abs()
            + (self.cur_range.1 - self.target_range.1).abs();

        let snap_threshold = current_range * (1.0 - self.options.snap_range);
        let epsilon = 1e-5; // Account for floating point precision

        // Snap if we are almost at the target
        if position_offset <= snap_threshold + epsilon {
            self.cur_range = self.target_range;
            return;
        }

        // Interpolate towards target based on speed
        let alpha = (self.options.speed * dt).min(1.0);
        self.cur_range.0 = self.cur_range.0 + (self.target_range.0 - self.cur_range.0) * alpha;
        self.cur_range.1 = self.cur_range.1 + (self.target_range.1 - self.cur_range.1) * alpha;
    }

    pub fn zoom(&mut self, factor: f32, center: f32) {
        // If a pan was in progress, abandon it
        if self.last_operation == LastOperation::Pan {
            self.target_range = self.cur_range;
        }

        // Convert center from current_range to target_range coordinates
        let cur_range_size = self.cur_range.1 - self.cur_range.0;
        let center_in_target = self.target_range.0
            + (center - self.cur_range.0) * (self.target_range.1 - self.target_range.0)
                / cur_range_size;

        // Calculate new range size
        let current_size = self.target_range.1 - self.target_range.0;
        let mut new_size = current_size * factor;

        // Coerce new_size to visibility constraints
        new_size = new_size.clamp(self.options.min_zoom, self.options.max_zoom);

        // Calculate new bounds maintaining center position
        let size_diff = new_size - current_size;
        let new_start = self.target_range.0
            - size_diff * (center_in_target - self.target_range.0) / current_size;
        let new_end = new_start + new_size;

        // Check available time bounds and clamp if necessary
        let clamped_start = new_start.max(self.available_time.0);
        let clamped_end = new_end.min(self.available_time.1);

        // Apply the zoomed range
        self.target_range = (clamped_start, clamped_end);
        self.last_operation = LastOperation::Zoom;
    }

    pub fn pan(&mut self, amount: f32) {
        let range_size = self.target_range.1 - self.target_range.0;
        let pan_distance = amount * range_size;

        let mut new_start = self.target_range.0 + pan_distance;
        let mut new_end = self.target_range.1 + pan_distance;

        // Clamp while maintaining range size by adjusting the opposite bound
        if new_start < self.available_time.0 {
            let offset = self.available_time.0 - new_start;
            new_start = self.available_time.0;
            new_end += offset;
        }

        if new_end > self.available_time.1 {
            let offset = new_end - self.available_time.1;
            new_end = self.available_time.1;
            new_start -= offset;
        }

        // Final clamp to ensure we stay within bounds
        new_start = new_start.max(self.available_time.0);
        new_end = new_end.min(self.available_time.1);

        self.target_range = (new_start, new_end);
        self.last_operation = LastOperation::Pan;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // update_time tests
    #[test]
    fn test_update_time_no_change() {
        let mut zc = ZoomControl::default();
        zc.update_time((0.0, 30.0));
        assert_eq!(zc.target_range, (0.0, 30.0));
    }

    #[test]
    fn test_update_time_shrink_from_end() {
        let mut zc = ZoomControl::default();
        zc.target_range = (0.0, 30.0);
        zc.update_time((0.0, 20.0));
        assert_eq!(zc.target_range.1, 20.0);
        assert_eq!(zc.target_range.0, 0.0);
    }

    #[test]
    fn test_update_time_shrink_from_start() {
        let mut zc = ZoomControl::default();
        zc.target_range = (0.0, 30.0);
        zc.update_time((10.0, 40.0));
        assert_eq!(zc.target_range.0, 10.0);
        assert_eq!(zc.target_range.1 - zc.target_range.0, 30.0); // maintains range size
    }

    #[test]
    fn test_update_time_both_sides_constrain() {
        let mut zc = ZoomControl::default();
        zc.target_range = (0.0, 30.0);
        zc.update_time((5.0, 25.0));
        assert!(zc.target_range.0 >= 5.0);
        assert!(zc.target_range.1 <= 25.0);
    }

    #[test]
    fn test_update_time_range_larger_than_available() {
        let mut zc = ZoomControl::default();
        zc.target_range = (10.0, 50.0); // 40 unit range
        zc.update_time((0.0, 20.0)); // only 20 units available
        assert!(zc.target_range.1 - zc.target_range.0 <= 20.0);
        assert!(zc.target_range.0 >= 0.0);
        assert!(zc.target_range.1 <= 20.0);
    }

    // update/interpolation tests
    #[test]
    fn test_update_zero_delta_time() {
        let mut zc = ZoomControl::default();
        // Snap when position offset is small enough
        // With snap_range=0.995, position_offset must be <= size * (1-0.995) = size * 0.005
        zc.target_range = (100.0, 200.0); // 100 unit range
        zc.cur_range = (100.0, 200.0); // same range, zero position offset
        zc.update(0.0);
        // With dt=0 and zero offset, snap occurs immediately
        assert_eq!(zc.cur_range, zc.target_range);
    }

    #[test]
    fn test_update_snap_when_over_snap_range() {
        let mut zc = ZoomControl::default();
        zc.options.snap_range = 0.98;
        // Small zoom (1% range change) with no position drift should snap
        zc.cur_range = (0.0, 100.0); // 100 unit range
        zc.target_range = (0.0, 99.0); // 99 unit range (1% change)
        zc.update(0.016); // any positive dt
        assert_eq!(zc.cur_range, zc.target_range); // snaps immediately
    }

    #[test]
    fn test_update_interpolate_below_snap_range() {
        let mut zc = ZoomControl::default();
        zc.options.speed = 1.0;
        zc.cur_range = (0.0, 30.0);
        zc.target_range = (5.0, 25.0); // 20 unit range, cur is 30, ratio 0.667 < 0.98, so interpolates
        let orig_cur_range = zc.cur_range;
        zc.update(0.05); // small dt to not reach target
        // Should interpolate, not snap
        assert!(zc.cur_range != orig_cur_range);
        // May reach target with high enough alpha, so just verify it changed
    }

    #[test]
    fn test_update_clamped_alpha() {
        let mut zc = ZoomControl::default();
        zc.options.speed = 100.0; // very high speed
        zc.cur_range = (0.0, 30.0);
        zc.target_range = (10.0, 20.0);
        zc.update(0.1); // alpha would be 10.0, but clamped to 1.0
        assert_eq!(zc.cur_range, zc.target_range); // should reach target in one step
    }

    // zoom tests
    #[test]
    fn test_zoom_in() {
        let mut zc = ZoomControl::default();
        zc.target_range = (0.0, 30.0);
        zc.cur_range = (0.0, 30.0);
        zc.zoom(0.5, 15.0); // factor 0.5 makes new_size = 15.0, clamped to min_zoom 30.0
        let new_size = zc.target_range.1 - zc.target_range.0;
        // With default min_zoom=30, we can't zoom smaller than 30
        assert!(new_size >= zc.options.min_zoom);
    }

    #[test]
    fn test_zoom_out() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 1000.0);
        zc.target_range = (100.0, 200.0);
        zc.cur_range = (100.0, 200.0);
        let initial_size = zc.target_range.1 - zc.target_range.0;
        zc.zoom(2.0, 150.0); // zoom out by 2x
        let new_size = zc.target_range.1 - zc.target_range.0;
        assert!(new_size > initial_size);
    }

    #[test]
    fn test_zoom_respects_min_zoom() {
        let mut zc = ZoomControl::default();
        zc.options.min_zoom = 30.0;
        zc.target_range = (0.0, 30.0);
        zc.zoom(0.1, 15.0); // try to zoom way in
        let size = zc.target_range.1 - zc.target_range.0;
        assert!(size >= zc.options.min_zoom);
    }

    #[test]
    fn test_zoom_respects_max_zoom() {
        let mut zc = ZoomControl::default();
        zc.options.max_zoom = 600.0;
        zc.target_range = (0.0, 30.0);
        zc.zoom(100.0, 15.0); // try to zoom way out
        let size = zc.target_range.1 - zc.target_range.0;
        assert!(size <= zc.options.max_zoom);
    }

    #[test]
    fn test_zoom_maintains_center() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 1000.0);
        zc.target_range = (100.0, 200.0);
        zc.cur_range = (100.0, 200.0);
        let center = 150.0;
        zc.zoom(0.5, center); // zoom in, shouldn't be clamped
        let new_center = (zc.target_range.0 + zc.target_range.1) / 2.0;
        assert!((new_center - center).abs() < 0.1);
    }

    #[test]
    fn test_zoom_abandons_pan_animation() {
        // Test that zooming while a pan animation is active abandons the pan
        let mut zc = ZoomControl::default();
        zc.options.snap_range = 2.0; // disable snap (always false: size > 2.0 * size)
        zc.options.speed = 1.0; // normal speed
        zc.available_time = (0.0, 1000.0);
        zc.target_range = (100.0, 200.0);
        zc.cur_range = (100.0, 200.0);

        // Pan operation
        zc.pan(0.2); // move to (120.0, 220.0)
        assert!(matches!(zc.last_operation, LastOperation::Pan));

        let target_after_pan = zc.target_range;

        // Simulate animation with very low speed to keep it incomplete
        zc.options.speed = 0.001; // now make it very slow
        zc.update(1.0); // update with 1 second, but with slow speed, alpha = 0.001
        let cur_range_after_update = zc.cur_range;
        assert_ne!(zc.cur_range, zc.target_range); // animation still in progress

        // Now zoom while pan animation is in flight
        zc.zoom(0.5, 150.0);

        // Pan animation should be abandoned: cur_range snapped to current state at zoom time
        assert_eq!(zc.cur_range, cur_range_after_update);
        assert!(matches!(zc.last_operation, LastOperation::Zoom));
    }

    #[test]
    fn test_zoom_maintains_center_from_equal_state() {
        // Test zoom when cur_range == target_range, then zoom again during animation
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 1000.0);

        // Start: both ranges equal
        zc.cur_range = (0.0, 100.0);
        zc.target_range = (0.0, 100.0);

        let center1 = 50.0;
        zc.zoom(0.5, center1); // zoom to 50 units

        // Verify center maintained after first zoom
        let new_center1 = (zc.target_range.0 + zc.target_range.1) / 2.0;
        assert!((new_center1 - center1).abs() < 0.1);

        // Now zoom again while cur_range still lags (hasn't been updated yet)
        // This simulates user zooming before animation completes
        let center2 = new_center1; // zoom around new center
        zc.zoom(0.5, center2); // zoom another 50%

        // Verify center is still maintained
        let new_center2 = (zc.target_range.0 + zc.target_range.1) / 2.0;
        assert!(
            (new_center2 - center2).abs() < 0.1,
            "center drifted on second zoom: expected {}, got {}",
            center2,
            new_center2
        );
    }

    #[test]
    fn test_zoom_respects_available_bounds() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 1000.0);
        zc.target_range = (100.0, 900.0);
        zc.cur_range = (100.0, 900.0);
        zc.zoom(0.5, 500.0); // zoom in to ~400 units
        assert!(zc.target_range.0 >= zc.available_time.0);
        assert!(zc.target_range.1 <= zc.available_time.1);
    }

    #[test]
    fn test_zoom_at_boundary() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 100.0);
        zc.target_range = (0.0, 100.0);
        zc.cur_range = (0.0, 100.0);
        zc.zoom(0.5, 0.0); // zoom at left boundary
        assert!(zc.target_range.0 >= 0.0);
        assert!(zc.target_range.1 <= 100.0);
    }

    #[test]
    fn test_zoom_clamped_if_new_size_exceeds_available() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 10.0);
        zc.target_range = (2.0, 8.0);
        zc.cur_range = (2.0, 8.0);
        // Zoom out by 2x: new_size would be 12, but clamped to bounds [0, 10]
        zc.zoom(2.0, 5.0);
        // Range should be clamped to available bounds
        assert!(zc.target_range.0 >= zc.available_time.0);
        assert!(zc.target_range.1 <= zc.available_time.1);
    }

    // pan tests
    #[test]
    fn test_pan_right() {
        let mut zc = ZoomControl::default();
        zc.target_range = (0.0, 10.0);
        zc.pan(0.5); // pan 50% of range size
        assert!(zc.target_range.0 > 0.0);
        assert!(zc.target_range.1 > 10.0);
        assert_eq!(zc.target_range.1 - zc.target_range.0, 10.0); // size preserved
    }

    #[test]
    fn test_pan_left() {
        let mut zc = ZoomControl::default();
        zc.target_range = (10.0, 20.0);
        zc.pan(-0.5); // pan left by 50% of range size
        assert!(zc.target_range.0 < 10.0);
        assert!(zc.target_range.1 < 20.0);
        assert_eq!(zc.target_range.1 - zc.target_range.0, 10.0); // size preserved
    }

    #[test]
    fn test_pan_zero() {
        let mut zc = ZoomControl::default();
        zc.target_range = (5.0, 15.0);
        let original = zc.target_range;
        zc.pan(0.0);
        assert_eq!(zc.target_range, original);
    }

    #[test]
    fn test_pan_respects_left_boundary() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 100.0);
        zc.target_range = (5.0, 15.0);
        zc.pan(-1.0); // try to pan far left
        assert!(zc.target_range.0 >= zc.available_time.0);
        assert_eq!(zc.target_range.1 - zc.target_range.0, 10.0); // size preserved
    }

    #[test]
    fn test_pan_respects_right_boundary() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 100.0);
        zc.target_range = (85.0, 95.0);
        zc.pan(1.0); // try to pan far right
        assert!(zc.target_range.1 <= zc.available_time.1);
        assert_eq!(zc.target_range.1 - zc.target_range.0, 10.0); // size preserved
    }

    #[test]
    fn test_pan_at_left_boundary() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 30.0);
        zc.target_range = (0.0, 10.0);
        let original = zc.target_range;
        zc.pan(-1.0);
        assert_eq!(zc.target_range, original); // can't pan further left
    }

    #[test]
    fn test_pan_at_right_boundary() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 30.0);
        zc.target_range = (20.0, 30.0);
        let original = zc.target_range;
        zc.pan(1.0);
        assert_eq!(zc.target_range, original); // can't pan further right
    }

    #[test]
    fn test_pan_maintains_range_size() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 100.0);
        zc.target_range = (20.0, 50.0);
        let original_size = zc.target_range.1 - zc.target_range.0;
        zc.pan(0.3);
        assert_eq!(zc.target_range.1 - zc.target_range.0, original_size);
    }

    // edge cases
    #[test]
    fn test_zoom_with_zero_center_position() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 1000.0);
        zc.target_range = (0.0, 500.0);
        zc.cur_range = (0.0, 500.0);
        zc.zoom(0.5, 0.0); // zoom at left boundary
        assert!(zc.target_range.0 >= zc.available_time.0);
        assert!(zc.target_range.1 <= zc.available_time.1);
    }

    #[test]
    fn test_zoom_with_max_center_position() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 1000.0);
        zc.target_range = (500.0, 1000.0);
        zc.cur_range = (500.0, 1000.0);
        zc.zoom(0.5, 1000.0); // zoom at right boundary
        assert!(zc.target_range.0 >= zc.available_time.0);
        assert!(zc.target_range.1 <= zc.available_time.1);
    }

    #[test]
    fn test_large_zoom_steps() {
        let mut zc = ZoomControl::default();
        zc.options.min_zoom = 5.0; // allow smaller zoom
        zc.target_range = (0.0, 30.0);
        zc.cur_range = (0.0, 30.0);
        // Multiple zoom operations
        zc.zoom(0.5, 15.0); // 30 * 0.5 = 15
        let size_after_first = zc.target_range.1 - zc.target_range.0;
        zc.zoom(0.5, zc.target_range.0 + size_after_first / 2.0); // zoom again
        let size_after_second = zc.target_range.1 - zc.target_range.0;
        assert!(size_after_second < size_after_first);
    }

    #[test]
    fn test_zoom_pan_combination() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 1000.0);
        zc.target_range = (200.0, 800.0);
        zc.cur_range = (200.0, 800.0);

        zc.zoom(0.5, 500.0); // zoom to ~300 units
        let range_size = zc.target_range.1 - zc.target_range.0;

        zc.pan(0.2);
        assert_eq!(zc.target_range.1 - zc.target_range.0, range_size);
        assert!(zc.target_range.0 >= zc.available_time.0);
        assert!(zc.target_range.1 <= zc.available_time.1);
    }

    #[test]
    fn test_very_small_range() {
        let mut zc = ZoomControl::default();
        zc.options.min_zoom = 0.1;
        zc.target_range = (0.0, 0.1);
        zc.cur_range = (0.0, 0.1);
        zc.zoom(0.5, 0.05);
        let size = zc.target_range.1 - zc.target_range.0;
        assert!(size >= 0.1);
    }

    #[test]
    fn test_very_large_range() {
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 10000.0);
        zc.target_range = (0.0, 10000.0);
        zc.cur_range = (0.0, 10000.0);
        zc.zoom(0.5, 5000.0);
        assert!(zc.target_range.1 - zc.target_range.0 > 0.0);
    }

    #[test]
    fn test_pan_does_not_snap_immediately() {
        // Test that panning animates smoothly and doesn't snap immediately
        let mut zc = ZoomControl::default();
        zc.available_time = (0.0, 1000.0);
        zc.options.speed = 1.0;
        zc.cur_range = (100.0, 200.0); // 100 unit range
        zc.target_range = (100.0, 200.0);

        // Pan to the right
        zc.pan(0.5); // pan by 50% of range (50 units) -> (150.0, 250.0)
        let target_after_pan = zc.target_range;
        assert_ne!(zc.cur_range, target_after_pan, "Pan updated target_range");

        // Update with very small dt to start interpolation
        zc.update(0.01);

        // cur_range should have moved towards target but NOT snapped immediately
        assert_ne!(
            zc.cur_range, target_after_pan,
            "Pan should animate smoothly, not snap immediately"
        );

        // cur_range should have moved in the right direction
        assert!(
            zc.cur_range.0 > 100.0,
            "cur_range.0 should move towards target"
        );
        assert!(
            zc.cur_range.1 > 200.0,
            "cur_range.1 should move towards target"
        );
    }
}
