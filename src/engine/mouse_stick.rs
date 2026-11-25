//! Deterministic, allocation-free Mouse-to-Right-Stick DSP engine.

pub(crate) const STICK_MAX_F64: f64 = 32767.0;
const STICK_MAX_I32: i32 = 32767;
const STICK_MAX_SQ_I64: i64 = (STICK_MAX_I32 as i64) * (STICK_MAX_I32 as i64);
const VELOCITY_EPSILON: f64 = 1e-6;

/// Quantizes floating-point stick coordinates in `[-32767.0, 32767.0]` to `i16`
/// while preserving the circular magnitude bound `x^2 + y^2 <= 32767^2`.
#[inline]
pub(crate) fn quantize_stick_axes(fx: f64, fy: f64) -> (i16, i16) {
    let mut ix = fx.round() as i32;
    let mut iy = fy.round() as i32;

    // Rounding at 45-degree saturation can exceed 32767^2 by 1 LSB; fall back to trunc.
    let sq = (ix as i64) * (ix as i64) + (iy as i64) * (iy as i64);
    if sq > STICK_MAX_SQ_I64 {
        ix = fx.trunc() as i32;
        iy = fy.trunc() as i32;
    }

    (
        ix.clamp(-STICK_MAX_I32, STICK_MAX_I32) as i16,
        iy.clamp(-STICK_MAX_I32, STICK_MAX_I32) as i16,
    )
}

/// Configuration parameters for [`MouseStickEngine`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseStickConfig {
    /// Target loop frequency in Hz (default: 250.0).
    pub hz: f64,
    /// Velocity scaling factor mapping counts/sec to normalized stick velocity `[0.0, 1.0]`.
    pub sensitivity: f64,
    /// Exponential moving average smoothing factor in `(0.0, 1.0]` applied on active ticks.
    pub ema_alpha: f64,
    /// Exponential decay half-life in milliseconds applied on idle ticks.
    pub decay_half_life_ms: f64,
    /// Radial anti-deadzone floor in `[0.0, 1.0)` (e.g., `0.18` for Zelda: BotW).
    pub anti_deadzone: f64,
    /// Power curve exponent applied to normalized radial velocity (e.g., `0.75`).
    pub power_gamma: f64,
    /// Whether to invert the Y-axis output.
    pub invert_y: bool,
}

impl Default for MouseStickConfig {
    fn default() -> Self {
        Self {
            hz: 250.0,
            sensitivity: 0.0005,
            ema_alpha: 0.4,
            decay_half_life_ms: 16.0,
            anti_deadzone: 0.18,
            power_gamma: 0.75,
            invert_y: false,
        }
    }
}

/// Deterministic, zero-allocation DSP engine converting raw mouse deltas into
/// circularly clamped, anti-deadzone-compensated 16-bit gamepad right-stick axes.
#[derive(Debug, Clone, Copy)]
pub struct MouseStickEngine {
    config: MouseStickConfig,
    accum_dx: f64,
    accum_dy: f64,
    vx: f64,
    vy: f64,
}

impl MouseStickEngine {
    /// Creates a new [`MouseStickEngine`] with the given configuration.
    #[must_use]
    pub const fn new(config: MouseStickConfig) -> Self {
        Self {
            config,
            accum_dx: 0.0,
            accum_dy: 0.0,
            vx: 0.0,
            vy: 0.0,
        }
    }

    /// Returns the engine's configuration.
    #[must_use]
    pub const fn config(&self) -> &MouseStickConfig {
        &self.config
    }

    /// Clears accumulated mouse deltas and internal velocity state.
    #[inline]
    pub fn reset(&mut self) {
        self.accum_dx = 0.0;
        self.accum_dy = 0.0;
        self.vx = 0.0;
        self.vy = 0.0;
    }

    /// Accumulates raw `REL_X` / `REL_Y` mouse deltas. Non-finite inputs are ignored.
    #[inline]
    pub fn feed_delta(&mut self, dx: f64, dy: f64) {
        if dx.is_finite() {
            self.accum_dx += dx;
        }
        if dy.is_finite() {
            self.accum_dy += dy;
        }
    }

    /// Advances the DSP filter by `dt_secs` and returns `(stick_x, stick_y)` in `-32767..=32767`.
    #[inline]
    pub fn tick(&mut self, dt_secs: f64) -> (i16, i16) {
        let dx = self.accum_dx;
        let dy = self.accum_dy;
        self.accum_dx = 0.0;
        self.accum_dy = 0.0;

        if !dt_secs.is_finite() || dt_secs <= 0.0 {
            return self.quantize();
        }

        if dx != 0.0 || dy != 0.0 {
            let inv_dt = 1.0 / dt_secs;
            let inst_vx = dx * self.config.sensitivity * inv_dt;
            let inst_vy = dy * self.config.sensitivity * inv_dt;
            let alpha = self.config.ema_alpha.clamp(0.0, 1.0);
            self.vx = alpha * inst_vx + (1.0 - alpha) * self.vx;
            self.vy = alpha * inst_vy + (1.0 - alpha) * self.vy;
        } else {
            let dt_ms = dt_secs * 1000.0;
            let decay = if self.config.decay_half_life_ms > 0.0 {
                0.5_f64.powf(dt_ms / self.config.decay_half_life_ms)
            } else {
                0.0
            };
            self.vx *= decay;
            self.vy *= decay;
        }

        self.quantize()
    }

    /// Advances the DSP filter using the configured `1.0 / hz` nominal timestep.
    #[inline]
    pub fn tick_default(&mut self) -> (i16, i16) {
        let dt = if self.config.hz.is_finite() && self.config.hz > 0.0 {
            1.0 / self.config.hz
        } else {
            1.0 / 250.0
        };
        self.tick(dt)
    }

    #[inline]
    fn quantize(&mut self) -> (i16, i16) {
        let r = self.vx.hypot(self.vy);
        if !r.is_finite() || r < VELOCITY_EPSILON {
            self.vx = 0.0;
            self.vy = 0.0;
            return (0, 0);
        }

        let r_clamped = r.min(1.0);
        let gamma = if self.config.power_gamma.is_finite() && self.config.power_gamma > 0.0 {
            self.config.power_gamma
        } else {
            1.0
        };
        let curved = r_clamped.powf(gamma).clamp(0.0, 1.0);
        let adz = self.config.anti_deadzone.clamp(0.0, 1.0);
        let mag = (adz + (1.0 - adz) * curved).min(1.0);

        let ux = self.vx / r;
        let uy = self.vy / r;
        let out_x = ux * mag;
        let out_y = if self.config.invert_y {
            -(uy * mag)
        } else {
            uy * mag
        };

        quantize_stick_axes(out_x * STICK_MAX_F64, out_y * STICK_MAX_F64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_input_stability() {
        let mut engine = MouseStickEngine::new(MouseStickConfig::default());
        for _ in 0..500 {
            assert_eq!(engine.tick(0.004), (0, 0));
        }
    }

    #[test]
    fn decay_convergence_after_impulse() {
        let mut engine = MouseStickEngine::new(MouseStickConfig::default());
        engine.feed_delta(40.0, -20.0);
        let first = engine.tick(0.004);
        assert_ne!(first, (0, 0));

        let mut prev_mag = (first.0 as f64).hypot(first.1 as f64);
        let mut settled = false;
        for _ in 0..200 {
            let (x, y) = engine.tick(0.004);
            let mag = (x as f64).hypot(y as f64);
            assert!(
                mag <= prev_mag + 1e-9,
                "decay must be monotonically non-increasing: prev={prev_mag}, cur={mag}"
            );
            prev_mag = mag;
            if (x, y) == (0, 0) {
                settled = true;
                break;
            }
        }
        assert!(settled, "engine did not decay to (0, 0) within 200 ticks");
    }

    #[test]
    fn anti_deadzone_floor_on_tiny_movement() {
        let cfg = MouseStickConfig::default();
        let mut engine = MouseStickEngine::new(cfg);
        engine.feed_delta(1.0, 0.0);
        let (x, y) = engine.tick(0.004);
        let floor = cfg.anti_deadzone * STICK_MAX_F64;
        assert!(
            (x as f64) >= floor,
            "expected x ({x}) >= anti_deadzone floor ({floor})"
        );
        assert_eq!(y, 0);
    }

    #[test]
    fn circular_clamping_preserves_45_degree_diagonal() {
        let mut engine = MouseStickEngine::new(MouseStickConfig::default());
        engine.feed_delta(10_000.0, 10_000.0);
        let (x, y) = engine.tick(0.004);
        let hypot = (x as f64).hypot(y as f64);
        assert!(
            hypot <= STICK_MAX_F64,
            "diagonal hypot ({hypot}) exceeded {STICK_MAX_F64}"
        );
        assert_eq!(x, y, "45-degree diagonal must preserve x == y");
        assert!(x > 20_000, "saturated diagonal should be near 23169, got {x}");
    }

    #[test]
    fn invert_y_flips_vertical_sign() {
        let mut normal = MouseStickEngine::new(MouseStickConfig {
            invert_y: false,
            ..MouseStickConfig::default()
        });
        let mut inverted = MouseStickEngine::new(MouseStickConfig {
            invert_y: true,
            ..MouseStickConfig::default()
        });

        normal.feed_delta(15.0, 30.0);
        inverted.feed_delta(15.0, 30.0);

        let (nx, ny) = normal.tick(0.004);
        let (ix, iy) = inverted.tick(0.004);

        assert_eq!(nx, ix);
        assert_eq!(ny, -iy);
        assert!(ny > 0);
        assert!(iy < 0);
    }

    #[test]
    fn extreme_delta_spikes_and_non_finite_safety() {
        let mut engine = MouseStickEngine::new(MouseStickConfig::default());
        engine.feed_delta(f64::NAN, f64::INFINITY);
        assert_eq!(engine.tick(0.004), (0, 0));

        engine.feed_delta(-1e9, 1e9);
        let (x, y) = engine.tick(0.004);
        let hypot = (x as f64).hypot(y as f64);
        assert!(hypot <= STICK_MAX_F64);
        assert_eq!(x, -y);

        // Zero or negative dt must not produce NaN or panic.
        assert_eq!(engine.tick(0.0), (x, y));
        assert_eq!(engine.tick(-0.004), (x, y));
    }

    #[test]
    fn reset_clears_accumulated_deltas_and_velocity() {
        let mut engine = MouseStickEngine::new(MouseStickConfig::default());
        engine.feed_delta(50.0, -30.0);
        let active = engine.tick(0.004);
        assert_ne!(active, (0, 0));

        // Feed another delta that hasn't been ticked yet, then reset.
        engine.feed_delta(100.0, 100.0);
        engine.reset();

        assert_eq!(engine.tick(0.004), (0, 0));
    }
}
