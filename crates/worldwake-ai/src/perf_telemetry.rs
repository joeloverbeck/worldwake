use std::cell::RefCell;
use std::time::Duration;
use worldwake_core::Tick;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TickWindow {
    pub start_tick: u64,
    pub end_tick_exclusive: u64,
}

impl TickWindow {
    pub const fn contains(self, tick: Tick) -> bool {
        tick.0 >= self.start_tick && tick.0 < self.end_tick_exclusive
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanningTelemetryConfig {
    pub early: TickWindow,
    pub late: TickWindow,
}

pub const SOAK_SEED_PERF_TELEMETRY_CONFIG: PlanningTelemetryConfig = PlanningTelemetryConfig {
    early: TickWindow {
        start_tick: 100,
        end_tick_exclusive: 500,
    },
    late: TickWindow {
        start_tick: 9000,
        end_tick_exclusive: 10000,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanningWindowSummary {
    pub window: TickWindow,
    pub sample_count: u64,
    pub total_duration_nanos: u128,
}

impl PlanningWindowSummary {
    pub fn average_duration_nanos(self) -> Option<u128> {
        if self.sample_count > 0 {
            Some(self.total_duration_nanos / u128::from(self.sample_count))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanningTelemetrySummary {
    pub early: PlanningWindowSummary,
    pub late: PlanningWindowSummary,
}

impl PlanningTelemetrySummary {
    pub fn late_to_early_average_ratio_millis(self) -> Option<u128> {
        if self.early.sample_count == 0
            || self.late.sample_count == 0
            || self.early.total_duration_nanos == 0
        {
            return None;
        }
        Some(
            self.late
                .total_duration_nanos
                .saturating_mul(u128::from(self.early.sample_count))
                .saturating_mul(1000)
                / self
                    .early
                    .total_duration_nanos
                    .saturating_mul(u128::from(self.late.sample_count)),
        )
    }
}

#[derive(Debug)]
struct PlanningTelemetryCollector {
    config: PlanningTelemetryConfig,
    early_sample_count: u64,
    early_total_duration_nanos: u128,
    late_sample_count: u64,
    late_total_duration_nanos: u128,
}

impl PlanningTelemetryCollector {
    fn new(config: PlanningTelemetryConfig) -> Self {
        Self {
            config,
            early_sample_count: 0,
            early_total_duration_nanos: 0,
            late_sample_count: 0,
            late_total_duration_nanos: 0,
        }
    }

    fn record(&mut self, tick: Tick, elapsed: Duration) {
        let elapsed_nanos = elapsed.as_nanos();
        if self.config.early.contains(tick) {
            self.early_sample_count += 1;
            self.early_total_duration_nanos += elapsed_nanos;
        }
        if self.config.late.contains(tick) {
            self.late_sample_count += 1;
            self.late_total_duration_nanos += elapsed_nanos;
        }
    }

    fn finish(self) -> PlanningTelemetrySummary {
        PlanningTelemetrySummary {
            early: PlanningWindowSummary {
                window: self.config.early,
                sample_count: self.early_sample_count,
                total_duration_nanos: self.early_total_duration_nanos,
            },
            late: PlanningWindowSummary {
                window: self.config.late,
                sample_count: self.late_sample_count,
                total_duration_nanos: self.late_total_duration_nanos,
            },
        }
    }
}

thread_local! {
    static PLANNING_TELEMETRY: RefCell<Option<PlanningTelemetryCollector>> = const { RefCell::new(None) };
}

pub struct PlanningTelemetrySession {
    active: bool,
}

impl PlanningTelemetrySession {
    pub fn start(config: PlanningTelemetryConfig) -> Self {
        PLANNING_TELEMETRY.with(|cell| {
            let mut slot = cell.borrow_mut();
            assert!(
                slot.is_none(),
                "planning telemetry session already active on this thread"
            );
            *slot = Some(PlanningTelemetryCollector::new(config));
        });
        Self { active: true }
    }

    pub fn finish(mut self) -> PlanningTelemetrySummary {
        self.active = false;
        PLANNING_TELEMETRY.with(|cell| {
            cell.borrow_mut()
                .take()
                .expect("planning telemetry session should be active")
                .finish()
        })
    }
}

impl Drop for PlanningTelemetrySession {
    fn drop(&mut self) {
        if self.active {
            PLANNING_TELEMETRY.with(|cell| {
                cell.borrow_mut().take();
            });
        }
    }
}

pub fn record_planning_phase_duration(tick: Tick, elapsed: Duration) {
    PLANNING_TELEMETRY.with(|cell| {
        if let Some(collector) = cell.borrow_mut().as_mut() {
            collector.record(tick, elapsed);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        PlanningTelemetryConfig, PlanningTelemetrySession, TickWindow, record_planning_phase_duration,
    };
    use std::time::Duration;
    use worldwake_core::Tick;

    #[test]
    fn records_only_configured_windows() {
        let session = PlanningTelemetrySession::start(PlanningTelemetryConfig {
            early: TickWindow {
                start_tick: 10,
                end_tick_exclusive: 20,
            },
            late: TickWindow {
                start_tick: 30,
                end_tick_exclusive: 40,
            },
        });

        record_planning_phase_duration(Tick(9), Duration::from_micros(10));
        record_planning_phase_duration(Tick(10), Duration::from_micros(11));
        record_planning_phase_duration(Tick(19), Duration::from_micros(13));
        record_planning_phase_duration(Tick(20), Duration::from_micros(17));
        record_planning_phase_duration(Tick(35), Duration::from_micros(19));

        let summary = session.finish();
        assert_eq!(summary.early.sample_count, 2);
        assert_eq!(summary.early.total_duration_nanos, 24_000);
        assert_eq!(summary.late.sample_count, 1);
        assert_eq!(summary.late.total_duration_nanos, 19_000);
    }

    #[test]
    fn computes_late_to_early_ratio_from_average_duration() {
        let session = PlanningTelemetrySession::start(PlanningTelemetryConfig {
            early: TickWindow {
                start_tick: 0,
                end_tick_exclusive: 10,
            },
            late: TickWindow {
                start_tick: 10,
                end_tick_exclusive: 20,
            },
        });

        record_planning_phase_duration(Tick(1), Duration::from_micros(10));
        record_planning_phase_duration(Tick(2), Duration::from_micros(20));
        record_planning_phase_duration(Tick(11), Duration::from_micros(60));

        let summary = session.finish();
        assert_eq!(summary.late_to_early_average_ratio_millis(), Some(4000));
    }
}
