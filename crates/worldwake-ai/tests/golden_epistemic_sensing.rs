//! Golden coverage for S139 `AskWitness` epistemic sensing.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::DecisionOutcome;
use worldwake_core::{
    AskWitnessMemory, AskWitnessMemoryKey, BelievedEntityState, CauseRef, ComponentDelta,
    ComponentDiff, ComponentKind, ComponentValue, DecisionEventPayload,
    EpistemicDispositionProfile, EventTag, EventView, GoalKind, HomeostaticNeeds,
    MetabolismProfile, PerceptionSource, Quantity, Seed, StateDelta, TellProfile, TellTopic, Tick,
    UtilityProfile, VisibilitySpec, WitnessData,
};
use worldwake_sim::{ActionTraceDetail, ActionTraceKind};

fn ask_profile() -> EpistemicDispositionProfile {
    EpistemicDispositionProfile {
        stale_evidence_barrier_threshold: pm(800),
        witness_query_duration_ticks: nz(4),
        ask_memory_retention_ticks: 6,
        witness_recency_preference: pm(0),
    }
}

fn set_epistemic_profile(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    profile: EpistemicDispositionProfile,
) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_epistemic_disposition_profile(agent, profile)
        .expect("golden fixture should set epistemic profile");
    commit_txn(txn, &mut h.event_log);
}

fn make_epistemic_fixture(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let seeker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Seeker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let witness = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let subject = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Subject",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    set_epistemic_profile(&mut h, seeker, ask_profile());
    set_epistemic_profile(&mut h, witness, ask_profile());
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        seeker,
        TellProfile {
            max_tell_candidates: 0,
            ..TellProfile::default()
        },
    );

    (h, seeker, witness, subject)
}

fn seed_witness_direct_belief(
    h: &mut GoldenHarness,
    witness: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
    observed_tick: Tick,
) {
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        witness,
        subject,
        observed_tick,
        PerceptionSource::DirectObservation,
    );
}

fn seed_seeker_belief(
    h: &mut GoldenHarness,
    seeker: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
    observed_tick: Tick,
    source: PerceptionSource,
) -> BelievedEntityState {
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        seeker,
        subject,
        observed_tick,
        source,
    )
}

fn seed_ask_witness_memory(
    h: &mut GoldenHarness,
    seeker: worldwake_core::EntityId,
    witness: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
    asked_tick: Tick,
) {
    let mut store = h
        .world
        .get_component_agent_belief_store(seeker)
        .cloned()
        .unwrap_or_else(worldwake_core::AgentBeliefStore::new);
    store.record_asked_witness(
        AskWitnessMemoryKey {
            counterparty: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        },
        AskWitnessMemory { asked_tick },
    );

    let mut txn = new_txn(&mut h.world, asked_tick.0);
    txn.set_component_agent_belief_store(seeker, store)
        .expect("golden fixture should seed ask witness memory");
    commit_txn(txn, &mut h.event_log);
}

fn is_ask_witness_goal(
    goal: GoalKind,
    witness: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
) -> bool {
    matches!(
        goal,
        GoalKind::AskWitness {
            witness: actual_witness,
            topic: TellTopic::EntityBelief {
                subject: actual_subject
            },
        } if actual_witness == witness && actual_subject == subject
    )
}

fn ask_witness_selected_tick(
    h: &GoldenHarness,
    seeker: worldwake_core::EntityId,
    witness: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
) -> Option<Tick> {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(seeker)
        .into_iter()
        .find_map(|trace| {
            let DecisionOutcome::Planning(ref pipeline) = trace.outcome else {
                return None;
            };
            pipeline
                .selection
                .selected_goal()
                .is_some_and(|goal| is_ask_witness_goal(goal.kind, witness, subject))
                .then_some(trace.tick)
        })
}

fn run_until_ask_witness_commit(
    h: &mut GoldenHarness,
    seeker: worldwake_core::EntityId,
    witness: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
    tick_budget: u32,
) -> Tick {
    for _ in 0..tick_budget {
        h.step_once();
        if let Some(tick) = ask_witness_selected_tick(h, seeker, witness, subject) {
            return tick;
        }
    }
    panic!(
        "expected AskWitness commit for seeker={seeker:?} witness={witness:?} subject={subject:?}; traces={:#?}",
        h.driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .traces_for(seeker)
    );
}

fn ask_witness_action_committed(
    h: &GoldenHarness,
    seeker: worldwake_core::EntityId,
    witness: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
) -> bool {
    h.action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(seeker)
        .into_iter()
        .any(|event| {
            event.action_name == "ask_witness"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && matches!(
                    event.detail,
                    Some(ActionTraceDetail::AskWitness {
                        target,
                        topic_entity: Some(topic_entity),
                        ..
                    }) if target == witness && topic_entity == subject
                )
        })
}

fn run_until_ask_witness_action_commit(
    h: &mut GoldenHarness,
    seeker: worldwake_core::EntityId,
    witness: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
    tick_budget: u32,
) {
    for _ in 0..tick_budget {
        h.step_once();
        if ask_witness_action_committed(h, seeker, witness, subject) {
            return;
        }
    }
    panic!(
        "expected ask_witness action commit; action_trace={:#?}",
        h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(seeker)
    );
}

fn imported_report_from(
    h: &GoldenHarness,
    seeker: worldwake_core::EntityId,
    witness: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
) -> Option<BelievedEntityState> {
    agent_belief_about(&h.world, seeker, subject)
        .filter(|belief| matches!(belief.source, PerceptionSource::Report { from, chain_len: 1 } if from == witness))
        .cloned()
}

fn belief_store_delta_writes_report_from(
    h: &GoldenHarness,
    seeker: worldwake_core::EntityId,
    witness: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
) -> bool {
    (0..h.event_log.len()).any(|index| {
        let Some(record) = h.event_log.get(worldwake_core::EventId(index as u64)) else {
            return false;
        };
        record.state_deltas().iter().any(|delta| {
            match delta {
                StateDelta::Component(ComponentDelta::Set {
                    entity,
                    component_kind: ComponentKind::AgentBeliefStore,
                    after: ComponentValue::AgentBeliefStore(store),
                    ..
                }) => {
                    *entity == seeker
                        && store.get_entity(&subject).is_some_and(|belief| {
                            matches!(belief.source, PerceptionSource::Report { from, chain_len: 1 } if from == witness)
                        })
                }
                StateDelta::Component(ComponentDelta::CompactSet {
                    entity,
                    component_kind: ComponentKind::AgentBeliefStore,
                    diff: ComponentDiff::BeliefStore(diff),
                }) => {
                    *entity == seeker
                        && diff.known_entities_set.iter().any(|(entity, belief)| {
                            *entity == subject
                                && matches!(belief.source, PerceptionSource::Report { from, chain_len: 1 } if from == witness)
                        })
                }
                _ => false,
            }
        })
    })
}

fn ask_witness_observation(seed: Seed, rumor: bool) -> (Tick, bool) {
    let (mut h, seeker, witness, subject) = make_epistemic_fixture(seed);
    seed_witness_direct_belief(&mut h, witness, subject, Tick(0));
    let source = if rumor {
        PerceptionSource::Rumor { chain_len: 1 }
    } else {
        PerceptionSource::Report {
            from: witness,
            chain_len: 1,
        }
    };
    seed_seeker_belief(&mut h, seeker, subject, Tick(0), source);

    let selected_tick = run_until_ask_witness_commit(&mut h, seeker, witness, subject, 12);
    run_until_ask_witness_action_commit(&mut h, seeker, witness, subject, 12);
    let _imported = imported_report_from(&h, seeker, witness, subject)
        .expect("ask_witness should import witness belief with report provenance");

    (
        selected_tick,
        belief_store_delta_writes_report_from(&h, seeker, witness, subject),
    )
}

// Scenario 415: S139 AskWitness Refreshes Stale Report
// Systems: AI, EpistemicActions, EventLog
// GoalKinds: AskWitness
// ActionDomains: Epistemic
// Principles: P7, P14, P15, P29
// Setup: a seeker has a low-confidence report-sourced entity belief from a
//        co-located witness; rival self-care pressure is absent.
// Proves: the AI emits and commits AskWitness, the existing action imports the
//         witness belief through Report provenance, and the belief-store delta
//         records the refresh path.
// Cross-system chain: stale report -> candidate/ranking/plan -> ask_witness
//                     action -> AgentBeliefStore report provenance.
#[test]
fn golden_ask_witness_refreshes_stale_report() {
    let (_commit_tick, wrote_report_delta) = ask_witness_observation(Seed([139; 32]), false);

    assert!(
        wrote_report_delta,
        "event-log deltas should include the AgentBeliefStore report-provenance write"
    );
}

#[test]
fn golden_ask_witness_refreshes_stale_report_replay_is_deterministic() {
    let first = ask_witness_observation(Seed([140; 32]), false);
    let second = ask_witness_observation(Seed([140; 32]), false);

    assert_eq!(first, second);
}

// Scenario 416: S139 AskWitness Cold-Start Local Witness
// Systems: AI, EpistemicActions, EventLog
// GoalKinds: AskWitness
// ActionDomains: Epistemic
// Principles: P7, P14, P15
// Setup: the seeker has only a low-confidence rumor about the subject, while a
//        co-located witness has direct belief about that subject.
// Proves: the cold-start branch emits AskWitness without prior testimony from
//         that witness and imports the witness's belief as Report provenance.
// Cross-system chain: rumor topic -> local witness candidate -> ask_witness
//                     action -> Report provenance.
#[test]
fn golden_ask_witness_cold_start_imports_local_witness_report() {
    let (_commit_tick, wrote_report_delta) = ask_witness_observation(Seed([141; 32]), true);

    assert!(wrote_report_delta);
}

// Scenario 417: S139 Critical Survival Suppresses AskWitness
// Systems: AI, Needs, EpistemicSensing
// GoalKinds: AskWitness, ConsumeOwnedCommodity
// ActionDomains: Epistemic, Needs
// Principles: P8, P20, P29
// Setup: a critically hungry seeker has local bread and a low-confidence
//        AskWitness topic; the intended branch excludes remote acquisition.
// Proves: the epistemic candidate is suppressed by stress policy while the
//         self-care action remains available and commits.
// Cross-system chain: critical hunger -> ranked stress context -> suppression
//                     policy -> self-care commit.
#[test]
fn golden_ask_witness_critical_survival_suppression() {
    let (mut h, seeker, witness, subject) = make_epistemic_fixture(Seed([142; 32]));
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_homeostatic_needs(
        seeker,
        HomeostaticNeeds::new(pm(950), pm(0), pm(0), pm(0), pm(0)),
    )
    .expect("golden fixture should set seeker hunger");
    commit_txn(txn, &mut h.event_log);
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        seeker,
        VILLAGE_SQUARE,
        worldwake_core::CommodityKind::Bread,
        Quantity(1),
    );
    seed_witness_direct_belief(&mut h, witness, subject, Tick(0));
    seed_seeker_belief(
        &mut h,
        seeker,
        subject,
        Tick(0),
        PerceptionSource::Rumor { chain_len: 1 },
    );

    for _ in 0..12 {
        h.step_once();
    }

    let suppressed = h
        .event_log
        .events_by_tag(EventTag::GoalSuppressed)
        .iter()
        .copied()
        .any(|event_id| {
            h.event_log
                .get(event_id)
                .and_then(EventView::decision_payload)
                .is_some_and(|payload| {
                    matches!(
                        payload,
                        DecisionEventPayload::GoalSuppressed(inner)
                            if inner.agent == seeker
                                && is_ask_witness_goal(inner.goal_key.kind, witness, subject)
                    )
                })
        });
    let ate = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(seeker)
        .into_iter()
        .any(|event| {
            event.action_name == "eat" && matches!(event.kind, ActionTraceKind::Committed { .. })
        });

    assert!(
        suppressed,
        "AskWitness should be suppressed under critical self-care stress"
    );
    assert!(
        ate,
        "critical self-care should proceed instead of epistemic sensing"
    );
}

// Scenario 418: S139 AskWitness Cooldown Gate
// Systems: AI, EpistemicSensing, BeliefMemory
// GoalKinds: AskWitness
// ActionDomains: Epistemic
// Principles: P20, P21, P29
// Setup: the seeker has a low-confidence AskWitness topic and a live
//        AskWitnessMemory entry for the same witness/topic pair.
// Proves: the cooldown gate suppresses AskWitness before retention elapses
//         and emission resumes once the retained memory expires.
// Cross-system chain: AskWitnessMemory -> candidate gate -> no selected plan
//                     before expiry -> selected AskWitness plan after expiry.
#[test]
fn golden_ask_witness_cooldown_gate_resumes_after_retention() {
    let (mut h, seeker, witness, subject) = make_epistemic_fixture(Seed([144; 32]));
    seed_witness_direct_belief(&mut h, witness, subject, Tick(0));
    seed_seeker_belief(
        &mut h,
        seeker,
        subject,
        Tick(0),
        PerceptionSource::Report {
            from: witness,
            chain_len: 1,
        },
    );
    seed_ask_witness_memory(&mut h, seeker, witness, subject, Tick(0));

    for _ in 0..5 {
        h.step_once();
        assert!(
            ask_witness_selected_tick(&h, seeker, witness, subject).is_none(),
            "AskWitness should stay suppressed while memory retention is active"
        );
    }

    let resumed = run_until_ask_witness_commit(&mut h, seeker, witness, subject, 4);
    assert!(
        resumed.0 >= 6,
        "AskWitness should resume only after ask_memory_retention_ticks elapses"
    );
}

// Scenario 419: S139 Witness Relocation Revalidates AskWitness
// Systems: AI, EpistemicActions, ActionTrace
// GoalKinds: AskWitness
// ActionDomains: Epistemic
// Principles: P7, P21, P29
// Setup: after the seeker commits an AskWitness plan, the witness relocates
//        before the action can commit; the remote-query branch is intentionally
//        excluded because S139 only emits co-located witness inquiries.
// Proves: the retained ask step does not import a report after the witness has
//         moved, and a later decision trace remains available for replanning.
// Cross-system chain: goal commit -> witness movement -> action revalidation
//                     failure/no report import -> next decision trace.
#[test]
fn golden_ask_witness_revalidates_when_witness_relocates_before_commit() {
    let (mut h, seeker, witness, subject) = make_epistemic_fixture(Seed([143; 32]));
    seed_witness_direct_belief(&mut h, witness, subject, Tick(0));
    seed_seeker_belief(
        &mut h,
        seeker,
        subject,
        Tick(0),
        PerceptionSource::Rumor { chain_len: 1 },
    );
    run_until_ask_witness_commit(&mut h, seeker, witness, subject, 12);

    for _ in 0..8 {
        h.step_once();
        let started = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(seeker)
            .into_iter()
            .any(|event| {
                event.action_name == "ask_witness"
                    && matches!(event.kind, ActionTraceKind::Started { .. })
            });
        if started {
            let mut txn = worldwake_core::WorldTxn::new(
                &mut h.world,
                h.scheduler.current_tick(),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::Hidden,
                WitnessData::default(),
            );
            txn.set_ground_location(witness, ORCHARD_FARM)
                .expect("golden fixture should relocate witness");
            commit_txn(txn, &mut h.event_log);
            break;
        }
    }

    for _ in 0..8 {
        h.step_once();
    }

    assert!(
        imported_report_from(&h, seeker, witness, subject).is_none(),
        "moved witness should not satisfy AskWitness through a stale co-location assumption"
    );
    let cutoff = Tick(h.scheduler.current_tick().0.saturating_sub(8));
    assert!(
        h.driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .traces_for(seeker)
            .into_iter()
            .any(|trace| trace.tick > cutoff
                && matches!(trace.outcome, DecisionOutcome::Planning(_))),
        "seeker should continue through the planning/replan surface after the failed ask"
    );
}
