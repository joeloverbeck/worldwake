//! Golden tests for S106 ground item decay.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    CommodityKind, EntityId, EventLog, EventTag, EventView, HomeostaticNeeds, MetabolismProfile,
    PrototypePlace, Seed, Tick, UtilityProfile, prototype_place_entity,
    verify_authoritative_conservation, verify_live_lot_conservation,
};

const FOREST_PATH: EntityId = prototype_place_entity(PrototypePlace::ForestPath);

fn decay_cycle_metabolism() -> MetabolismProfile {
    MetabolismProfile::new(
        pm(0),   // hunger_rate
        pm(0),   // thirst_rate
        pm(0),   // fatigue_rate
        pm(40),  // bladder_rate
        pm(0),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(2),   // toilet_ticks
        nz(12),  // wash_ticks
        nz(8),   // min_sleep_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(0),   // wilderness_relief_dirtiness_penalty
    )
}

fn live_waste_count(h: &GoldenHarness) -> usize {
    h.world
        .query_item_lot()
        .filter(|(_, lot)| lot.commodity == CommodityKind::Waste)
        .count()
}

fn tagged_events_through_tick(log: &EventLog, tag: EventTag, tick: Tick) -> usize {
    log.events_by_tag(tag)
        .iter()
        .filter(|event_id| {
            log.get(**event_id)
                .is_some_and(|record| record.tick() <= tick)
        })
        .count()
}

fn recent_wilderness_relief_events(log: &EventLog, tick: Tick, decay_window: u64) -> usize {
    let lower_bound = tick.0.saturating_sub(decay_window);
    log.events_by_tag(EventTag::WildernessRelief)
        .iter()
        .filter(|event_id| {
            log.get(**event_id).is_some_and(|record| {
                let event_tick = record.tick().0;
                lower_bound <= event_tick && event_tick <= tick.0
            })
        })
        .count()
}

// ---------------------------------------------------------------------------
// Scenario 342: Waste Decay Reaches A Bounded Steady State
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, ItemDecay
// GoalKinds: Relieve
// ActionDomains: Needs
// Places: ForestPath
// Principles: 4, 10, 11, 26
//
// Setup: Two AI agents start at ForestPath, an outdoor place where
//   `relieve_wilderness` is lawful. Hunger, thirst, and fatigue are inert;
//   only bladder rises, so the repeated self-care loop is local wilderness
//   relief rather than travel to a latrine. Waste decay is configured to 200
//   ticks and the run continues for 400 ticks.
//
// Proves: repeated Waste production through the live AI loop no longer grows
//   without bound. `ItemDecay` archives older Waste, emits the decay tags, and
//   preserves conservation via `wilderness_relief_events - item_decay_events ==
//   live_waste_lots` at authoritative checkpoints.
//
// Chain: bladder escalation -> GoalKind::Relieve -> committed
//   `relieve_wilderness` -> Waste lot on ground -> `ItemDecay` archives Waste
//   at the 200-tick threshold -> live Waste count stays bounded to the active
//   decay window instead of accumulating forever.

#[test]
#[ignore = "CI-only: long-running item-decay steady-state golden; run via golden-item-decay workflow"]
fn golden_waste_decay_reaches_steady_state() {
    let mut h = GoldenHarness::new(Seed([181; 32]));
    h.world
        .set_commodity_decay(std::collections::BTreeMap::from([(
            CommodityKind::Waste,
            nz(200),
        )]));

    let utility = UtilityProfile {
        bladder_weight: pm(1000),
        ..UtilityProfile::default()
    };

    for name in ["Reliever A", "Reliever B"] {
        let agent = seed_agent(
            &mut h.world,
            &mut h.event_log,
            name,
            FOREST_PATH,
            HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(700), pm(0)),
            decay_cycle_metabolism(),
            utility.clone(),
        );
        seed_actor_local_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            Tick(0),
            worldwake_core::PerceptionSource::DirectObservation,
        );
    }

    let mut max_live_waste = 0usize;
    for _ in 0..400 {
        h.step_once();

        let tick = h.scheduler.current_tick();
        let live_waste = live_waste_count(&h);
        max_live_waste = max_live_waste.max(live_waste);

        let created_waste =
            tagged_events_through_tick(&h.event_log, EventTag::WildernessRelief, tick);
        let decayed_waste = tagged_events_through_tick(&h.event_log, EventTag::ItemDecay, tick);
        let expected_live_waste = created_waste.saturating_sub(decayed_waste) as u64;

        if tick.0.is_multiple_of(25) {
            verify_live_lot_conservation(&h.world, CommodityKind::Waste, expected_live_waste)
                .unwrap_or_else(|err| {
                    panic!(
                        "live Waste conservation should hold at tick {}: {err}",
                        tick.0
                    )
                });
            verify_authoritative_conservation(&h.world, CommodityKind::Waste, expected_live_waste)
                .unwrap_or_else(|err| {
                    panic!(
                        "authoritative Waste conservation should hold at tick {}: {err}",
                        tick.0
                    )
                });
        }

        let recent_creations = recent_wilderness_relief_events(&h.event_log, tick, 200);
        assert!(
            live_waste <= recent_creations,
            "live Waste ({live_waste}) at tick {} should be bounded by Waste created in the active decay window ({recent_creations})",
            tick.0,
        );
    }

    let created_total = h.event_log.events_by_tag(EventTag::WildernessRelief).len();
    let decay_events = h.event_log.events_by_tag(EventTag::ItemDecay);

    assert!(
        created_total >= 8,
        "scenario should create repeated Waste before checking decay; created_total={created_total}"
    );
    assert!(
        !decay_events.is_empty(),
        "ItemDecay should archive at least one Waste lot during the 400 tick run"
    );
    assert!(
        live_waste_count(&h) < created_total,
        "final live Waste should be lower than total Waste created once decay is active; live={}, created_total={created_total}, max_live={max_live_waste}",
        live_waste_count(&h),
    );

    for event_id in decay_events {
        let record = h
            .event_log
            .get(*event_id)
            .expect("ItemDecay event ids should resolve");
        assert!(
            record.tags().contains(&EventTag::WorldMutation),
            "ItemDecay event {event_id:?} should also carry WorldMutation",
        );
    }
}
