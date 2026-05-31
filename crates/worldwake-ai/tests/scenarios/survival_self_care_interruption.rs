//! Golden coverage for S173 self-care interruption and occupancy contracts.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::golden_harness::*;
use worldwake_core::{
    AgentData, ClaimantOutcome, CommodityKind, ComponentKind, ComponentValue, ContentionGrant,
    ContentionIntents, ContentionPolicy, ContentionQueue, ContentionResolutionRule, ControlSource,
    DeathCause, DecisionEventPayload, DeprivationKind, EntityId, EventId, EventTag, EventView,
    GoalKey, GoalKind, HomeostaticNeedId, HomeostaticNeeds, MetabolismProfile, Quantity,
    QueuedContentionIntent, Seed, SelfCareUseKind, SleepFailureCause, StateDelta, Tick,
    UtilityProfile, WashBasinState, WorkstationTag, prototype_place_entity,
};
use worldwake_sim::{
    ActionRequestMode, ActionTraceDetail, ActionTraceKind, InputKind, RequestProvenance,
};

const PUBLIC_LATRINE: EntityId =
    prototype_place_entity(worldwake_core::PrototypePlace::PublicLatrine);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StartedAction {
    instance_id: worldwake_sim::ActionInstanceId,
    def_id: worldwake_core::ActionDefId,
}

fn action_id(h: &GoldenHarness, name: &str) -> worldwake_core::ActionDefId {
    h.defs.iter().find(|def| def.name == name).map_or_else(
        || panic!("full registries should include {name}"),
        |def| def.id,
    )
}

fn set_control_source(h: &mut GoldenHarness, agent: EntityId, control_source: ControlSource) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn quiet_self_care_metabolism() -> MetabolismProfile {
    MetabolismProfile {
        hunger_rate: pm(0),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        toilet_ticks: nz(4),
        wash_ticks: nz(4),
        min_sleep_ticks: nz(4),
        ..MetabolismProfile::default()
    }
}

fn self_care_utility() -> UtilityProfile {
    UtilityProfile {
        hunger_weight: pm(1000),
        thirst_weight: pm(1000),
        fatigue_weight: pm(1000),
        bladder_weight: pm(1000),
        dirtiness_weight: pm(1000),
        ..UtilityProfile::default()
    }
}

fn seed_human_agent(
    h: &mut GoldenHarness,
    name: &str,
    place: EntityId,
    needs: HomeostaticNeeds,
) -> EntityId {
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        needs,
        quiet_self_care_metabolism(),
        self_care_utility(),
    );
    set_control_source(h, agent, ControlSource::Human);
    agent
}

fn request_action(h: &mut GoldenHarness, actor: EntityId, def_name: &str, targets: Vec<EntityId>) {
    let def_id = action_id(h, def_name);
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

fn cancel_action(
    h: &mut GoldenHarness,
    actor: EntityId,
    instance_id: worldwake_sim::ActionInstanceId,
) {
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::CancelAction {
            actor,
            action_instance_id: instance_id,
        },
    );
}

fn active_action(h: &GoldenHarness, actor: EntityId, action_name: &str) -> StartedAction {
    h.scheduler
        .active_actions()
        .iter()
        .find_map(|(instance_id, instance)| {
            let def = h.defs.get(instance.def_id)?;
            (instance.actor == actor && def.name == action_name).then_some(StartedAction {
                instance_id: *instance_id,
                def_id: instance.def_id,
            })
        })
        .unwrap_or_else(|| panic!("{actor:?} should have active {action_name} action"))
}

fn maybe_active_action(
    h: &GoldenHarness,
    actor: EntityId,
    action_name: &str,
) -> Option<StartedAction> {
    h.scheduler
        .active_actions()
        .iter()
        .find_map(|(instance_id, instance)| {
            let def = h.defs.get(instance.def_id)?;
            (instance.actor == actor && def.name == action_name).then_some(StartedAction {
                instance_id: *instance_id,
                def_id: instance.def_id,
            })
        })
}

fn start_requested_action(
    h: &mut GoldenHarness,
    actor: EntityId,
    action_name: &str,
    targets: Vec<EntityId>,
) -> StartedAction {
    request_action(h, actor, action_name, targets);
    h.step_once();
    active_action(h, actor, action_name)
}

fn cancel_and_expect_detail(
    h: &mut GoldenHarness,
    actor: EntityId,
    started: StartedAction,
    action_name: &str,
    expected_detail: &ActionTraceDetail,
) {
    let aborted_before = h.event_log.events_by_tag(EventTag::ActionAborted).len();
    cancel_action(h, actor, started.instance_id);
    h.step_once();

    let event = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(actor)
        .into_iter()
        .find(|event| {
            event.def_id == started.def_id
                && event.action_name == action_name
                && matches!(
                    event.kind,
                    ActionTraceKind::Aborted {
                        instance_id,
                        ..
                    } if instance_id == started.instance_id
                )
        })
        .unwrap_or_else(|| panic!("{action_name} should record an abort trace"));
    assert_eq!(event.detail.as_ref(), Some(expected_detail));
    assert_eq!(
        h.event_log.events_by_tag(EventTag::ActionAborted).len(),
        aborted_before + 1,
        "{action_name} abort should emit one authoritative ActionAborted event"
    );
}

fn place_wash_basin(h: &mut GoldenHarness, place: EntityId) -> EntityId {
    let basin = place_workstation(
        &mut h.world,
        &mut h.event_log,
        place,
        WorkstationTag::WashBasin,
        ProductionOutputOwner::Actor,
    );
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_wash_basin_state(
        basin,
        WashBasinState {
            clean_water_units: 12,
            max_clean_water: 12,
            refill_per_tick: 0,
            units_per_full_wash: 3,
            dirtiness_level: pm(0),
            dirtiness_per_use: pm(40),
            max_effective_dirtiness: pm(1000),
            ..WashBasinState::default()
        },
    )
    .unwrap();
    txn.set_component_contention_policy(
        basin,
        ContentionPolicy {
            grant_hold_ticks: NonZeroU32::new(4).unwrap(),
            auto_promote: true,
            max_waiters: None,
        },
    )
    .unwrap();
    txn.set_component_contention_queue(basin, ContentionQueue::default())
        .unwrap();
    commit_txn(txn, &mut h.event_log);
    basin
}

fn set_self_care_queue(
    h: &mut GoldenHarness,
    facility: EntityId,
    waiters: &[(EntityId, Tick)],
    intended_action: &str,
    goal_key: GoalKey,
) {
    let action = action_id(h, intended_action);
    let mut queue = h
        .world
        .get_component_contention_queue(facility)
        .cloned()
        .unwrap_or_default();
    for (actor, queued_at) in waiters {
        queue.enqueue(*actor, action, *queued_at, None).unwrap();
    }

    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_contention_queue(facility, queue).unwrap();
    for (actor, _) in waiters {
        txn.set_component_contention_intents(
            *actor,
            ContentionIntents {
                intents: BTreeMap::from([(
                    facility,
                    QueuedContentionIntent {
                        goal_key,
                        intended_action: action,
                    },
                )]),
            },
        )
        .unwrap();
    }
    commit_txn(txn, &mut h.event_log);
}

fn contention_payloads(
    h: &GoldenHarness,
) -> Vec<(EventId, worldwake_core::ContentionEventPayload)> {
    h.event_log
        .events_by_tag(EventTag::ContentionResolved)
        .iter()
        .filter_map(|event_id| {
            h.event_log
                .get(*event_id)
                .and_then(EventView::contention_event_payload)
                .cloned()
                .map(|payload| (*event_id, payload))
        })
        .collect()
}

fn payload_for_facility(
    h: &GoldenHarness,
    facility: EntityId,
) -> (EventId, worldwake_core::ContentionEventPayload) {
    contention_payloads(h)
        .into_iter()
        .find(|(_, payload)| payload.contested_affordance.facility == facility)
        .unwrap_or_else(|| panic!("expected contention payload for {facility:?}"))
}

fn assert_occupancy_cleared_by_event(h: &GoldenHarness, target: EntityId) {
    assert!(
        h.event_log
            .events_by_tag(EventTag::ActionAborted)
            .iter()
            .any(|event_id| {
                h.event_log
                    .get(*event_id)
                    .is_some_and(|record| event_clears_self_care_occupancy(record, target))
            }),
        "ActionAborted event should include SelfCareOccupancy removal for {target:?}"
    );
}

fn event_clears_self_care_occupancy(record: &impl EventView, target: EntityId) -> bool {
    record.state_deltas().iter().any(|delta| {
        matches!(
            delta,
            StateDelta::Component(worldwake_core::ComponentDelta::Removed {
                entity,
                component_kind: ComponentKind::SelfCareOccupancy,
                before: ComponentValue::SelfCareOccupancy(_),
            }) if *entity == target
        )
    })
}

fn queue_grant_actor(h: &GoldenHarness, facility: EntityId) -> Option<EntityId> {
    h.world
        .get_component_contention_queue(facility)
        .and_then(|queue| queue.granted.as_ref().map(|grant| grant.actor))
}

fn collapse_metabolism() -> MetabolismProfile {
    MetabolismProfile {
        hunger_rate: pm(25),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        starvation_tolerance_ticks: nz(2),
        dehydration_tolerance_ticks: nz(1000),
        exhaustion_collapse_ticks: nz(1000),
        bladder_accident_tolerance_ticks: nz(1000),
        wash_ticks: nz(50),
        ..MetabolismProfile::default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InterruptionCollapseObservation {
    wash_interruptions: usize,
    death_tick: Tick,
    death_cause: DeathCause,
    max_hunger_critical_ticks: u32,
    starvation_wound_seen: bool,
    post_death_started_actions: Vec<String>,
}

fn run_repeated_interruption_deprivation_collapse(seed: Seed) -> InterruptionCollapseObservation {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();
    h.driver.enable_tracing();

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Repeatedly Interrupted Washer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(500), pm(0), pm(0), pm(0), pm(950)),
        collapse_metabolism(),
        UtilityProfile {
            hunger_weight: pm(100),
            thirst_weight: pm(0),
            fatigue_weight: pm(0),
            bladder_weight: pm(0),
            dirtiness_weight: pm(1000),
            enterprise_weight: pm(0),
            social_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    let basin = place_wash_basin(&mut h, VILLAGE_SQUARE);

    let mut interrupted_instances = Vec::new();
    let mut max_hunger_critical_ticks = 0;
    let mut starvation_wound_seen = false;

    loop {
        h.step_once();

        if let Some(exposure) = h.world.get_component_deprivation_exposure(agent) {
            max_hunger_critical_ticks =
                max_hunger_critical_ticks.max(exposure.hunger_critical_ticks);
        }
        if h.world
            .get_component_wound_list(agent)
            .is_some_and(|wounds| {
                wounds
                    .find_deprivation_wound(DeprivationKind::Starvation)
                    .is_some()
            })
        {
            starvation_wound_seen = true;
        }

        if let Some(dead_at) = h.world.get_component_dead_at(agent).copied() {
            for _ in 0..3 {
                h.step_once();
            }
            let post_death_started_actions = h
                .action_trace_sink()
                .expect("action tracing should be enabled")
                .events_for(agent)
                .into_iter()
                .filter(|event| {
                    event.tick.0 > dead_at.tick.0
                        && matches!(event.kind, ActionTraceKind::Started { .. })
                })
                .map(|event| event.action_name.clone())
                .collect();

            return InterruptionCollapseObservation {
                wash_interruptions: interrupted_instances.len(),
                death_tick: dead_at.tick,
                death_cause: dead_at.cause,
                max_hunger_critical_ticks,
                starvation_wound_seen,
                post_death_started_actions,
            };
        }

        if let Some(started) = maybe_active_action(&h, agent, "wash")
            && !interrupted_instances.contains(&started.instance_id)
        {
            let aborted_before = h.event_log.events_by_tag(EventTag::ActionAborted).len();
            cancel_action(&mut h, agent, started.instance_id);
            h.step_once();
            assert_eq!(
                h.event_log.events_by_tag(EventTag::ActionAborted).len(),
                aborted_before + 1,
                "cancelled wash should emit an authoritative ActionAborted event"
            );
            assert!(
                h.world.get_component_self_care_occupancy(basin).is_none(),
                "cancelled wash should release SelfCareOccupancy before the next retry"
            );
            interrupted_instances.push(started.instance_id);
        }

        assert!(
            h.scheduler.current_tick().0 < 160,
            "agent should die from hunger deprivation after repeated interrupted wash attempts; \
             interruptions={}, hunger={}, wound_load={}",
            interrupted_instances.len(),
            h.agent_hunger(agent).value(),
            h.agent_wound_load(agent),
        );
    }
}

// Scenario 478: S173 Self-Care Abort Traces Cover Every Family
//
// Setup: Human-controlled agents start each self-care action family and cancel before commit.
//
// Proves: Every self-care abort keeps the authoritative ActionAborted event and adds the typed action-trace discriminator; sleep uses the structured SleepInterrupted detail.
//
// Cross-system chain: external lawful request -> action start -> input cancellation -> handler abort cleanup -> EventTag::ActionAborted + action-trace detail.
#[test]
fn golden_self_care_abort_traces_cover_every_family() {
    let mut h = GoldenHarness::new(Seed([0x73; 32]));
    h.enable_action_tracing();

    let eater = seed_human_agent(
        &mut h,
        "Eater",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
    );
    let bread = give_commodity(
        &mut h.world,
        &mut h.event_log,
        eater,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );
    let started = start_requested_action(&mut h, eater, "eat", vec![bread]);
    cancel_and_expect_detail(
        &mut h,
        eater,
        started,
        "eat",
        &ActionTraceDetail::SelfCareInterrupted {
            kind: SelfCareUseKind::Eat,
            basin: None,
        },
    );
    assert_eq!(
        h.world.get_component_item_lot(bread).unwrap().quantity,
        Quantity(1),
        "aborted eat should not consume bread"
    );

    let drinker = seed_human_agent(
        &mut h,
        "Drinker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(900), pm(0), pm(0), pm(0)),
    );
    let apple = give_commodity(
        &mut h.world,
        &mut h.event_log,
        drinker,
        VILLAGE_SQUARE,
        CommodityKind::Apple,
        Quantity(1),
    );
    let started = start_requested_action(&mut h, drinker, "drink", vec![apple]);
    cancel_and_expect_detail(
        &mut h,
        drinker,
        started,
        "drink",
        &ActionTraceDetail::SelfCareInterrupted {
            kind: SelfCareUseKind::Drink,
            basin: None,
        },
    );
    assert_eq!(
        h.world.get_component_item_lot(apple).unwrap().quantity,
        Quantity(1),
        "aborted drink should not consume the thirst-relieving apple"
    );

    let sleeper = seed_human_agent(
        &mut h,
        "Sleeper",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(900), pm(0), pm(0)),
    );
    let started = start_requested_action(&mut h, sleeper, "sleep", vec![]);
    let sleep_aborted_before = h.event_log.events_by_tag(EventTag::ActionAborted).len();
    cancel_action(&mut h, sleeper, started.instance_id);
    h.step_once();
    assert_eq!(
        h.event_log.events_by_tag(EventTag::ActionAborted).len(),
        sleep_aborted_before + 1,
        "sleep abort should emit one authoritative ActionAborted event"
    );
    let sleep_payload = h
        .event_log
        .events_by_tag(EventTag::SleepEpisodeEnded)
        .iter()
        .find_map(|event_id| {
            h.event_log
                .get(*event_id)
                .and_then(|record| match record.decision_payload() {
                    Some(DecisionEventPayload::SleepEpisodeEnded(payload))
                        if payload.sleeper == sleeper =>
                    {
                        Some(payload)
                    }
                    _ => None,
                })
        })
        .expect("aborted sleep should end the SleepEpisode through the existing event surface");
    let sleep_abort = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(sleeper)
        .into_iter()
        .find(|event| {
            event.def_id == started.def_id
                && event.action_name == "sleep"
                && matches!(
                    event.kind,
                    ActionTraceKind::Aborted {
                        instance_id,
                        ..
                    } if instance_id == started.instance_id
                )
        })
        .expect("sleep should record an abort trace");
    assert_eq!(
        sleep_abort.detail.as_ref(),
        Some(&ActionTraceDetail::SleepInterrupted {
            place: sleep_payload.place,
            cause: SleepFailureCause::Generic,
            accumulated_recovery: sleep_payload.accumulated_recovery,
            was_rough_sleep: true,
        })
    );
    assert!(
        h.event_log
            .events_by_tag(EventTag::SleepEpisodeEnded)
            .iter()
            .any(|event_id| h.event_log.get(*event_id).is_some_and(|record| {
                matches!(
                    record.decision_payload(),
                    Some(DecisionEventPayload::SleepEpisodeEnded(payload))
                        if payload.sleeper == sleeper
                )
            })),
        "aborted sleep should end the SleepEpisode through the existing event surface"
    );

    let latrine_user = seed_human_agent(
        &mut h,
        "Latrine User",
        PUBLIC_LATRINE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(900), pm(0)),
    );
    let started = start_requested_action(&mut h, latrine_user, "toilet", vec![PUBLIC_LATRINE]);
    assert!(
        h.world
            .get_component_self_care_occupancy(PUBLIC_LATRINE)
            .is_some()
    );
    cancel_and_expect_detail(
        &mut h,
        latrine_user,
        started,
        "toilet",
        &ActionTraceDetail::SelfCareInterrupted {
            kind: SelfCareUseKind::LatrineRelief,
            basin: Some(PUBLIC_LATRINE),
        },
    );
    assert!(
        h.world
            .get_component_self_care_occupancy(PUBLIC_LATRINE)
            .is_none()
    );
    assert_occupancy_cleared_by_event(&h, PUBLIC_LATRINE);

    let wilderness_user = seed_human_agent(
        &mut h,
        "Wilderness User",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(900), pm(0)),
    );
    let started = start_requested_action(&mut h, wilderness_user, "relieve_wilderness", vec![]);
    cancel_and_expect_detail(
        &mut h,
        wilderness_user,
        started,
        "relieve_wilderness",
        &ActionTraceDetail::SelfCareInterrupted {
            kind: SelfCareUseKind::WildernessRelief,
            basin: None,
        },
    );

    let washer = seed_human_agent(
        &mut h,
        "Washer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(900)),
    );
    let basin = place_wash_basin(&mut h, VILLAGE_SQUARE);
    let started = start_requested_action(&mut h, washer, "wash", vec![basin]);
    assert!(h.world.get_component_self_care_occupancy(basin).is_some());
    cancel_and_expect_detail(
        &mut h,
        washer,
        started,
        "wash",
        &ActionTraceDetail::SelfCareInterrupted {
            kind: SelfCareUseKind::Wash,
            basin: Some(basin),
        },
    );
    assert!(h.world.get_component_self_care_occupancy(basin).is_none());
    assert_occupancy_cleared_by_event(&h, basin);
}

// Scenario 479: S173 Contested Wash Basin Promotes One Occupant
//
// Setup: Two dirty agents wait in the same wash-basin facility queue; the queue system grants the head claimant, and that claimant starts a real wash action.
//
// Proves: Self-care facility contention uses the same ContentionResolved/QueueGrantPromoted event surface as other exclusive facilities, and only the granted actor becomes the SelfCareOccupancy occupant.
//
// Cross-system chain: ContentionQueue.waiting -> contention_system promotion -> QueueGrantPromoted/ContentionResolved -> wash start -> SelfCareOccupancy.
#[test]
fn golden_self_care_contested_basin_promotes_one_occupant() {
    let mut h = GoldenHarness::new(Seed([0x74; 32]));
    h.enable_action_tracing();
    let basin = place_wash_basin(&mut h, VILLAGE_SQUARE);
    let agent_a = seed_human_agent(
        &mut h,
        "Washer A",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(900)),
    );
    let agent_b = seed_human_agent(
        &mut h,
        "Washer B",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(900)),
    );
    set_self_care_queue(
        &mut h,
        basin,
        &[(agent_a, Tick(10)), (agent_b, Tick(11))],
        "wash",
        GoalKey::from(GoalKind::Wash),
    );

    h.step_once();

    let (event_id, payload) = payload_for_facility(&h, basin);
    assert!(
        h.event_log
            .events_by_tag(EventTag::QueueGrantPromoted)
            .contains(&event_id),
        "self-care contention event should share the QueueGrantPromoted event"
    );
    assert_eq!(payload.contested_affordance.facility, basin);
    assert_eq!(payload.contested_affordance.action, action_id(&h, "wash"));
    assert_eq!(
        payload.resolution_rule,
        ContentionResolutionRule::ArrivalTime
    );
    assert_eq!(payload.total_claimants, 2);
    assert_eq!(payload.winner, Some(agent_a));
    assert_eq!(
        payload
            .claimants
            .iter()
            .map(|claimant| (
                claimant.agent,
                claimant.arrived_tick,
                claimant.queue_position,
                claimant.outcome
            ))
            .collect::<Vec<_>>(),
        vec![
            (agent_a, Tick(10), 1, ClaimantOutcome::Granted),
            (agent_b, Tick(11), 2, ClaimantOutcome::QueuedBehind),
        ]
    );

    request_action(&mut h, agent_a, "wash", vec![basin]);
    h.step_once();

    let occupancy = h
        .world
        .get_component_self_care_occupancy(basin)
        .expect("granted wash should write occupancy");
    assert_eq!(occupancy.occupant, agent_a);
    assert_eq!(occupancy.use_kind, SelfCareUseKind::Wash);
    assert_eq!(queue_grant_actor(&h, basin), Some(agent_a));
    assert!(
        h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(agent_b)
            .into_iter()
            .all(|event| {
                event.action_name != "wash"
                    || !matches!(event.kind, ActionTraceKind::Committed { .. })
            }),
        "non-granted actor must not commit wash while the basin is occupied"
    );
}

// Scenario 480: S173 Interrupted Wash Releases Basin And Promotes Waiter
//
// Setup: Agent A starts wash and writes occupancy; Agent B waits in the same basin queue. Cancelling A's wash releases occupancy, then the same tick's post-action system pass promotes B.
//
// Proves: Interrupted wash cleans up SelfCareOccupancy through the abort event and recovers through the ordinary queue-grant path, without planner-intent locks or silent reservations.
//
// Cross-system chain: wash start -> SelfCareOccupancy -> input cancellation -> ActionAborted with occupancy removal -> post-action contention_system promotion -> QueueGrantPromoted.
#[test]
fn golden_interrupted_wash_releases_basin_and_promotes_waiter() {
    let mut h = GoldenHarness::new(Seed([0x75; 32]));
    h.enable_action_tracing();
    let basin = place_wash_basin(&mut h, VILLAGE_SQUARE);
    let agent_a = seed_human_agent(
        &mut h,
        "Interrupted Washer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(900)),
    );
    let agent_b = seed_human_agent(
        &mut h,
        "Waiting Washer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(900)),
    );

    let started = start_requested_action(&mut h, agent_a, "wash", vec![basin]);
    let occupancy = h
        .world
        .get_component_self_care_occupancy(basin)
        .expect("wash start should write occupancy");
    assert_eq!(occupancy.occupant, agent_a);

    set_self_care_queue(
        &mut h,
        basin,
        &[(agent_b, Tick(20))],
        "wash",
        GoalKey::from(GoalKind::Wash),
    );
    cancel_and_expect_detail(
        &mut h,
        agent_a,
        started,
        "wash",
        &ActionTraceDetail::SelfCareInterrupted {
            kind: SelfCareUseKind::Wash,
            basin: Some(basin),
        },
    );
    assert!(h.world.get_component_self_care_occupancy(basin).is_none());
    assert_occupancy_cleared_by_event(&h, basin);
    assert_eq!(
        queue_grant_actor(&h, basin),
        Some(agent_b),
        "released basin should promote the waiting self-care claimant in the post-abort system pass"
    );

    let (event_id, payload) = payload_for_facility(&h, basin);
    assert!(
        h.event_log
            .events_by_tag(EventTag::QueueGrantPromoted)
            .contains(&event_id),
        "released basin should promote waiting self-care claimant on the next system tick"
    );
    assert_eq!(payload.contested_affordance.action, action_id(&h, "wash"));
    assert_eq!(payload.winner, Some(agent_b));
    assert_eq!(
        h.world
            .get_component_contention_queue(basin)
            .and_then(|queue| queue.granted.as_ref())
            .cloned(),
        Some(ContentionGrant {
            actor: agent_b,
            intended_action: action_id(&h, "wash"),
            granted_at: h.scheduler.current_tick() - 1,
            expires_at: h.scheduler.current_tick() - 1 + 4,
        })
    );
}

// Scenario 481: S173 Repeated Self-Care Interruption Can End In Deprivation Death
//
// Setup: An AI-controlled dirty agent repeatedly selects a local wash action.
//   The harness applies repeated external local cancellations before commit,
//   while hunger rises under the normal needs system and no food source is
//   available.
//
// Proves: repeated self-care aborts leave typed ActionAborted/ActionTraceDetail
//   evidence, release occupancy between retries, and do not rescue the agent
//   from the existing hunger-deprivation wound/death substrate.
//
// Cross-system chain: Wash candidate selection -> wash start -> controlled local
//   cancellation -> ActionAborted + SelfCareInterrupted + occupancy release ->
//   repeated retry under rising hunger -> starvation wound -> DeadAt/Death.
#[test]
#[ignore = "CI-only: repeated-interruption deprivation collapse; run via golden-survival workflow"]
fn golden_repeated_self_care_interruption_can_end_in_deprivation_death() {
    let observation = run_repeated_interruption_deprivation_collapse(Seed([0x76; 32]));

    assert!(
        observation.wash_interruptions >= 3,
        "scenario should prove repeated, not one-off, interruption; observation={observation:?}"
    );
    assert_eq!(
        observation.death_cause,
        DeathCause::NeedDeprivation {
            need: HomeostaticNeedId::Hunger
        },
        "live deprivation death substrate is hunger/thirst wound based; observation={observation:?}"
    );
    assert!(
        observation.starvation_wound_seen,
        "starvation wound should appear before death; observation={observation:?}"
    );
    assert!(
        observation.max_hunger_critical_ticks >= 1,
        "hunger critical exposure should accumulate before starvation wounds reset it; observation={observation:?}"
    );
    assert!(
        observation.post_death_started_actions.is_empty(),
        "dead agent should not start new actions after deprivation death; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: repeated-interruption deprivation collapse; run via golden-survival workflow"]
fn golden_repeated_self_care_interruption_collapse_replays_deterministically() {
    assert_eq!(
        run_repeated_interruption_deprivation_collapse(Seed([0x76; 32])),
        run_repeated_interruption_deprivation_collapse(Seed([0x76; 32])),
    );
}
