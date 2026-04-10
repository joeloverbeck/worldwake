use worldwake_core::{
    BanditCamp, CauseRef, EventLog, EventTag, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
};
use worldwake_sim::{SystemError, SystemExecutionContext};

pub fn bandit_camp_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions: _active_actions,
        action_defs: _action_defs,
        politics_trace: _,
        perception_trace: _,
        tick,
        system_id: _system_id,
    } = ctx;

    let updates = collect_updates(world, tick)?;
    for update in updates {
        apply_update(world, event_log, tick, update)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingUpdate {
    MarkEmptySince {
        place: worldwake_core::EntityId,
        camp: BanditCamp,
    },
    ClearEmptySince {
        place: worldwake_core::EntityId,
        camp: BanditCamp,
    },
    Abandon {
        place: worldwake_core::EntityId,
    },
}

fn collect_updates(world: &World, tick: Tick) -> Result<Vec<PendingUpdate>, SystemError> {
    let mut updates = Vec::new();

    for (place, camp) in world.query_bandit_camp() {
        let grace_ticks = world
            .get_component_bandit_faction_policy(camp.faction)
            .map(|policy| u64::from(policy.abandonment_grace_ticks.get()))
            .ok_or_else(|| {
                SystemError::new(format!(
                    "bandit faction {} lacks BanditFactionPolicy for camp {}",
                    camp.faction, place
                ))
            })?;

        let occupied = world.members_of(camp.faction).into_iter().any(|member| {
            world.is_alive(member)
                && world.get_component_dead_at(member).is_none()
                && world.effective_place(member) == Some(place)
        });

        match (occupied, camp.empty_since_tick) {
            (true, Some(_)) => {
                let mut next = camp.clone();
                next.empty_since_tick = None;
                updates.push(PendingUpdate::ClearEmptySince { place, camp: next });
            }
            (false, None) => {
                let mut next = camp.clone();
                next.empty_since_tick = Some(tick);
                updates.push(PendingUpdate::MarkEmptySince { place, camp: next });
            }
            (false, Some(empty_since_tick))
                if tick.0.saturating_sub(empty_since_tick.0) >= grace_ticks =>
            {
                updates.push(PendingUpdate::Abandon { place });
            }
            (true, None) | (false, Some(_)) => {}
        }
    }

    Ok(updates)
}

fn apply_update(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
    update: PendingUpdate,
) -> Result<(), SystemError> {
    let (place, visibility) = match &update {
        PendingUpdate::MarkEmptySince { place, .. }
        | PendingUpdate::ClearEmptySince { place, .. } => (*place, VisibilitySpec::Hidden),
        PendingUpdate::Abandon { place } => (*place, VisibilitySpec::SamePlace),
    };

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        Some(place),
        visibility,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::System)
        .add_tag(EventTag::WorldMutation)
        .add_target(place);

    match update {
        PendingUpdate::MarkEmptySince { place, camp }
        | PendingUpdate::ClearEmptySince { place, camp } => txn
            .set_component_bandit_camp(place, camp)
            .map_err(|err| SystemError::new(err.to_string()))?,
        PendingUpdate::Abandon { place } => txn
            .clear_component_bandit_camp(place)
            .map_err(|err| SystemError::new(err.to_string()))?,
    }

    let _ = txn.commit(event_log);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::bandit_camp_system;
    use crate::dispatch_table;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        BanditCamp, BanditFactionPolicy, ComponentDelta, ComponentKind, Container, ControlSource,
        EventLog, EventTag, EventView, InTransitOnEdge, Permille, PrototypePlace, Seed, StateDelta,
        Tick, VisibilitySpec, World, WorldTxn, build_prototype_world, prototype_place_entity,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionInstance, ActionInstanceId, DeterministicRng,
        SystemExecutionContext, SystemId,
    };

    struct Harness {
        world: World,
        log: EventLog,
        rng: DeterministicRng,
        defs: ActionDefRegistry,
        active: BTreeMap<ActionInstanceId, ActionInstance>,
        faction: worldwake_core::EntityId,
        camp_place: worldwake_core::EntityId,
        other_place: worldwake_core::EntityId,
        member_a: worldwake_core::EntityId,
        member_b: worldwake_core::EntityId,
        supplies: worldwake_core::EntityId,
    }

    impl Harness {
        fn new(grace_ticks: u32) -> Self {
            let mut world = World::new(build_prototype_world()).unwrap();
            let camp_place = prototype_place_entity(PrototypePlace::BanditCamp);
            let other_place = prototype_place_entity(PrototypePlace::ForestPath);
            let (faction, member_a, member_b, supplies) = {
                let mut txn = new_txn(&mut world, 1);
                let faction = txn.create_faction("Forest Bandits").unwrap();
                let member_a = txn.create_agent("Rook", ControlSource::Ai).unwrap();
                let member_b = txn.create_agent("Mora", ControlSource::Ai).unwrap();
                for member in [member_a, member_b] {
                    txn.add_member(member, faction).unwrap();
                    txn.set_ground_location(member, camp_place).unwrap();
                }
                txn.set_component_bandit_faction_policy(
                    faction,
                    BanditFactionPolicy {
                        min_regroup_count: 2,
                        establishment_duration_ticks: NonZeroU32::new(1).unwrap(),
                        abandonment_grace_ticks: NonZeroU32::new(grace_ticks).unwrap(),
                        flee_wound_threshold: Permille::new(650).unwrap(),
                        rally_place: Some(other_place),
                    },
                )
                .unwrap();
                let supplies = txn
                    .create_container(Container {
                        capacity: worldwake_core::LoadUnits(3),
                        allowed_commodities: None,
                        allows_unique_items: false,
                        allows_nested_containers: false,
                    })
                    .unwrap();
                txn.set_ground_location(supplies, camp_place).unwrap();
                txn.set_owner(supplies, faction).unwrap();
                txn.set_component_bandit_camp(
                    camp_place,
                    BanditCamp {
                        faction,
                        supplies,
                        empty_since_tick: None,
                    },
                )
                .unwrap();
                txn.commit(&mut EventLog::new());
                (faction, member_a, member_b, supplies)
            };

            Self {
                world,
                log: EventLog::new(),
                rng: DeterministicRng::new(Seed([7; 32])),
                defs: ActionDefRegistry::new(),
                active: BTreeMap::new(),
                faction,
                camp_place,
                other_place,
                member_a,
                member_b,
                supplies,
            }
        }

        fn run(&mut self, tick: u64) {
            bandit_camp_system(SystemExecutionContext {
                world: &mut self.world,
                event_log: &mut self.log,
                rng: &mut self.rng,
                active_actions: &self.active,
                action_defs: &self.defs,
                politics_trace: None,
                perception_trace: None,
                tick: Tick(tick),
                system_id: SystemId::BanditCamp,
            })
            .unwrap();
        }

        fn camp(&self) -> Option<&BanditCamp> {
            self.world.get_component_bandit_camp(self.camp_place)
        }

        fn move_member(
            &mut self,
            member: worldwake_core::EntityId,
            place: worldwake_core::EntityId,
            tick: u64,
        ) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.set_ground_location(member, place).unwrap();
            txn.clear_component_in_transit_on_edge(member).unwrap();
            txn.commit(&mut self.log);
        }

        fn set_member_in_transit(&mut self, member: worldwake_core::EntityId, tick: u64) {
            let edge_id = self
                .world
                .topology()
                .unique_direct_edge(self.camp_place, self.other_place)
                .unwrap()
                .unwrap()
                .id();
            let mut txn = new_txn(&mut self.world, tick);
            txn.set_in_transit(member).unwrap();
            txn.set_component_in_transit_on_edge(
                member,
                InTransitOnEdge {
                    edge_id,
                    origin: self.camp_place,
                    destination: self.other_place,
                    departure_tick: Tick(tick),
                    arrival_tick: Tick(tick + 2),
                },
            )
            .unwrap();
            txn.commit(&mut self.log);
        }
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            worldwake_core::CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::Hidden,
            worldwake_core::WitnessData::default(),
        )
    }

    fn abandonment_record(log: &EventLog) -> &worldwake_core::EventRecord {
        log.events_by_tag(EventTag::WorldMutation)
            .iter()
            .rev()
            .find_map(|event_id| {
                let record = log.get(*event_id).unwrap();
                record
                    .state_deltas()
                    .iter()
                    .any(|delta| {
                        matches!(
                            delta,
                            StateDelta::Component(ComponentDelta::Removed {
                                component_kind: ComponentKind::BanditCamp,
                                ..
                            })
                        )
                    })
                    .then_some(record)
            })
            .unwrap()
    }

    #[test]
    fn empty_camp_persists_until_grace_period_expires_then_abandons() {
        let mut harness = Harness::new(2);
        harness.move_member(harness.member_a, harness.other_place, 2);
        harness.move_member(harness.member_b, harness.other_place, 2);

        harness.run(2);
        assert_eq!(
            harness.camp().unwrap().empty_since_tick,
            Some(Tick(2)),
            "first empty tick should arm the abandonment timer"
        );

        harness.run(3);
        assert!(
            harness.camp().is_some(),
            "camp should survive within grace period"
        );

        harness.run(4);
        assert!(
            harness.camp().is_none(),
            "camp should be abandoned once grace expires"
        );

        let record = abandonment_record(&harness.log);
        assert!(record.tags().contains(&EventTag::System));
        assert!(record.tags().contains(&EventTag::WorldMutation));
        assert_eq!(record.visibility(), VisibilitySpec::SamePlace);
    }

    #[test]
    fn returning_member_clears_empty_since_tick_and_resets_timer() {
        let mut harness = Harness::new(2);
        harness.move_member(harness.member_a, harness.other_place, 2);
        harness.move_member(harness.member_b, harness.other_place, 2);

        harness.run(2);
        assert_eq!(harness.camp().unwrap().empty_since_tick, Some(Tick(2)));

        harness.move_member(harness.member_a, harness.camp_place, 3);
        harness.run(3);
        assert_eq!(harness.camp().unwrap().empty_since_tick, None);

        harness.move_member(harness.member_a, harness.other_place, 4);
        harness.run(4);
        assert_eq!(harness.camp().unwrap().empty_since_tick, Some(Tick(4)));
        assert!(harness.camp().is_some());
    }

    #[test]
    fn dead_and_in_transit_members_do_not_count_as_present() {
        let mut harness = Harness::new(1);
        {
            let mut txn = new_txn(&mut harness.world, 2);
            txn.set_component_dead_at(
                harness.member_a,
                worldwake_core::DeadAt {
                    tick: Tick(2),
                    cause: worldwake_core::DeathCause::CombatWounds,
                },
            )
            .unwrap();
            txn.commit(&mut harness.log);
        }
        harness.set_member_in_transit(harness.member_b, 2);

        harness.run(2);
        assert_eq!(harness.camp().unwrap().empty_since_tick, Some(Tick(2)));

        harness.run(3);
        assert!(harness.camp().is_none());
    }

    #[test]
    fn abandonment_preserves_supplies_and_faction() {
        let mut harness = Harness::new(1);
        harness.move_member(harness.member_a, harness.other_place, 2);
        harness.move_member(harness.member_b, harness.other_place, 2);

        harness.run(2);
        harness.run(3);

        assert!(
            harness
                .world
                .get_component_bandit_camp(harness.camp_place)
                .is_none()
        );
        assert_eq!(
            harness.world.effective_place(harness.supplies),
            Some(harness.camp_place)
        );
        assert!(
            harness
                .world
                .get_component_container(harness.supplies)
                .is_some()
        );
        assert!(harness.world.is_alive(harness.faction));
    }

    #[test]
    fn dispatch_table_uses_bandit_camp_system_for_bandit_camp_slot() {
        let mut harness = Harness::new(1);
        harness.move_member(harness.member_a, harness.other_place, 2);
        harness.move_member(harness.member_b, harness.other_place, 2);

        let systems = dispatch_table();
        systems.get(SystemId::BanditCamp)(SystemExecutionContext {
            world: &mut harness.world,
            event_log: &mut harness.log,
            rng: &mut harness.rng,
            active_actions: &harness.active,
            action_defs: &harness.defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(2),
            system_id: SystemId::BanditCamp,
        })
        .unwrap();

        assert_eq!(harness.camp().unwrap().empty_since_tick, Some(Tick(2)));
    }
}
