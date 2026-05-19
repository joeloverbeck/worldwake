//! Golden tests for the survival ask-consult roadmap landing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    DriveThresholds, EntityId, InstitutionalBeliefRead, LastSeenMemory, LastSeenProvenance,
    PerceptionSource, Tick,
};
use worldwake_sim::ActionTraceKind;

const SURVIVAL_TICKS: u32 = 1440;

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AskConsultObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agents: BTreeMap<String, AgentSurvivalObservation>,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    ask_tick: Tick,
    witness_return_tick: Tick,
    searcher_subject_belief_tick: Tick,
    consult_tick: Tick,
    declare_support_tick: Tick,
    installation_tick: Tick,
    claim_goal_generated: bool,
    final_office_belief: InstitutionalBeliefRead<Option<EntityId>>,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-ask-consult.ron")
}

fn load_survival_ask_consult_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival ask-consult scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival ask-consult scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    let agents = harness
        .world
        .query_name_and_agent_data()
        .map(|(agent, _, _)| agent)
        .collect::<Vec<_>>();
    for agent in agents {
        let place = harness
            .world
            .effective_place(agent)
            .expect("scenario agents should have a starting place");
        let local_entities = harness
            .world
            .entities()
            .filter(|entity| harness.world.effective_place(*entity) == Some(place))
            .filter(|entity| *entity != agent)
            .collect::<Vec<_>>();
        seed_actor_beliefs(
            &mut harness.world,
            &mut harness.event_log,
            agent,
            &local_entities,
            Tick(0),
            PerceptionSource::DirectObservation,
        );
    }
    harness.driver.enable_tracing();
    harness.enable_action_tracing();
    (harness, def)
}

fn named_entities(h: &GoldenHarness) -> BTreeMap<String, EntityId> {
    h.world
        .query_name()
        .map(|(entity, name)| (name.0.clone(), entity))
        .collect()
}

fn scenario_place_id(def: &ScenarioDef, place_name: &str) -> EntityId {
    let slot = def
        .places
        .iter()
        .position(|place| place.name == place_name)
        .and_then(|index| u32::try_from(index).ok())
        .expect("scenario place should exist within u32 slot bounds");
    EntityId {
        slot,
        generation: 0,
    }
}

fn contract_run_limit_overrides(
    limits: Option<&worldwake_cli::scenario::types::SurvivalCriticalRunLimitsDef>,
) -> SurvivalCriticalRunLimitOverrides {
    let Some(limits) = limits else {
        return SurvivalCriticalRunLimitOverrides::default();
    };

    SurvivalCriticalRunLimitOverrides {
        hunger: limits.hunger,
        thirst: limits.thirst,
        fatigue: limits.fatigue,
        bladder: limits.bladder,
        dirtiness: limits.dirtiness,
    }
}

fn searcher_subject_record(store: &LastSeenMemory, subject: EntityId) -> Option<(Tick, EntityId)> {
    let record = store.records.get(&subject).copied()?;
    match record.provenance {
        LastSeenProvenance::Hearsay {
            original_observer, ..
        } => Some((record.observed_tick, original_observer)),
        LastSeenProvenance::DirectObservation => None,
    }
}

fn run_survival_ask_consult() -> AskConsultObservation {
    let (mut h, def) = load_survival_ask_consult_harness();
    let contract = expect_survival_health_contract(
        def.survival_health_contract.as_ref(),
        "survival ask-consult",
    )
    .clone();
    let entities = named_entities(&h);
    let searcher = *entities
        .get("Searcher Rowan")
        .expect("scenario should include Searcher Rowan");
    let claimant = *entities
        .get("Claimant Ivo")
        .expect("scenario should include Claimant Ivo");
    let witness = *entities
        .get("Witness Mira")
        .expect("scenario should include Witness Mira");
    let subject = *entities
        .get("Forager Nia")
        .expect("scenario should include Forager Nia");
    let office = *entities
        .get("Commons Steward")
        .expect("scenario should include Commons Steward");
    let hall = scenario_place_id(&def, "Commons Hall");
    assert!(
        searcher_subject_record(
            h.world
                .get_component_last_seen_memory(searcher)
                .expect("searcher should have a last-seen memory"),
            subject
        )
        .is_none(),
        "searcher should start without a hearsay last-seen record for the subject"
    );
    assert_eq!(
        h.world
            .get_component_agent_belief_store(claimant)
            .expect("claimant should have a belief store")
            .believed_office_holder(office),
        InstitutionalBeliefRead::Unknown,
        "claimant should start with unknown office-holder belief before consult_record"
    );

    let tracked_agents = [searcher, witness];
    let tracked_names = ["Searcher Rowan", "Witness Mira"];
    let critical_thresholds = tracked_names
        .into_iter()
        .zip(tracked_agents)
        .map(|(name, agent)| {
            (
                name.to_string(),
                h.world
                    .get_component_drive_thresholds(agent)
                    .copied()
                    .expect("tracked agents should have drive thresholds"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut critical_need_runs = critical_thresholds
        .keys()
        .cloned()
        .map(|name| (name, SurvivalNeedRunTracker::default()))
        .collect::<BTreeMap<_, _>>();
    let mut idle_state: BTreeMap<String, (Option<u32>, u16, u32)> = critical_thresholds
        .keys()
        .cloned()
        .map(|name| (name, (None, 0, 0)))
        .collect();
    let mut stuck_idle_windows = Vec::new();

    let mut witness_return_tick = None;
    let mut searcher_subject_belief_tick = None;
    let mut ask_tick = None;
    let mut consult_tick = None;
    let mut declare_support_tick = None;
    let mut installation_tick = None;

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        for (agent_name, agent) in critical_thresholds.keys().zip(tracked_agents) {
            let needs = h
                .world
                .get_component_homeostatic_needs(agent)
                .expect("tracked agents should always have needs");
            critical_need_runs
                .get_mut(agent_name)
                .expect("run tracker should exist")
                .observe(
                    needs,
                    critical_thresholds
                        .get(agent_name)
                        .expect("threshold should exist for agent"),
                );

            let had_action = crate::golden_harness::agent_has_non_failed_action_or_active(
                &h,
                action_sink,
                agent,
                tick,
            );
            let (start, max_need, count) = idle_state
                .get_mut(agent_name)
                .expect("every tracked agent should have idle state");
            if had_action {
                if let Some(s) = start.take()
                    && *count >= contract.max_idle_window_ticks_with_elevated_need
                    && *max_need > contract.elevated_need_floor.value()
                {
                    stuck_idle_windows.push(StuckIdleWindow {
                        agent_name: agent_name.clone(),
                        start_tick: s,
                        end_tick: tick_num.saturating_sub(1),
                        max_need_at_start: *max_need,
                    });
                }
                *count = 0;
            } else {
                if start.is_none() {
                    *start = Some(tick_num);
                    *max_need = max_need_value(needs);
                }
                *count += 1;
            }
        }

        if witness_return_tick.is_none() && h.world.effective_place(witness) == Some(hall) {
            witness_return_tick = Some(tick);
        }
        if installation_tick.is_none() && h.world.office_holder(office) == Some(claimant) {
            installation_tick = Some(tick);
        }
        if searcher_subject_belief_tick.is_none()
            && searcher_subject_record(
                h.world
                    .get_component_last_seen_memory(searcher)
                    .expect("searcher should have a last-seen memory"),
                subject,
            )
            .is_some_and(|(_, original_observer)| original_observer == witness)
        {
            searcher_subject_belief_tick = Some(tick);
        }

        for event in action_sink.events_for_at(searcher, tick) {
            if event.action_name == "ask_about_person"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && ask_tick.is_none()
            {
                ask_tick = Some(tick);
            }
        }
        for event in action_sink.events_for_at(claimant, tick) {
            if event.action_name == "consult_record"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && consult_tick.is_none()
            {
                consult_tick = Some(tick);
            }
            if event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && declare_support_tick.is_none()
            {
                declare_support_tick = Some(tick);
            }
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should remain enabled");
    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should remain enabled");
    let agents = critical_thresholds
        .into_iter()
        .zip(tracked_agents)
        .map(|((name, thresholds), agent)| {
            (
                name.clone(),
                AgentSurvivalObservation {
                    alive: !h.agent_is_dead(agent),
                    critical_thresholds: thresholds,
                    critical_need_runs: critical_need_runs
                        .remove(&name)
                        .expect("final run tracker should exist"),
                    committed_actions: action_sink
                        .events_for(agent)
                        .iter()
                        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
                        .map(|event| event.action_name.clone())
                        .collect(),
                },
            )
        })
        .collect();

    AskConsultObservation {
        contract,
        agents,
        stuck_idle_windows,
        ask_tick: ask_tick.expect("scenario should commit ask_about_person"),
        witness_return_tick: witness_return_tick.expect("witness should return to the hall"),
        searcher_subject_belief_tick: searcher_subject_belief_tick
            .expect("searcher should acquire a hearsay last-seen record"),
        consult_tick: consult_tick.unwrap_or_else(|| {
            panic!(
                "scenario should commit consult_record; claim_goal_generated={} final_office_belief={:?} declare_support_tick={declare_support_tick:?} installation_tick={installation_tick:?}",
                decision_sink
                    .goal_history_for(
                        claimant,
                        &worldwake_core::GoalKind::ClaimOffice { office },
                    )
                    .into_iter()
                    .any(|entry| entry.status.is_generated()),
                h.world
                    .get_component_agent_belief_store(claimant)
                    .expect("claimant should have a belief store")
                    .believed_office_holder(office),
            )
        }),
        declare_support_tick: declare_support_tick
            .expect("scenario should commit declare_support"),
        installation_tick: installation_tick.expect("claimant should be installed as office holder"),
        claim_goal_generated: decision_sink
            .goal_history_for(claimant, &worldwake_core::GoalKind::ClaimOffice { office })
            .into_iter()
            .any(|entry| entry.status.is_generated()),
        final_office_belief: h
            .world
            .get_component_agent_belief_store(claimant)
            .expect("claimant should have a belief store")
            .believed_office_holder(office),
    }
}

// ---------------------------------------------------------------------------
// Scenario 170: Survival Ask-Consult Lands Roadmap Row Six
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Search, Epistemic, Offices
// GoalKinds: SearchForMissing, ClaimOffice, AcquireCommodity(SelfConsume), ConsumeOwnedCommodity,
//   Wash, Sleep, Relieve
// ActionDomains: Needs, Travel, Epistemic, Social
// Places: Commons Hall, North Orchard
// Principles: 6, 7, 14, 15, 20
//
// Setup: Run the authored survival ask-consult scenario for 1440 ticks. The
// witness begins beside the missing subject at the orchard, the searcher starts
// at the hall with an overdue expectation, and the claimant starts at the same
// hall with a visible vacant support-law office plus its unconsulted register.
//
// Proves: the tracked survival agents satisfy the authored survival-health
// contract; the searcher only gains the orchard lead after a committed
// ask_about_person; and the claimant only reaches declare_support through a
// committed consult_record before office installation.
//
// Chain: overdue expectation -> witness returns under survival pressure ->
// committed ask_about_person relays hearsay last-seen -> searcher belief store
// gains the orchard lead; unknown office-holder belief + local OfficeRegister ->
// consult_record commits -> declare_support commits -> succession installs the
// claimant.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_ask_consult_lands_row_six() {
    let observation = run_survival_ask_consult();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());

    for (agent_name, agent) in &observation.agents {
        assert!(
            agent.alive,
            "{agent_name} should remain alive for the full {SURVIVAL_TICKS}-tick scenario; observation={agent:?}"
        );
        assert_authored_critical_runs_with_overrides(
            observation.contract.max_authored_critical_run_ticks,
            run_limit_overrides,
            agent_name,
            &agent.critical_thresholds,
            &agent.critical_need_runs,
        );
        assert_required_self_care_families(
            &observation.contract.required_self_care_families,
            agent_name,
            &agent.committed_actions,
            "survival-ask-consult",
        );
    }
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival-ask-consult",
        &observation.stuck_idle_windows,
    );
    assert!(
        observation.witness_return_tick <= observation.ask_tick,
        "the witness should be back at the hall before ask_about_person commits; observation={observation:?}"
    );
    assert!(
        observation.ask_tick <= observation.searcher_subject_belief_tick,
        "the searcher's hearsay last-seen record must not predate ask_about_person; observation={observation:?}"
    );
    assert!(
        observation.consult_tick <= observation.declare_support_tick,
        "declare_support must not commit before consult_record; observation={observation:?}"
    );
    assert!(
        observation.declare_support_tick <= observation.installation_tick,
        "office installation must follow declare_support; observation={observation:?}"
    );
}
