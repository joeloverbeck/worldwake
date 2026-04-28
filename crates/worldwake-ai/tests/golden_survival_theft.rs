//! Golden tests for the survival theft roadmap row.

mod golden_harness;

use std::collections::BTreeSet;
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKey, GoalKind};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    CommodityKind, DisturbanceKind, DriveThresholds, EntityId, EvidenceKind, PerceptionSource,
    Quantity, SocialObservationDetail, SocialObservationKind, TellTopic, Tick,
};
use worldwake_sim::{ActionTraceDetail, ActionTraceKind, TellBeliefDeltaKind, TellCommitResult};

const SURVIVAL_TICKS: u32 = 1440;
const STOLEN_COMMODITY: CommodityKind = CommodityKind::Apple;
const STAGED_APPLE_QUANTITY: Quantity = Quantity(60);

#[derive(Clone, Debug, Eq, PartialEq)]
struct ThiefSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TheftObservation {
    listing_tick: Tick,
    stage_tick: Tick,
    selected_theft_branch_tick: Tick,
    first_started_steal_tick: Tick,
    first_steal_tick: Tick,
    first_eat_tick: Tick,
    investigate_tick: Tick,
    social_transfer_tick: Tick,
    clerk_suspicion_tick: Tick,
    staged_lot: EntityId,
    merchant_saw_immediate_theft: bool,
    thief_commodity_after_steal: Quantity,
    scene_evidence: Vec<EvidenceKind>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalTheftObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    thief: ThiefSurvivalObservation,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    theft: TheftObservation,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-theft.ron")
}

fn load_survival_theft_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival theft scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival theft scenario should spawn");
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
    harness.enable_perception_tracing();
    (harness, def)
}

fn find_named_agent(h: &GoldenHarness, expected_name: &str) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find_map(|(entity, name, _)| (name.0 == expected_name).then_some(entity))
        .unwrap_or_else(|| panic!("scenario should include {expected_name}"))
}

fn merchant_has_listed_apple(h: &GoldenHarness, merchant: EntityId) -> Option<EntityId> {
    h.world.entities().find(|entity| {
        h.world
            .get_component_item_lot(*entity)
            .is_some_and(|lot| lot.commodity == STOLEN_COMMODITY && lot.quantity > Quantity(0))
            && h.world.get_component_sale_listing(*entity).is_some()
            && h.world.effective_place(*entity) == h.world.effective_place(merchant)
    })
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

fn run_survival_theft() -> SurvivalTheftObservation {
    let (mut h, def) = load_survival_theft_harness();
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "survival theft")
            .clone();
    let thief = find_named_agent(&h, "Thief Rana");
    let merchant = find_named_agent(&h, "Merchant Sera");
    let clerk = find_named_agent(&h, "Clerk Nia");
    let thief_thresholds = h
        .world
        .get_component_drive_thresholds(thief)
        .copied()
        .expect("thief should have drive thresholds");
    let mut thief_need_runs = SurvivalNeedRunTracker::default();
    let mut idle_state: (Option<u32>, u16, u32) = (None, 0, 0);
    let mut stuck_idle_windows = Vec::new();

    let mut listing_tick = None;
    let mut stage_tick = None;
    let mut staged_lot = None;
    let mut selected_theft_branch_tick = None;
    let mut first_started_steal_tick = None;
    let mut first_steal_tick = None;
    let mut first_eat_tick = None;
    let mut investigate_tick = None;
    let mut social_transfer_tick = None;
    let mut clerk_suspicion_tick = None;
    let mut merchant_saw_immediate_theft = false;
    let mut thief_commodity_after_steal = None;
    let mut scene_evidence = Vec::new();

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));

        let needs = h
            .world
            .get_component_homeostatic_needs(thief)
            .copied()
            .expect("thief should always have needs");
        thief_need_runs.observe(&needs, &thief_thresholds);

        if listing_tick.is_none()
            && let Some(lot) = merchant_has_listed_apple(&h, merchant)
        {
            listing_tick = Some(tick);
            staged_lot = Some(lot);
        }

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");
        let had_action =
            golden_harness::agent_has_non_failed_action_or_active(&h, action_sink, thief, tick);
        if had_action {
            if let Some(start_tick) = idle_state.0.take()
                && idle_state.2 >= contract.max_idle_window_ticks_with_elevated_need
                && idle_state.1 > contract.elevated_need_floor.value()
            {
                stuck_idle_windows.push(StuckIdleWindow {
                    agent_name: "Thief Rana".to_string(),
                    start_tick,
                    end_tick: tick_num.saturating_sub(1),
                    max_need_at_start: idle_state.1,
                });
            }
            idle_state.2 = 0;
        } else {
            if idle_state.0.is_none() {
                idle_state.0 = Some(tick_num);
                idle_state.1 = max_need_value(&needs);
            }
            idle_state.2 += 1;
        }

        for event in action_sink.events_for_at(merchant, tick) {
            if event.action_name == "stage_stock_for_sale"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && stage_tick.is_none()
            {
                stage_tick = Some(tick);
            }
            if event.action_name == "investigate"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && investigate_tick.is_none()
            {
                investigate_tick = Some(tick);
            }
            if event.action_name == "tell"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && social_transfer_tick.is_none()
            {
                let Some(ActionTraceDetail::Tell { listener, topic }) = &event.detail else {
                    continue;
                };
                if *listener != clerk {
                    continue;
                }
                let TellTopic::SocialObservation { observation } = topic else {
                    continue;
                };
                if !matches!(
                    observation.detail,
                    SocialObservationDetail::SuspectedTheft { .. }
                ) {
                    continue;
                }
                if event.tell_commit_result() != Some(TellCommitResult::Accepted)
                    || event.tell_belief_delta() != Some(TellBeliefDeltaKind::SocialObservation)
                {
                    continue;
                }
                social_transfer_tick = Some(tick);
            }
        }

        if selected_theft_branch_tick.is_none()
            && let Some(trace) = h
                .driver
                .trace_sink()
                .and_then(|sink| sink.trace_at(thief, tick))
            && let Some(staged_lot) = staged_lot
            && let DecisionOutcome::Planning(planning) = &trace.outcome
            && planning.selection.selected_goal()
                == Some(GoalKey::from(GoalKind::StealItem {
                    target_item: staged_lot,
                }))
        {
            selected_theft_branch_tick = Some(tick);
        }

        for event in action_sink.events_for_at(thief, tick) {
            if event.action_name == "steal"
                && matches!(event.kind, ActionTraceKind::Started { .. })
                && first_started_steal_tick.is_none()
            {
                first_started_steal_tick = Some(tick);
            }
            if event.action_name == "steal"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && first_steal_tick.is_none()
            {
                first_steal_tick = Some(tick);
                thief_commodity_after_steal = Some(
                    h.world
                        .controlled_commodity_quantity(thief, STOLEN_COMMODITY),
                );
                let merchant_store = h
                    .world
                    .get_component_agent_belief_store(merchant)
                    .expect("merchant should have a belief store");
                merchant_saw_immediate_theft =
                    merchant_store
                        .iter_social_observations()
                        .any(|observation| {
                            observation.kind() == SocialObservationKind::SuspectedTheft
                                && observation.observed_tick == tick
                        });
                let place = h
                    .world
                    .effective_place(merchant)
                    .expect("merchant should remain at the market");
                scene_evidence = h
                    .world
                    .get_component_scene_evidence(place)
                    .map(|scene| scene.evidence.iter().map(|entry| entry.kind).collect())
                    .unwrap_or_default();
            }
            if event.action_name == "eat"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && first_eat_tick.is_none()
            {
                first_eat_tick = Some(tick);
            }
        }

        if clerk_suspicion_tick.is_none()
            && h.world
                .get_component_agent_belief_store(clerk)
                .is_some_and(|store| {
                    store.iter_social_observations().any(|observation| {
                        matches!(
                            observation.detail,
                            SocialObservationDetail::SuspectedTheft { .. }
                        )
                    })
                })
        {
            clerk_suspicion_tick = Some(tick);
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let merchant_actions = action_sink
        .events_for(merchant)
        .iter()
        .map(|event| format!("{:?}: {}", event.tick, event.summary()))
        .collect::<Vec<_>>();
    let thief_actions = action_sink
        .events_for(thief)
        .iter()
        .map(|event| format!("{:?}: {}", event.tick, event.summary()))
        .collect::<Vec<_>>();
    let staged_lot = staged_lot.unwrap_or_else(|| {
        panic!(
            "merchant should expose a listed apple lot before theft; merchant_actions={merchant_actions:?}"
        )
    });

    SurvivalTheftObservation {
        contract,
        thief: ThiefSurvivalObservation {
            alive: !h.agent_is_dead(thief),
            critical_thresholds: thief_thresholds,
            critical_need_runs: thief_need_runs,
            committed_actions: action_sink
                .events_for(thief)
                .iter()
                .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
                .map(|event| event.action_name.clone())
                .collect(),
        },
        stuck_idle_windows,
        theft: TheftObservation {
            listing_tick: listing_tick.unwrap_or_else(|| {
                panic!(
                    "merchant should expose a listed apple lot before the theft branch can land; merchant_actions={merchant_actions:?}"
                )
            }),
            stage_tick: stage_tick.unwrap_or_else(|| {
                panic!("merchant should commit stage_stock_for_sale; merchant_actions={merchant_actions:?}")
            }),
            selected_theft_branch_tick: selected_theft_branch_tick.unwrap_or_else(|| {
                panic!(
                    "thief should select a StealItem branch against the staged lot; merchant_actions={merchant_actions:?}; thief_actions={thief_actions:?}"
                )
            }),
            first_started_steal_tick: first_started_steal_tick.unwrap_or_else(|| {
                panic!(
                    "thief should at least start steal against the staged lot; merchant_actions={merchant_actions:?}; thief_actions={thief_actions:?}"
                )
            }),
            first_steal_tick: first_steal_tick.unwrap_or_else(|| {
                panic!(
                    "thief should commit steal in the survival-theft scenario; merchant_actions={merchant_actions:?}; thief_actions={thief_actions:?}"
                )
            }),
            first_eat_tick: first_eat_tick.unwrap_or_else(|| {
                panic!("thief should later commit eat after stealing food; thief_actions={thief_actions:?}")
            }),
            investigate_tick: investigate_tick.unwrap_or_else(|| {
                panic!("merchant should investigate the missing displayed lot after theft; merchant_actions={merchant_actions:?}")
            }),
            social_transfer_tick: social_transfer_tick.unwrap_or_else(|| {
                panic!("merchant should tell Clerk Nia a SuspectedTheft social observation; merchant_actions={merchant_actions:?}")
            }),
            clerk_suspicion_tick: clerk_suspicion_tick.unwrap_or_else(|| {
                panic!("Clerk Nia should learn the SuspectedTheft observation from testimony")
            }),
            staged_lot,
            merchant_saw_immediate_theft,
            thief_commodity_after_steal: thief_commodity_after_steal
                .expect("steal commit should snapshot thief commodity state"),
            scene_evidence,
        },
    }
}

// ---------------------------------------------------------------------------
// Scenario 176: Survival Theft Lands the Concealed Staged-Lot Branch
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Trade, Perception, Transport
// GoalKinds: StealItem, ConsumeOwnedCommodity, SellCommodity, Drink, Wash, Sleep, Relieve
// ActionDomains: Trade, Needs, Transport
// Places: Shaded Market
// Principles: 4, 7, 8, 17, 20, 21
//
// Setup: Run the authored survival theft scenario for 1440 ticks. The merchant
// starts with apples plus private bread at a concealed market stall and can
// lawfully stage apples for sale. The thief starts hungry with no coin, no
// harvestable food source, and no remote food fallback, so the only local food
// branch is the merchant's displayed apple lot.
//
// Proves: the thief stays within the authored survival-health envelope; the
// merchant commits `stage_stock_for_sale`; the thief later selects a real
// `StealItem` branch against the staged apple lot and commits `steal`; the
// thief later commits `eat`; immediate direct witness pickup on the merchant is
// suppressed under the authored concealment/profile math; and the stolen
// display still leaves lawful physical scene evidence at the place. The same
// branch then matures into a local `investigate` commit, and Merchant Sera
// relays the resulting `SuspectedTheft` social observation to Clerk Nia, whose
// zero-fidelity perception profile prevents direct event pickup and makes the
// accepted testimony the proof surface for learning the theft suspicion.
//
// Chain: merchant stages displayed apples -> listed owned lot becomes visible
// local stock -> hungry thief selects `StealItem` against that lot -> committed
// `steal` clears the listing and moves the apple lot into the thief's
// possession -> thief later commits `eat` -> concealed same-place witness path
// stays quiet immediately, but the market keeps container/damage evidence ->
// merchant investigation records a theft suspicion -> accepted testimony
// transfers that social observation to the support listener.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_theft_proves_concealed_staged_lot_branch() {
    let observation = run_survival_theft();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());

    assert!(
        observation.thief.alive,
        "Thief Rana should remain alive for the full {SURVIVAL_TICKS}-tick scenario; observation={:?}",
        observation.thief
    );
    assert_authored_critical_runs_with_overrides(
        observation.contract.max_authored_critical_run_ticks,
        run_limit_overrides,
        "Thief Rana",
        &observation.thief.critical_thresholds,
        &observation.thief.critical_need_runs,
    );
    assert_required_self_care_families(
        &observation.contract.required_self_care_families,
        "Thief Rana",
        &observation.thief.committed_actions,
        "survival theft",
    );
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival theft",
        &observation.stuck_idle_windows,
    );

    assert!(
        observation.theft.stage_tick <= observation.theft.listing_tick,
        "stage_stock_for_sale should happen before or at listing visibility; theft={:?}",
        observation.theft
    );
    assert!(
        observation.theft.listing_tick <= observation.theft.selected_theft_branch_tick,
        "the displayed lot should exist before the theft branch is selected; theft={:?}",
        observation.theft
    );
    assert!(
        observation.theft.selected_theft_branch_tick <= observation.theft.first_started_steal_tick,
        "theft selection should precede steal start; theft={:?}",
        observation.theft
    );
    assert!(
        observation.theft.first_started_steal_tick <= observation.theft.first_steal_tick,
        "steal start should precede commit; theft={:?}",
        observation.theft
    );
    assert!(
        observation.theft.first_steal_tick < observation.theft.first_eat_tick,
        "the thief should eat only after the staged-lot theft lands; theft={:?}",
        observation.theft
    );
    assert!(
        observation.theft.first_steal_tick <= observation.theft.investigate_tick,
        "the merchant should investigate only after the theft commit; theft={:?}",
        observation.theft
    );
    assert!(
        observation.theft.investigate_tick <= observation.theft.social_transfer_tick,
        "the theft suspicion should be relayed after investigation; theft={:?}",
        observation.theft
    );
    assert!(
        observation.theft.social_transfer_tick <= observation.theft.clerk_suspicion_tick,
        "Clerk Nia's theft suspicion should follow the accepted testimony; theft={:?}",
        observation.theft
    );
    assert_eq!(
        observation.theft.thief_commodity_after_steal, STAGED_APPLE_QUANTITY,
        "the thief should hold the full staged apple lot immediately after steal commit; theft={:?}",
        observation.theft
    );
    assert!(
        !observation.theft.merchant_saw_immediate_theft,
        "concealment should suppress immediate direct theft pickup on the merchant at the theft tick; theft={:?}",
        observation.theft
    );
    assert!(
        observation.theft.scene_evidence.iter().any(|kind| matches!(
            kind,
            EvidenceKind::DisturbanceMarker {
                kind: DisturbanceKind::ForcedEntry,
                ..
            }
        )),
        "stealing a displayed lot should leave forced-entry disturbance evidence; theft={:?}",
        observation.theft
    );
    assert!(
        observation
            .theft
            .scene_evidence
            .iter()
            .any(|kind| matches!(kind, EvidenceKind::ContainerTampered { .. })),
        "stealing a displayed lot should leave container-tampering evidence; theft={:?}",
        observation.theft
    );
}

#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_theft_replays_deterministically() {
    assert_eq!(run_survival_theft(), run_survival_theft());
}
