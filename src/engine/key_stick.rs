use crate::engine::mouse_stick::{quantize_stick_axes, STICK_MAX_F64};

/// Configuration for duration-based analog ramping on keyboard-driven sticks (`WASD`).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KeyStickConfig {
    /// Time (ms) to sweep ~95% of the way to full deflection (`0.0` = instant digital snap).
    pub sweep_ms: f64,
    /// Time (ms) to decay ~95% back to center on release (`0.0` = instant stop).
    pub release_ms: f64,
    /// Inner deadzone floor jumped past immediately on initial press (`0.0..1.0`).
    pub anti_deadzone: f64,
}

impl Default for KeyStickConfig {
    fn default() -> Self {
        Self {
            sweep_ms: 70.0,
            release_ms: 30.0,
            anti_deadzone: 0.18,
        }
    }
}

/// Stateful 2D ease-out + arc-blended smoother for a keyboard-driven analog stick.
#[derive(Debug, Clone, Default)]
pub struct KeyStickSmoother {
    cur_x: f64,
    cur_y: f64,
}

impl KeyStickSmoother {
    /// Creates a centered smoother.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cur_x: 0.0,
            cur_y: 0.0,
        }
    }

    /// Resets the stick position to `(0.0, 0.0)`.
    pub fn reset(&mut self) {
        self.cur_x = 0.0;
        self.cur_y = 0.0;
    }

    /// Advances the stick toward normalized `(target_x, target_y)` over `dt_secs`
    /// and returns circularly clamped `i16` stick coordinates.
    pub fn update(
        &mut self,
        target_x: f64,
        target_y: f64,
        dt_secs: f64,
        cfg: &KeyStickConfig,
    ) -> (i16, i16) {
        let tgt_r = target_x.hypot(target_y);

        if tgt_r > 1e-6 {
            if cfg.sweep_ms <= 0.0 || dt_secs <= 0.0 {
                self.cur_x = target_x;
                self.cur_y = target_y;
            } else {
                let mut cur_r = self.cur_x.hypot(self.cur_y);
                let tgt_angle = target_y.atan2(target_x);
                let anti_dz = cfg.anti_deadzone.clamp(0.0, 0.95);

                // Jump past the game's inner deadzone on initial press to avoid dead-time lag.
                let mut cur_angle = if cur_r < 1e-6 {
                    if anti_dz > 0.0 {
                        cur_r = anti_dz.min(tgt_r);
                    }
                    tgt_angle
                } else {
                    if cur_r < anti_dz {
                        cur_r = anti_dz.min(tgt_r);
                    }
                    self.cur_y.atan2(self.cur_x)
                };

                let tau_up = (cfg.sweep_ms / 1000.0) / 3.0;
                let alpha = 1.0 - (-dt_secs / tau_up.max(1e-6)).exp();

                // Ease magnitude toward target radius.
                let new_r = cur_r + (tgt_r - cur_r) * alpha;

                // Arc-blend direction along the shortest angular path so turns don't dip to (0,0).
                let diff = (tgt_angle - cur_angle + std::f64::consts::PI)
                    .rem_euclid(std::f64::consts::TAU)
                    - std::f64::consts::PI;
                let angle_alpha = (alpha * 1.5).min(1.0);
                cur_angle += diff * angle_alpha;

                self.cur_x = new_r * cur_angle.cos();
                self.cur_y = new_r * cur_angle.sin();
            }
        } else if cfg.release_ms <= 0.0 || dt_secs <= 0.0 {
            self.cur_x = 0.0;
            self.cur_y = 0.0;
        } else {
            let tau_down = (cfg.release_ms / 1000.0) / 3.0;
            let decay = (-dt_secs / tau_down.max(1e-6)).exp();
            self.cur_x *= decay;
            self.cur_y *= decay;
            if self.cur_x.hypot(self.cur_y) < 0.02 {
                self.cur_x = 0.0;
                self.cur_y = 0.0;
            }
        }

        quantize_stick_axes(self.cur_x * STICK_MAX_F64, self.cur_y * STICK_MAX_F64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_sweep_snaps_instantly() {
        let mut s = KeyStickSmoother::new();
        let cfg = KeyStickConfig {
            sweep_ms: 0.0,
            release_ms: 0.0,
            anti_deadzone: 0.18,
        };
        let (x, y) = s.update(0.0, -1.0, 0.004, &cfg);
        assert_eq!((x, y), (0, -32767));
        let (rx, ry) = s.update(0.0, 0.0, 0.004, &cfg);
        assert_eq!((rx, ry), (0, 0));
    }

    #[test]
    fn initial_press_jumps_past_anti_deadzone_and_sweeps_smoothly() {
        let mut s = KeyStickSmoother::new();
        let cfg = KeyStickConfig {
            sweep_ms: 80.0,
            release_ms: 30.0,
            anti_deadzone: 0.18,
        };

        let (_, y1) = s.update(0.0, -1.0, 0.004, &cfg);
        // Should immediately exceed anti_deadzone (0.18 * 32767 ≈ 5898), but stay well below 32767.
        assert!(y1 < -5800, "y1={y1} should jump past anti_deadzone");
        assert!(y1 > -15000, "y1={y1} should sweep smoothly rather than snap");

        // After ~100ms (25 ticks of 4ms), should converge near -32767.
        let mut y_end = y1;
        for _ in 0..30 {
            let (_, y) = s.update(0.0, -1.0, 0.004, &cfg);
            y_end = y;
        }
        assert!(y_end < -31500, "y_end={y_end} should converge near -32767");
    }

    #[test]
    fn arc_blend_keeps_radius_high_when_turning_90_degrees() {
        let mut s = KeyStickSmoother::new();
        let cfg = KeyStickConfig::default();

        // Saturate upward (0, -1).
        for _ in 0..40 {
            s.update(0.0, -1.0, 0.004, &cfg);
        }

        // Turn 90 degrees right (1, 0) and verify radius stays near 1.0 instead of dipping through 0.
        let (x, y) = s.update(1.0, 0.0, 0.004, &cfg);
        let mag = (x as f64).hypot(y as f64) / STICK_MAX_F64;
        assert!(
            mag > 0.90,
            "arc turn should preserve radius near 1.0, got mag={mag}"
        );
    }

    #[test]
    fn release_decays_cleanly_to_zero() {
        let mut s = KeyStickSmoother::new();
        let cfg = KeyStickConfig {
            sweep_ms: 60.0,
            release_ms: 25.0,
            anti_deadzone: 0.18,
        };

        for _ in 0..40 {
            s.update(1.0, 0.0, 0.004, &cfg);
        }

        let mut last = (0, 0);
        for _ in 0..30 {
            last = s.update(0.0, 0.0, 0.004, &cfg);
        }
        assert_eq!(last, (0, 0));
    }
}
