//! Append-only authoritative politics trace for debugging and golden assertions.
//!
//! Records per-office succession evaluations during system execution. Follows
//! the same opt-in pattern as `ActionTraceSink`.

use worldwake_core::{EntityId, SuccessionLaw, Tick};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoliticalTraceEvent {
    pub tick: Tick,
    pub office: EntityId,
    pub trace: OfficeSuccessionTrace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfficeSuccessionTrace {
    pub jurisdiction: EntityId,
    pub succession_law: SuccessionLaw,
    pub holder_before: Option<EntityId>,
    pub vacancy_since_before: Option<Tick>,
    pub availability_phase: OfficeAvailabilityPhase,
    pub vacancy_timer: Option<VacancyTimerTrace>,
    pub outcome: OfficeSuccessionOutcome,
    pub support_declarations: Vec<SupportDeclarationTrace>,
    pub support_resolution: Option<SupportResolutionTrace>,
    pub force_candidates: Vec<ForceCandidateTrace>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VacancyTimerTrace {
    pub start_tick: Tick,
    pub waited_ticks: u64,
    pub required_ticks: u64,
    pub remaining_ticks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportResolutionTrace {
    pub counted_support: Vec<SupportCountTrace>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupportCountTrace {
    pub candidate: EntityId,
    pub support: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OfficeAvailabilityPhase {
    VacantClaimable,
    VacantWaitingForTimer,
    VacantPendingResolution,
    ClosedOccupied,
    VacantReopenedAfterReset,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OfficeSuccessionOutcome {
    OccupiedNoAction {
        holder: EntityId,
        cleared_stale_vacancy: bool,
    },
    VacancyActivated,
    WaitingForTimer,
    SupportInstalled {
        holder: EntityId,
    },
    SupportResetNoEligibleDeclarations,
    SupportResetTie {
        tied_candidates: Vec<EntityId>,
    },
    ForceInstalled {
        holder: EntityId,
    },
    ForceBlocked {
        eligible_contender_count: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportDeclarationTrace {
    pub supporter: EntityId,
    pub candidate: EntityId,
    pub candidate_eligible: bool,
    pub counted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForceCandidateTrace {
    pub candidate: EntityId,
    pub eligible: bool,
}

impl PoliticalTraceEvent {
    fn support_count_for(&self, candidate: EntityId) -> Option<usize> {
        self.trace.support_resolution.as_ref().and_then(|resolution| {
            resolution
                .counted_support
                .iter()
                .find_map(|support| (support.candidate == candidate).then_some(support.support))
        })
    }

    fn max_support_count(&self) -> Option<usize> {
        self.trace.support_resolution.as_ref().and_then(|resolution| {
            resolution
                .counted_support
                .iter()
                .map(|support| support.support)
                .max()
        })
    }

    #[must_use]
    pub fn summary(&self) -> String {
        let phase = self.trace.availability_phase.summary_label();
        let timer_suffix = self
            .trace
            .vacancy_timer
            .map(|timer| {
                format!(
                    ", timer(start {}, waited {}, required {}, remaining {})",
                    timer.start_tick.0,
                    timer.waited_ticks,
                    timer.required_ticks,
                    timer.remaining_ticks
                )
            })
            .unwrap_or_default();
        match &self.trace.outcome {
            OfficeSuccessionOutcome::OccupiedNoAction {
                holder,
                cleared_stale_vacancy,
            } => format!(
                "tick {}: office {} remains occupied by {}{} [{}]",
                self.tick.0,
                self.office,
                holder,
                if *cleared_stale_vacancy {
                    " and clears stale vacancy_since"
                } else {
                    ""
                },
                phase,
            ),
            OfficeSuccessionOutcome::VacancyActivated => format!(
                "tick {}: office {} becomes vacant [{}]",
                self.tick.0, self.office, phase
            ),
            OfficeSuccessionOutcome::WaitingForTimer => {
                let timer = self
                    .trace
                    .vacancy_timer
                    .expect("waiting outcome should include vacancy_timer trace");
                format!(
                    "tick {}: office {} waits for succession timer (start {}, waited {}, required {}, remaining {}) [{}]",
                    self.tick.0,
                    self.office,
                    timer.start_tick.0,
                    timer.waited_ticks,
                    timer.required_ticks,
                    timer.remaining_ticks,
                    phase
                )
            }
            OfficeSuccessionOutcome::SupportInstalled { holder } => format!(
                "tick {}: office {} installs {} by support with {} declarations{} [{}]",
                self.tick.0,
                self.office,
                holder,
                self.support_count_for(*holder)
                    .expect("support install should include support_resolution count for holder"),
                timer_suffix,
                phase
            ),
            OfficeSuccessionOutcome::SupportResetNoEligibleDeclarations => format!(
                "tick {}: office {} resets vacancy clock due to no eligible support declarations{} [{}]",
                self.tick.0, self.office, timer_suffix, phase
            ),
            OfficeSuccessionOutcome::SupportResetTie { tied_candidates } => format!(
                "tick {}: office {} resets vacancy clock due to support tie {:?} at {}{} [{}]",
                self.tick.0,
                self.office,
                tied_candidates,
                self.max_support_count()
                    .expect("support tie should include support_resolution counts"),
                timer_suffix,
                phase
            ),
            OfficeSuccessionOutcome::ForceInstalled { holder } => format!(
                "tick {}: office {} installs {} by force-law uncontested succession [{}]",
                self.tick.0, self.office, holder, phase
            ),
            OfficeSuccessionOutcome::ForceBlocked {
                eligible_contender_count,
            } => format!(
                "tick {}: office {} force-law succession blocked by {} eligible contenders [{}]",
                self.tick.0, self.office, eligible_contender_count, phase
            ),
        }
    }
}

impl OfficeAvailabilityPhase {
    #[must_use]
    pub fn summary_label(self) -> &'static str {
        match self {
            Self::VacantClaimable => "phase: vacant claimable",
            Self::VacantWaitingForTimer => "phase: vacant waiting for timer",
            Self::VacantPendingResolution => "phase: vacant pending resolution",
            Self::ClosedOccupied => "phase: closed occupied",
            Self::VacantReopenedAfterReset => "phase: vacant reopened after reset",
        }
    }
}

pub struct PoliticalTraceSink {
    events: Vec<PoliticalTraceEvent>,
}

impl PoliticalTraceSink {
    #[must_use]
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, event: PoliticalTraceEvent) {
        self.events.push(event);
    }

    #[must_use]
    pub fn events(&self) -> &[PoliticalTraceEvent] {
        &self.events
    }

    #[must_use]
    pub fn events_for_office(&self, office: EntityId) -> Vec<&PoliticalTraceEvent> {
        self.events
            .iter()
            .filter(|event| event.office == office)
            .collect()
    }

    #[must_use]
    pub fn events_at(&self, tick: Tick) -> Vec<&PoliticalTraceEvent> {
        self.events
            .iter()
            .filter(|event| event.tick == tick)
            .collect()
    }

    #[must_use]
    pub fn event_for_office_at(
        &self,
        office: EntityId,
        tick: Tick,
    ) -> Option<&PoliticalTraceEvent> {
        self.events
            .iter()
            .find(|event| event.office == office && event.tick == tick)
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn dump_office(&self, office: EntityId) {
        let events = self.events_for_office(office);
        if events.is_empty() {
            eprintln!("[PoliticalTrace] No events for office {office}");
            return;
        }
        eprintln!(
            "[PoliticalTrace] {} events for office {office}:",
            events.len()
        );
        for event in events {
            eprintln!("  {}", event.summary());
        }
    }
}

impl Default for PoliticalTraceSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn office(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    #[test]
    fn sink_starts_empty() {
        let sink = PoliticalTraceSink::new();
        assert!(sink.events().is_empty());
    }

    #[test]
    fn records_and_queries_by_office_and_tick() {
        let mut sink = PoliticalTraceSink::new();
        let office_a = office(1);
        let office_b = office(2);
        let jurisdiction = office(3);
        let trace = OfficeSuccessionTrace {
            jurisdiction,
            succession_law: SuccessionLaw::Force,
            holder_before: None,
            vacancy_since_before: Some(Tick(1)),
            availability_phase: OfficeAvailabilityPhase::VacantPendingResolution,
            vacancy_timer: Some(VacancyTimerTrace {
                start_tick: Tick(1),
                waited_ticks: 6,
                required_ticks: 5,
                remaining_ticks: 0,
            }),
            outcome: OfficeSuccessionOutcome::ForceBlocked {
                eligible_contender_count: 2,
            },
            support_declarations: Vec::new(),
            support_resolution: None,
            force_candidates: vec![
                ForceCandidateTrace {
                    candidate: office(4),
                    eligible: true,
                },
                ForceCandidateTrace {
                    candidate: office(5),
                    eligible: true,
                },
            ],
        };
        sink.record(PoliticalTraceEvent {
            tick: Tick(7),
            office: office_a,
            trace: trace.clone(),
        });
        sink.record(PoliticalTraceEvent {
            tick: Tick(8),
            office: office_b,
            trace,
        });

        assert_eq!(sink.events_for_office(office_a).len(), 1);
        assert_eq!(sink.events_at(Tick(7)).len(), 1);
        assert!(sink.event_for_office_at(office_a, Tick(7)).is_some());
        assert!(sink.event_for_office_at(office_a, Tick(8)).is_none());
        assert!(sink
            .event_for_office_at(office_a, Tick(7))
            .unwrap()
            .summary()
            .contains("phase: vacant pending resolution"));
    }
}
