use std::env;
use std::hint::{black_box, spin_loop};
use std::thread;
use std::time::{Duration, Instant};

use dsp_spike::{MouseStickConfig, MouseStickEngine};

fn run_bench() {
    let cfg = MouseStickConfig::default();
    let target_hz = cfg.hz;
    let total_secs = 2.0_f64;
    let total_ticks = (target_hz * total_secs).round() as usize;
    let period_ns = (1_000_000_000.0 / target_hz).round() as u64;
    let period = Duration::from_nanos(period_ns);
    let target_us = period_ns as f64 / 1_000.0;
    // Leave 750 us for spin-wait to absorb OS sleep wakeup overshoot.
    let spin_margin = Duration::from_micros(750);

    let mut engine = MouseStickEngine::new(cfg);
    let mut intervals_us = Vec::with_capacity(total_ticks);
    let mut jitters_us = Vec::with_capacity(total_ticks);

    let mut prev_tick = Instant::now();
    let mut next_deadline = prev_tick + period;

    for i in 0..total_ticks {
        let now = Instant::now();
        if let Some(remaining) = next_deadline.checked_duration_since(now) {
            if remaining > spin_margin {
                thread::sleep(remaining - spin_margin);
            }
            while Instant::now() < next_deadline {
                spin_loop();
            }
        }

        let tick_now = Instant::now();
        let actual_dt = tick_now.duration_since(prev_tick);
        let actual_us = actual_dt.as_nanos() as f64 / 1_000.0;
        intervals_us.push(actual_us);
        jitters_us.push((actual_us - target_us).abs());

        // Simulate 125Hz mouse packets arriving on even ticks of a 250Hz loop.
        if i % 2 == 0 {
            engine.feed_delta(6.0, -2.0);
        }
        let out = engine.tick(actual_dt.as_secs_f64());
        black_box(out);

        prev_tick = tick_now;
        next_deadline += period;
        // Resync deadline if OS stalled past one full period.
        if next_deadline < tick_now {
            next_deadline = tick_now + period;
        }
    }

    let sum_us: f64 = intervals_us.iter().sum();
    let mean_us = sum_us / intervals_us.len() as f64;

    jitters_us.sort_by(|a, b| a.total_cmp(b));
    let p95_idx = ((jitters_us.len() as f64) * 0.95).ceil() as usize - 1;
    let p99_idx = ((jitters_us.len() as f64) * 0.99).ceil() as usize - 1;
    let p95_jitter = jitters_us[p95_idx.min(jitters_us.len() - 1)];
    let p99_jitter = jitters_us[p99_idx.min(jitters_us.len() - 1)];
    let max_jitter = *jitters_us.last().unwrap_or(&0.0);

    println!("=== GamepadV DSP Spike 0.3 Benchmark ===");
    println!("target_hz        : {target_hz:.1} Hz (period = {target_us:.2} us)");
    println!("total_ticks      : {total_ticks} ({total_secs:.1} s)");
    println!("mean_interval_us : {mean_us:.2} us");
    println!("p95_jitter_us    : {p95_jitter:.2} us");
    println!("p99_jitter_us    : {p99_jitter:.2} us");
    println!("max_jitter_us    : {max_jitter:.2} us");
}

fn main() {
    if env::args().any(|arg| arg == "--bench") {
        run_bench();
    } else {
        println!("Usage: dsp_spike --bench");
    }
}
