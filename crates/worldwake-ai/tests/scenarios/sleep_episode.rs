//! Golden coverage for S128 sleep episodes.

use std::num::NonZeroU32;

use crate::golden_harness::*;
use worldwake_ai::DecisionOutcome;
use worldwake_core::{
    AgentData, ControlSource, DecisionEventPayload, EntityId, EventTag, EventView,
    ExpectationStore, FrameAssumption, FrameState, GoalKey, GoalKind, GroundComfortTag,
    HomeostaticNeedId, HomeostaticNeeds, IntentionDomain, IntentionFrame, MetabolismProfile,
    OpportunityAnchor, OpportunityKey, PerceptionSource, Permille, RestCapacity, Seed, ShelterTag,
    SleepEpisodeEndedPayload, SleepEpisodeStartedPayload, SleepQualityProfile,
    SleepRecoveryModifier, Tick, WakeCondition, WakeReason,
};
use worldwake_sim::{ActionRequestMode, ActionTraceKind, InputKind, RequestProvenance};

fn set_control_source(
    h: &mut GoldenHarness,
    agent: EntityId,
    control_source: ControlSource,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn request_simple_action(h: &mut GoldenHarness, actor: EntityId, def_name: &str) {
    request_action_with_targets(h, actor, def_name, vec![]);
}

fn request_action_with_targets(
    h: &mut GoldenHarness,
    actor: EntityId,
    def_name: &str,
    targets: Vec<EntityId>,
) {
    let def_id = h.defs.iter().find(|def| def.name == def_name).map_or_else(
        || panic!("full registries should include {def_name}"),
        |def| def.id,
    );
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor,
            def_id,
            targets,
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
}

fn set_sleep_quality(h: &mut GoldenHarness, place: EntityId, recovery_modifier: u16) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_sleep_quality_profile(
        place,
        SleepQualityProfile {
            shelter: ShelterTag::Roofed,
            ground_comfort: GroundComfortTag::Earth,
            recovery_modifier: SleepRecoveryModifier::new(recovery_modifier),
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_rest_capacity(h: &mut GoldenHarness, place: EntityId, capacity: u32) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_rest_capacity(place, RestCapacity(NonZeroU32::new(capacity).unwrap()))
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn quiet_metabolism(rest_efficiency: u16) -> MetabolismProfile {
    MetabolismProfile {
        hunger_rate: pm(0),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        rest_efficiency: pm(rest_efficiency),
        min_sleep_ticks: nz(4),
        ..MetabolismProfile::default()
    }
}

fn sleep_agent(h: &mut GoldenHarness, name: &str, place: EntityId, fatigue: u16) -> EntityId {
    seed_agent(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        HomeostaticNeeds::new(pm(50), pm(50), pm(fatigue), pm(50), pm(50)),
        quiet_metabolism(100),
        worldwake_core::UtilityProfile {
            fatigue_weight: pm(1000),
            hunger_weight: pm(50),
            thirst_weight: pm(50),
            bladder_weight: pm(50),
            dirtiness_weight: pm(50),
            ..worldwake_core::UtilityProfile::default()
        },
    )
}

fn sleep_started_payloads(h: &GoldenHarness, sleeper: EntityId) -> Vec<SleepEpisodeStartedPayload> {
    h.event_log
        .events_by_tag(EventTag::SleepEpisodeStarted)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .filter_map(|record| match record.decision_payload()? {
            DecisionEventPayload::SleepEpisodeStarted(payload) if payload.sleeper == sleeper => {
                Some(payload.clone())
            }
            _ => None,
        })
        .collect()
}

fn sleep_ended_payloads(h: &GoldenHarness, sleeper: EntityId) -> Vec<SleepEpisodeEndedPayload> {
    h.event_log
        .events_by_tag(EventTag::SleepEpisodeEnded)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .filter_map(|record| match record.decision_payload()? {
            DecisionEventPayload::SleepEpisodeEnded(payload) if payload.sleeper == sleeper => {
                Some(payload.clone())
            }
            _ => None,
        })
        .collect()
}

fn run_until_sleep_ended(
    h: &mut GoldenHarness,
    sleeper: EntityId,
    tick_budget: u32,
) -> SleepEpisodeEndedPayload {
    for _ in 0..tick_budget {
        h.step_once();
        if let Some(payload) = sleep_ended_payloads(h, sleeper).into_iter().next() {
            return payload;
        }
    }
    panic!("sleep episode should end within {tick_budget} ticks");
}

fn seed_intention_frame_with_hunger_breach(h: &mut GoldenHarness, agent: EntityId, breach: Tick) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_expectation_store(agent, ExpectationStore::default())
        .unwrap();
    txn.set_component_intention_frame(
        agent,
        IntentionFrame {
            goal: GoalKey::from(GoalKind::Sleep),
            domain: IntentionDomain::Generic,
            assumptions: vec![FrameAssumption::NeedSafeUntilTick {
                need: HomeostaticNeedId::Hunger,
                until_tick: breach,
            }],
            state: FrameState::Active,
            established_at: Tick(0),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 16,
            motive_refs: Vec::new(),
            resume_conditions: Vec::new(),
            abandon_conditions: Vec::new(),
            explicit_claims: Vec::new(),
            causal_links: Vec::new(),
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);
}

// Scenario 356: Sleep Episode Lifecycle
// Setup: One fatigued AI agent at a default-quality place with no competing needs.
// Proves: One duration-bearing Sleep episode starts, commits once, records payloads, and applies recovery.
// Chain: fatigue pressure -> Sleep candidate -> SleepEpisode tick recovery -> SleepEpisodeEnded -> fatigue.
#[test]
fn sleep_episode_at_default_place_runs_to_intended_max() {
    let mut h = GoldenHarness::new(Seed([0x81; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();
    let agent = sleep_agent(&mut h, "Drowsy", VILLAGE_SQUARE, 900);
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let ended = run_until_sleep_ended(&mut h, agent, 32);
    let started = sleep_started_payloads(&h, agent);
    assert_eq!(started.len(), 1);
    assert_eq!(sleep_ended_payloads(&h, agent).len(), 1);
    assert_eq!(started[0].place, VILLAGE_SQUARE);
    assert_eq!(ended.place, VILLAGE_SQUARE);
    assert!(matches!(
        ended.end_reason,
        WakeReason::IntendedDuration | WakeReason::TargetRecovery
    ));
    assert_eq!(
        h.world
            .get_component_homeostatic_needs(agent)
            .unwrap()
            .fatigue,
        pm(900).saturating_sub(ended.accumulated_recovery)
    );

    let sleep_events = h
        .action_trace_sink()
        .unwrap()
        .events_for(agent)
        .into_iter()
        .filter(|event| event.action_name == "sleep")
        .collect::<Vec<_>>();
    assert_eq!(
        sleep_events
            .iter()
            .filter(|event| matches!(event.kind, ActionTraceKind::Started { .. }))
            .count(),
        1
    );
    assert_eq!(
        sleep_events
            .iter()
            .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
            .count(),
        1
    );
}

// Scenario 357: Projected Hunger Breach Wakes Sleep
// Setup: A scripted human-controlled sleep start preserves a preloaded S126 NeedSafeUntilTick(Hunger) assumption.
// Proves: WakeCondition::ProjectedNeedBreach ends sleep early and records the projected breach tick.
// Chain: IntentionFrame assumption -> wake-condition synthesis -> sleep tick -> SleepEpisodeEnded(ProjectedNeedBreach).
#[test]
fn projected_hunger_breach_wakes_sleep_early() {
    let mut h = GoldenHarness::new(Seed([0x82; 32]));
    h.enable_action_tracing();
    let agent = sleep_agent(&mut h, "HungrySleeper", VILLAGE_SQUARE, 900);
    set_control_source(&mut h, agent, ControlSource::Human, 0);
    seed_intention_frame_with_hunger_breach(&mut h, agent, Tick(3));
    request_simple_action(&mut h, agent, "sleep");

    let ended = run_until_sleep_ended(&mut h, agent, 16);
    let started = sleep_started_payloads(&h, agent)
        .into_iter()
        .next()
        .expect("sleep start payload should exist");
    assert!(started.intended_max_ticks.get() > 3);
    assert_eq!(
        ended.end_reason,
        WakeReason::ProjectedNeedBreach {
            need: HomeostaticNeedId::Hunger,
            projected_breach_tick: Tick(3),
        }
    );
    assert!(ended.end_tick.0 < u64::from(started.intended_max_ticks.get()));
}

// Scenario 358: Sleep Place Quality Modulates Recovery
// Setup: Two human-controlled sleepers start identical episodes at places with 900 and 700 recovery modifiers.
// Proves: Place-quality recovery modifiers change accumulated recovery and final fatigue.
// Chain: SleepQualityProfile -> cached recovery_modifier -> recovery formula -> SleepEpisodeEnded -> fatigue.
#[test]
fn place_quality_modulates_per_tick_recovery() {
    let mut h = GoldenHarness::new(Seed([0x83; 32]));
    set_sleep_quality(&mut h, VILLAGE_SQUARE, 900);
    set_sleep_quality(&mut h, ORCHARD_FARM, 700);
    set_rest_capacity(&mut h, VILLAGE_SQUARE, 1);
    set_rest_capacity(&mut h, ORCHARD_FARM, 1);
    let better = sleep_agent(&mut h, "Better", VILLAGE_SQUARE, 800);
    let worse = sleep_agent(&mut h, "Worse", ORCHARD_FARM, 800);
    set_control_source(&mut h, better, ControlSource::Human, 0);
    set_control_source(&mut h, worse, ControlSource::Human, 0);
    request_action_with_targets(&mut h, better, "sleep", vec![VILLAGE_SQUARE]);
    request_action_with_targets(&mut h, worse, "sleep", vec![ORCHARD_FARM]);

    let better_end = run_until_sleep_ended(&mut h, better, 16);
    let worse_end = run_until_sleep_ended(&mut h, worse, 16);

    assert_eq!(better_end.end_tick, worse_end.end_tick);
    assert_eq!(better_end.accumulated_recovery, pm(810));
    assert_eq!(worse_end.accumulated_recovery, pm(630));
    assert_eq!(better_end.final_fatigue, pm(0));
    assert_eq!(worse_end.final_fatigue, pm(170));
}

// Scenario 359: Interrupted Sleep Records Partial Recovery
// Setup: A projected hunger breach cuts off sleep before full recovery.
// Proves: Partial accumulated recovery is preserved exactly in the event payload and authoritative fatigue.
// Chain: projected breach -> early commit -> SleepEpisodeEnded.accumulated_recovery -> fatigue subtraction.
#[test]
fn interrupted_sleep_records_partial_recovery() {
    let mut h = GoldenHarness::new(Seed([0x84; 32]));
    let agent = sleep_agent(&mut h, "Partial", VILLAGE_SQUARE, 900);
    set_control_source(&mut h, agent, ControlSource::Human, 0);
    seed_intention_frame_with_hunger_breach(&mut h, agent, Tick(2));
    request_simple_action(&mut h, agent, "sleep");

    let ended = run_until_sleep_ended(&mut h, agent, 16);

    assert!(ended.accumulated_recovery > Permille::ZERO);
    assert!(ended.accumulated_recovery < pm(900));
    assert_eq!(
        h.world
            .get_component_homeostatic_needs(agent)
            .unwrap()
            .fatigue,
        pm(900).saturating_sub(ended.accumulated_recovery)
    );
}

// Scenario 360: Sleep Site Preference Ranks Higher Quality Place
// Setup: A fatigued AI agent knows both its current 900-quality camp and a reachable 1000-quality orchard.
// Proves: Per-place Sleep opportunities rank by anchored SleepQualityProfile and select the higher-quality place.
// Chain: place belief -> per-place Sleep candidate -> ranking modifier -> selected OpportunityKey.
#[test]
fn site_preference_adopts_higher_quality_sleep_place() {
    let mut h = GoldenHarness::new(Seed([0x85; 32]));
    h.driver.enable_tracing();
    set_sleep_quality(&mut h, VILLAGE_SQUARE, 900);
    set_sleep_quality(&mut h, ORCHARD_FARM, 1000);
    let agent = sleep_agent(&mut h, "Chooser", VILLAGE_SQUARE, 500);
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::Inference,
    );

    h.step_once();
    let trace = h
        .driver
        .trace_sink()
        .and_then(|sink| sink.trace_at(agent, Tick(0)))
        .expect("first decision trace should exist");
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        panic!("agent should run planning on the first tick");
    };
    let sleep_goal = GoalKey::from(GoalKind::Sleep);
    let current = OpportunityKey {
        goal_key: sleep_goal,
        anchor: OpportunityAnchor::Place(VILLAGE_SQUARE),
    };
    let high_quality = OpportunityKey {
        goal_key: sleep_goal,
        anchor: OpportunityAnchor::Place(ORCHARD_FARM),
    };
    assert!(planning.candidates.generated_contains_opportunity(current));
    assert!(
        planning
            .candidates
            .generated_contains_opportunity(high_quality)
    );
    let ranked = planning.candidates.ranked_summaries_for_goal(sleep_goal);
    assert!(
        ranked.len() >= 2,
        "expected at least two ranked Sleep opportunities, got {ranked:?}"
    );
    assert_eq!(ranked[0].opportunity, high_quality);
    assert_eq!(planning.selection.selected_opportunity, Some(high_quality));
}

// Scenario 361: Sleep Episode Events Carry Decision-History Payloads
// Setup: A minimal scripted sleep episode runs to completion through the full action registry.
// Proves: SleepEpisodeStarted and SleepEpisodeEnded carry fields consumed by the observer decision renderer.
// Chain: sleep start/end events -> DecisionEventPayload variants -> observer-renderable names and summaries.
#[test]
fn sleep_episode_events_render_in_decision_trace() {
    let mut h = GoldenHarness::new(Seed([0x86; 32]));
    let agent = sleep_agent(&mut h, "Traceable", VILLAGE_SQUARE, 500);
    set_control_source(&mut h, agent, ControlSource::Human, 0);
    request_simple_action(&mut h, agent, "sleep");

    let ended = run_until_sleep_ended(&mut h, agent, 16);
    let started = sleep_started_payloads(&h, agent)
        .into_iter()
        .next()
        .expect("sleep start payload should exist");

    assert_eq!(started.sleeper, agent);
    assert_eq!(started.place, VILLAGE_SQUARE);
    assert!(started.intended_min_ticks >= NonZeroU32::new(1).unwrap());
    assert!(
        started
            .wake_conditions
            .contains(&WakeCondition::IntendedDurationReached)
    );
    assert_eq!(ended.sleeper, agent);
    assert_eq!(ended.place, VILLAGE_SQUARE);

    let started_summary = format!(
        "SleepEpisodeStarted place={} min={} max={} target={} modifier={} wake_conditions={:?}",
        started.place,
        started.intended_min_ticks,
        started.intended_max_ticks,
        started.target_recovery.value(),
        started.recovery_modifier.value(),
        started.wake_conditions
    );
    let ended_summary = format!(
        "SleepEpisodeEnded place={} ticks={}->{} reason={:?} recovery={} fatigue={}",
        ended.place,
        ended.start_tick.0,
        ended.end_tick.0,
        ended.end_reason,
        ended.accumulated_recovery.value(),
        ended.final_fatigue.value()
    );
    assert!(!started_summary.contains('\n'));
    assert!(!ended_summary.contains('\n'));
}
