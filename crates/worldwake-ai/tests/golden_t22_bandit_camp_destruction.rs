//! Golden T22 coverage for the bandit-camp destruction and diaspora chain.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{
    CommodityPurpose, DecisionOutcome, PlannerOpKind, SelectedPlanSource,
};
use worldwake_core::{
    build_believed_entity_state, hash_event_log, hash_world, ActionDomain, AgentData,
    BeliefConfidencePolicy, BelievedActivity, BanditCamp, BanditFactionPolicy, CombatProfile,
    CommodityKind, Container, ControlSource, EntityId, EventLog, GoalKey, GoalKind,
    HomeostaticNeeds, InstitutionalBeliefRead, KnownRecipes, MetabolismProfile, PerceptionProfile,
    PerceptionSource, Place, PlaceTag, Quantity, ResourceSource, Seed, Tick, Topology,
    TravelEdge, TravelEdgeId, UtilityProfile, World, WorkstationTag,
};
use worldwake_sim::{ActionTraceKind, ControllerState, Scheduler, SystemManifest};
use worldwake_sim::{
    ActionPayload, ActionRequestMode, CombatActionPayload, RequestProvenance,
};

const HOME_MARKET: EntityId = entity(100);
const BANDIT_ROAD: EntityId = entity(101);
const CAMP_PLACE: EntityId = entity(102);
const SAFE_FARM: EntityId = entity(103);
const RALLY_GLEN: EntityId = entity(104);

const fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn place(name: &str, tags: &[PlaceTag]) -> Place {
    Place {
        name: name.to_string(),
        capacity: None,
        tags: tags.iter().copied().collect(),
    }
}

fn connect(topology: &mut Topology, base_id: u32, from: EntityId, to: EntityId, ticks: u32) {
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(base_id), from, to, ticks, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(base_id + 1), to, from, ticks, None).unwrap())
        .unwrap();
}

fn build_t22_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(HOME_MARKET, place("Home Market", &[PlaceTag::Village, PlaceTag::Store]))
        .unwrap();
    topology
        .add_place(BANDIT_ROAD, place("Bandit Road", &[PlaceTag::Road, PlaceTag::Forest]))
        .unwrap();
    topology
        .add_place(CAMP_PLACE, place("Bandit Camp", &[PlaceTag::Camp, PlaceTag::Forest]))
        .unwrap();
    topology
        .add_place(SAFE_FARM, place("Safe Farm", &[PlaceTag::Farm, PlaceTag::Field]))
        .unwrap();
    topology
        .add_place(RALLY_GLEN, place("Rally Glen", &[PlaceTag::Forest, PlaceTag::Camp]))
        .unwrap();

    connect(&mut topology, 200, HOME_MARKET, BANDIT_ROAD, 1);
    connect(&mut topology, 210, BANDIT_ROAD, CAMP_PLACE, 1);
    connect(&mut topology, 220, HOME_MARKET, SAFE_FARM, 3);
    connect(&mut topology, 230, BANDIT_ROAD, RALLY_GLEN, 1);
    topology
}

fn default_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 64,
        memory_retention_ticks: 240,
        observation_fidelity: pm(875),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn bandit_profile() -> CombatProfile {
    CombatProfile::new(
        pm(700),
        pm(450),
        pm(220),
        pm(250),
        pm(50),
        pm(15),
        pm(0),
        pm(150),
        pm(40),
        nz(2),
        nz(4),
    )
}

fn guard_profile() -> CombatProfile {
    CombatProfile::new(
        pm(1000),
        pm(700),
        pm(900),
        pm(800),
        pm(120),
        pm(40),
        pm(0),
        pm(320),
        pm(120),
        nz(1),
        nz(4),
    )
}

fn bandit_utility_profile() -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(650),
        danger_weight: pm(900),
        courage: pm(150),
        enterprise_weight: pm(0),
        ..UtilityProfile::default()
    }
}

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

fn build_custom_harness(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    h.world = World::new(build_t22_topology()).unwrap();
    h.event_log = EventLog::new();
    h.scheduler = Scheduler::new(SystemManifest::canonical());
    h.controller = ControllerState::new();
    h
}

struct ScenarioIds {
    faction: EntityId,
    camp_supplies: EntityId,
    bandits: Vec<EntityId>,
    outsider: EntityId,
    guards: Vec<EntityId>,
    traveler_fresh: EntityId,
    traveler_late: EntityId,
}

fn seed_danger_belief(h: &mut GoldenHarness, traveler: EntityId, subject: EntityId, place: EntityId) {
    let mut belief =
        build_believed_entity_state(&h.world, subject, Tick(0), PerceptionSource::DirectObservation)
            .expect("should build belief for live combatant");
    belief.last_known_place = Some(place);
    belief.believed_activity = Some(BelievedActivity {
        action_domain: ActionDomain::Combat,
        target: None,
        observed_tick: Tick(0),
    });
    seed_belief(&mut h.world, &mut h.event_log, traveler, subject, belief);
}

fn seed_hungry_traveler(
    h: &mut GoldenHarness,
    name: &str,
    control: ControlSource,
) -> EntityId {
    let traveler = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        HOME_MARKET,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile {
            hunger_rate: pm(0),
            thirst_rate: pm(0),
            fatigue_rate: pm(0),
            bladder_rate: pm(0),
            dirtiness_rate: pm(0),
            ..MetabolismProfile::default()
        },
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    set_control_source(h, traveler, control, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        traveler,
        default_perception_profile(),
    );
    traveler
}

fn seed_scenario(h: &mut GoldenHarness) -> ScenarioIds {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        CAMP_PLACE,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(12),
            max_quantity: Quantity(12),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        SAFE_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(12),
            max_quantity: Quantity(12),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    let traveler_fresh = seed_hungry_traveler(h, "Fresh Traveler", ControlSource::Ai);
    let traveler_late = seed_hungry_traveler(h, "Late Traveler", ControlSource::None);

        let (
            faction,
            bandits,
            outsider,
            guards,
            camp_supplies,
            camp_apples,
            safe_apples,
        ) = {
            let mut txn = new_txn(&mut h.world, 0);
            let faction = txn.create_faction("Forest Bandits").unwrap();

        let mut bandits = Vec::new();
        for name in ["Rook", "Mora", "Tarn", "Sable"] {
            let bandit = txn.create_agent(name, ControlSource::Ai).unwrap();
            txn.add_member(bandit, faction).unwrap();
            txn.set_ground_location(bandit, CAMP_PLACE).unwrap();
            txn.set_component_perception_profile(bandit, default_perception_profile())
                .unwrap();
            txn.set_component_combat_profile(bandit, bandit_profile()).unwrap();
            txn.set_component_utility_profile(bandit, bandit_utility_profile())
                .unwrap();
            bandits.push(bandit);
        }
        txn.set_component_wound_list(bandits[0], stable_wound_list(350))
            .unwrap();
        txn.set_component_wound_list(bandits[1], stable_wound_list(350))
            .unwrap();

        let outsider = txn.create_agent("Stray", ControlSource::Ai).unwrap();
        txn.add_member(outsider, faction).unwrap();
        txn.set_ground_location(outsider, HOME_MARKET).unwrap();
        txn.set_component_perception_profile(outsider, default_perception_profile())
            .unwrap();
        txn.set_component_combat_profile(outsider, bandit_profile()).unwrap();
        txn.set_component_utility_profile(outsider, bandit_utility_profile())
            .unwrap();

        txn.set_component_bandit_faction_policy(
            faction,
            BanditFactionPolicy {
                min_regroup_count: 2,
                establishment_duration_ticks: nz(2),
                abandonment_grace_ticks: nz(2),
                flee_wound_threshold: pm(300),
                rally_place: Some(RALLY_GLEN),
            },
        )
        .unwrap();

        let camp_supplies = txn
            .create_container(Container {
                capacity: worldwake_core::LoadUnits(20),
                allowed_commodities: None,
                allows_unique_items: false,
                allows_nested_containers: false,
            })
            .unwrap();
        txn.set_ground_location(camp_supplies, CAMP_PLACE).unwrap();
        txn.set_owner(camp_supplies, faction).unwrap();
        txn.set_component_bandit_camp(
            CAMP_PLACE,
            BanditCamp {
                faction,
                supplies: camp_supplies,
                empty_since_tick: None,
            },
        )
        .unwrap();
        let bread = txn.create_item_lot(CommodityKind::Bread, Quantity(6)).unwrap();
        txn.set_ground_location(bread, CAMP_PLACE).unwrap();
        txn.set_owner(bread, faction).unwrap();
        txn.put_into_container(bread, camp_supplies).unwrap();
        let camp_apples = txn.create_item_lot(CommodityKind::Apple, Quantity(4)).unwrap();
        txn.set_ground_location(camp_apples, CAMP_PLACE).unwrap();
        let safe_apples = txn.create_item_lot(CommodityKind::Apple, Quantity(4)).unwrap();
        txn.set_ground_location(safe_apples, SAFE_FARM).unwrap();
        let rally_bread = txn.create_item_lot(CommodityKind::Bread, Quantity(2)).unwrap();
        txn.set_ground_location(rally_bread, RALLY_GLEN).unwrap();
        txn.set_owner(rally_bread, faction).unwrap();

        let mut guards = Vec::new();
        for name in ["Marshal", "Pike"] {
            let guard = txn.create_agent(name, ControlSource::None).unwrap();
            txn.set_ground_location(guard, CAMP_PLACE).unwrap();
            txn.set_component_perception_profile(guard, default_perception_profile())
                .unwrap();
            txn.set_component_combat_profile(guard, guard_profile()).unwrap();
            txn.set_component_utility_profile(
                guard,
                UtilityProfile {
                    danger_weight: pm(900),
                    social_weight: pm(0),
                    enterprise_weight: pm(0),
                    courage: pm(900),
                    ..UtilityProfile::default()
                },
            )
            .unwrap();
            guards.push(guard);
        }

        txn.commit(&mut h.event_log);
        (
            faction,
            bandits,
            outsider,
            guards,
            camp_supplies,
            camp_apples,
            safe_apples,
        )
    };

    for traveler in [traveler_fresh, traveler_late] {
        for lot in [camp_apples, safe_apples] {
            let belief = build_believed_entity_state(
                &h.world,
                lot,
                Tick(0),
                PerceptionSource::DirectObservation,
            )
            .expect("traveler food-lot belief should build");
            seed_belief(&mut h.world, &mut h.event_log, traveler, lot, belief);
        }
        seed_danger_belief(h, traveler, bandits[0], BANDIT_ROAD);
        seed_danger_belief(h, traveler, bandits[1], CAMP_PLACE);
    }

    ScenarioIds {
        faction,
        camp_supplies,
        bandits,
        outsider,
        guards,
        traveler_fresh,
        traveler_late,
    }
}

fn activate_attack(h: &mut GoldenHarness, ids: &ScenarioIds, tick: u64) {
    let attack_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "attack")
        .map(|def| def.id)
        .expect("full registries should include attack");
    for guard in &ids.guards {
        set_control_source(h, *guard, ControlSource::Ai, tick);
    }
    for (index, guard) in ids.guards.iter().enumerate() {
        for bandit in &ids.bandits {
            add_hostility(&mut h.world, &mut h.event_log, *guard, *bandit);
            add_hostility(&mut h.world, &mut h.event_log, *bandit, *guard);
        }
        let target = ids.bandits[index % ids.bandits.len()];
        let _ = h.scheduler.input_queue_mut().enqueue(
            Tick(tick),
            worldwake_sim::InputKind::RequestAction {
                actor: *guard,
                def_id: attack_def_id,
                targets: vec![target],
                payload_override: Some(ActionPayload::Combat(CombatActionPayload {
                    target,
                    weapon: worldwake_core::CombatWeaponRef::Unarmed,
                })),
                mode: ActionRequestMode::BestEffort,
                provenance: RequestProvenance::External,
            },
        );
    }
}

fn latest_traveler_selected_travel_destination(h: &GoldenHarness, agent: EntityId) -> Option<EntityId> {
    h.driver.trace_sink()?.traces_for(agent).into_iter().rev().find_map(|trace| {
        let DecisionOutcome::Planning(planning) = &trace.outcome else {
            return None;
        };
        if planning.selection.selected_plan_source != Some(SelectedPlanSource::SearchSelection) {
            return None;
        }
        if !planning
            .selection
            .selected_goal_is(GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            }))
        {
            return None;
        }
        planning
            .selection
            .selected_plan
            .as_ref()?
            .steps
            .iter()
            .rev()
            .find(|step| step.op_kind == PlannerOpKind::Travel)
            .and_then(|step| step.targets.first().copied())
    })
}

fn run_t22_scenario(seed: Seed) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = build_custom_harness(seed);
    let ids = seed_scenario(&mut h);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    h.step_once();

    let fresh_destination = latest_traveler_selected_travel_destination(&h, ids.traveler_fresh);
    if fresh_destination.is_none() {
        h.driver
            .trace_sink()
            .expect("decision tracing should stay enabled")
            .dump_agent(ids.traveler_fresh, &h.defs);
    }
    let fresh_trace_summaries = h
        .driver
        .trace_sink()
        .expect("decision tracing should stay enabled")
        .traces_for(ids.traveler_fresh)
        .into_iter()
        .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
        .collect::<Vec<_>>();
    assert_eq!(
        fresh_destination,
        Some(SAFE_FARM),
        "fresh danger beliefs should route the first traveler toward the safe farm; traces={fresh_trace_summaries:?}"
    );
    assert_eq!(
        h.world
            .get_component_agent_belief_store(ids.bandits[0])
            .expect("bandit should have a belief store")
            .believed_faction_rally_point(ids.faction),
        InstitutionalBeliefRead::Certain(Some(RALLY_GLEN)),
        "bandits co-located with the active camp should learn the rally doctrine before the attack"
    );
    assert_eq!(
        h.world
            .get_component_agent_belief_store(ids.outsider)
            .expect("outsider should have a belief store")
            .believed_faction_rally_point(ids.faction),
        InstitutionalBeliefRead::Unknown,
        "remote faction members should not learn rally doctrine without lawful camp observation"
    );

    activate_attack(&mut h, &ids, 1);

    let mut saw_bandit_departure = false;
    let mut saw_regroup_selection = false;
    let mut saw_regroup_travel = false;
    let mut saw_establish_goal = false;
    let mut old_camp_removed = false;
    let mut new_camp_established = false;
    let mut saw_establish_commit = false;

    for _ in 0..200 {
        h.step_once();
        saw_bandit_departure |= ids.bandits.iter().any(|bandit| {
            h.world.effective_place(*bandit).is_some_and(|place| place != CAMP_PLACE)
                || h.world.is_in_transit(*bandit)
        });
        saw_regroup_selection |= ids.bandits.iter().any(|bandit| {
            h.driver
                .trace_sink()
                .is_some_and(|sink| {
                    sink.traces_for(*bandit).into_iter().any(|trace| match &trace.outcome {
                        DecisionOutcome::Planning(planning) => planning
                            .selection
                            .selected_goal_is(GoalKey::from(GoalKind::RegroupWithFaction {
                                faction: ids.faction,
                            })),
                        _ => false,
                    })
                })
        });
        saw_regroup_travel |= ids.bandits.iter().any(|bandit| {
            h.action_trace_sink()
                .is_some_and(|sink| {
                    sink.events_for(*bandit).into_iter().any(|event| {
                        event.action_name == "travel"
                            && matches!(event.kind, ActionTraceKind::Committed { .. })
                    })
                })
        });
        saw_establish_goal |= ids.bandits.iter().any(|bandit| {
            h.driver.trace_sink().is_some_and(|sink| {
                sink.traces_for(*bandit).into_iter().any(|trace| match &trace.outcome {
                    DecisionOutcome::Planning(planning) => {
                        planning
                            .selection
                            .selected_goal_is(GoalKey::from(GoalKind::EstablishBanditCamp {
                                faction: ids.faction,
                            }))
                            || planning.candidates.generated.iter().any(|goal| {
                                goal.goal_key
                                    == GoalKey::from(GoalKind::EstablishBanditCamp {
                                        faction: ids.faction,
                                    })
                            })
                    }
                    _ => false,
                })
            })
        });
        old_camp_removed |= h.world.get_component_bandit_camp(CAMP_PLACE).is_none();
        new_camp_established |= h
            .world
            .get_component_bandit_camp(RALLY_GLEN)
            .is_some_and(|camp| camp.faction == ids.faction);
        saw_establish_commit |= ids.bandits.iter().any(|bandit| {
            h.action_trace_sink()
                .is_some_and(|sink| {
                    sink.events_for(*bandit).into_iter().any(|event| {
                        event.action_name == "establish_camp"
                            && matches!(event.kind, ActionTraceKind::Committed { .. })
                    })
                })
        });

        if saw_bandit_departure
            && saw_regroup_selection
            && saw_regroup_travel
            && saw_establish_goal
            && old_camp_removed
            && new_camp_established
            && saw_establish_commit
        {
            break;
        }
    }

    assert!(
        ids.bandits.iter().any(|bandit| {
            h.world.get_component_dead_at(*bandit).is_some()
                || h.world
                    .get_component_wound_list(*bandit)
                    .is_some_and(|wounds| !wounds.wounds.is_empty())
        }),
        "the external attack should leave at least one bandit dead or wounded"
    );
    assert!(
        saw_bandit_departure,
        "at least one surviving bandit should leave the original camp under attack pressure"
    );
    assert!(
        saw_regroup_selection,
        "a surviving bandit with rally doctrine should later select RegroupWithFaction after the camp starts breaking apart"
    );
    assert!(
        saw_regroup_travel,
        "regrouping should require ordinary travel rather than any teleport or direct camp transfer"
    );
    assert!(
        saw_establish_goal,
        "survivors at the rally should eventually generate or select EstablishBanditCamp once regrouping makes camp recreation lawful"
    );
    assert!(
        old_camp_removed,
        "the original camp should be removed through the abandonment path once no living faction members remain there"
    );
    assert!(
        new_camp_established,
        "surviving bandits should eventually establish a new camp at the rally place"
    );
    assert!(
        saw_establish_commit,
        "camp recreation should commit through the ordinary establish_camp action lifecycle"
    );
    assert_eq!(
        h.world.effective_place(ids.camp_supplies),
        Some(CAMP_PLACE),
        "abandonment should leave the old camp supplies behind as concrete aftermath"
    );
    assert!(
        h.driver
            .trace_sink()
            .expect("decision tracing should stay enabled")
            .traces_for(ids.outsider)
            .into_iter()
            .all(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => !planning
                    .selection
                    .selected_goal_is(GoalKey::from(GoalKind::RegroupWithFaction {
                        faction: ids.faction,
                    })),
                _ => true,
            }),
        "a same-faction outsider without rally doctrine should never select regroup"
    );

    let late_activation_tick = h.scheduler.current_tick().0;
    set_control_source(&mut h, ids.traveler_late, ControlSource::Ai, late_activation_tick);

    let mut late_destination = None;
    let mut late_trace_summaries = Vec::new();
    for _ in 0..12 {
        h.step_once();
        late_destination = latest_traveler_selected_travel_destination(&h, ids.traveler_late);
        if late_destination.is_some() {
            break;
        }
    }
    if late_destination.is_none() {
        late_trace_summaries = h
            .driver
            .trace_sink()
            .expect("decision tracing should stay enabled")
            .traces_for(ids.traveler_late)
            .into_iter()
            .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
            .collect::<Vec<_>>();
    }

    assert_eq!(
        late_destination,
        Some(CAMP_PLACE),
        "after the camp diaspora and normal belief aging, the later traveler should reselect the shorter abandoned-camp source; traces={late_trace_summaries:?}"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Scenario 22: Bandit Camp Destruction Chain
// ---------------------------------------------------------------------------
//
// Systems: Combat, Perception, Beliefs, AI, Travel, Production
// GoalKinds: ReduceDanger, RegroupWithFaction, AcquireCommodity(SelfConsume)
// ActionDomains: Combat, Generic, Production, Travel
// Places: HomeMarket, BanditRoad, BanditCamp, SafeFarm, RallyGlen
// Principles: 1, 4, 7, 12, 17, 25
//
// Setup: A custom five-place topology makes the danger-cost flip legible.
// Bandits occupy an active camp with faction policy and rally doctrine. Two
// otherwise identical hungry travelers hold the same source and threat
// beliefs. One plans immediately under fresh danger beliefs; one stays
// inactive until those same beliefs age to zero confidence. External guards
// attack the camp, survivors disperse, regroup, and re-establish camp at a
// rally glen.
//
// Proves:
// 1. Rally doctrine is acquired locally at the active camp and not by remote faction members.
// 2. External attack can break camp cohesion, produce abandonment, and leave concrete aftermath.
// 3. Survivors can later select RegroupWithFaction, travel, and re-establish camp elsewhere.
// 4. Fresh danger beliefs steer food acquisition toward the safe source, while stale beliefs later
//    reopen the shorter abandoned-camp route.
//
// Chain: active camp -> local rally belief -> external attack -> death / departure
// -> abandonment -> regroup travel -> establish camp -> stale danger decay
// -> downstream travel-route reversal.

#[test]
fn golden_t22_bandit_camp_destruction() {
    let _ = run_t22_scenario(Seed([222; 32]));
}

#[test]
fn golden_t22_bandit_camp_destruction_replays_deterministically() {
    let first = run_t22_scenario(Seed([222; 32]));
    let second = run_t22_scenario(Seed([222; 32]));
    assert_eq!(
        first, second,
        "T22 bandit camp destruction scenario should replay deterministically"
    );
}
