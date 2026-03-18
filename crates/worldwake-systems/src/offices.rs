use std::collections::{BTreeMap, BTreeSet};

use worldwake_core::{
    CauseRef, EligibilityRule, EntityId, EntityKind, EventLog, EventTag, OfficeData, Permille,
    VisibilitySpec, WitnessData, World, WorldTxn,
};
use worldwake_sim::{SystemError, SystemExecutionContext};

const PUBLIC_ORDER_BASELINE: Permille = Permille::new_unchecked(750);
const VACANT_OFFICE_PENALTY: Permille = Permille::new_unchecked(200);
const HOSTILE_FACTION_PAIR_PENALTY: Permille = Permille::new_unchecked(100);

pub fn succession_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions: _active_actions,
        action_defs: _action_defs,
        tick,
        system_id: _system_id,
    } = ctx;

    let offices = world
        .query_office_data()
        .map(|(office, office_data)| (office, office_data.clone()))
        .collect::<Vec<_>>();

    for (office, office_data) in offices {
        if let Some(holder) = living_holder(world, office) {
            if office_data.vacancy_since.is_some() {
                let mut next = office_data.clone();
                next.vacancy_since = None;
                commit_hidden_office_update(world, event_log, tick, office, next)?;
            }
            let _ = holder;
            continue;
        }

        if office_data.vacancy_since.is_none() {
            let mut txn = new_political_txn(world, tick, Some(office_data.jurisdiction));
            let mut next = office_data.clone();
            next.vacancy_since = Some(tick);
            txn.set_component_office_data(office, next)
                .map_err(|error| SystemError::new(error.to_string()))?;
            txn.vacate_office(office)
                .map_err(|error| SystemError::new(error.to_string()))?;
            txn.add_target(office);
            let _ = txn.commit(event_log);
            continue;
        }

        let start_tick = office_data
            .vacancy_since
            .expect("vacancy_since checked above to be some");
        if tick.0.saturating_sub(start_tick.0) < office_data.succession_period_ticks {
            continue;
        }

        match office_data.succession_law {
            worldwake_core::SuccessionLaw::Support => {
                resolve_support_succession(world, event_log, tick, office, &office_data)?;
            }
            worldwake_core::SuccessionLaw::Force => {
                resolve_force_succession(world, event_log, tick, office, &office_data)?;
            }
        }
    }

    Ok(())
}

pub fn offices_with_jurisdiction(place: EntityId, world: &World) -> Vec<EntityId> {
    world
        .query_office_data()
        .filter_map(|(office, office_data)| (office_data.jurisdiction == place).then_some(office))
        .collect()
}

pub fn office_is_vacant(office: EntityId, world: &World) -> bool {
    world.entity_kind(office) == Some(EntityKind::Office) && living_holder(world, office).is_none()
}

pub fn public_order(place: EntityId, world: &World) -> Permille {
    let mut order = PUBLIC_ORDER_BASELINE;

    for office in offices_with_jurisdiction(place, world) {
        if office_is_vacant(office, world) {
            order = order.saturating_sub(VACANT_OFFICE_PENALTY);
        }
    }

    for _ in 0..count_present_hostile_faction_pairs_at(place, world) {
        order = order.saturating_sub(HOSTILE_FACTION_PAIR_PENALTY);
    }

    order
}

pub fn count_present_hostile_faction_pairs_at(place: EntityId, world: &World) -> usize {
    let present_factions = present_factions_at(place, world)
        .into_iter()
        .collect::<Vec<_>>();
    let mut count = 0;

    for (index, faction_a) in present_factions.iter().enumerate() {
        for faction_b in present_factions.iter().skip(index + 1) {
            if factions_are_hostile(*faction_a, *faction_b, world) {
                count += 1;
            }
        }
    }

    count
}

pub fn eligible_agents_at(office: EntityId, place: EntityId, world: &World) -> Vec<EntityId> {
    let Some(office_data) = world.get_component_office_data(office) else {
        return Vec::new();
    };

    world
        .entities_effectively_at(place)
        .into_iter()
        .filter(|entity| world.entity_kind(*entity) == Some(EntityKind::Agent))
        .filter(|entity| world.get_component_dead_at(*entity).is_none())
        .filter(|entity| candidate_is_eligible(world, office_data, *entity))
        .collect()
}

pub(crate) fn candidate_is_eligible(
    world: &World,
    office: &OfficeData,
    candidate: EntityId,
) -> bool {
    world.entity_kind(candidate) == Some(EntityKind::Agent)
        && world.get_component_dead_at(candidate).is_none()
        && office.eligibility_rules.iter().all(|rule| match rule {
            EligibilityRule::FactionMember(faction) => {
                world.factions_of(candidate).contains(faction)
            }
        })
}

fn resolve_support_succession(
    world: &mut World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    office: EntityId,
    office_data: &OfficeData,
) -> Result<(), SystemError> {
    let mut counts = BTreeMap::<EntityId, usize>::new();
    for (_, candidate) in world.support_declarations_for_office(office) {
        if candidate_is_eligible(world, office_data, candidate) {
            *counts.entry(candidate).or_default() += 1;
        }
    }

    let Some(max_support) = counts.values().copied().max() else {
        reset_vacancy_clock(world, event_log, tick, office, office_data)?;
        return Ok(());
    };

    let winners = counts
        .into_iter()
        .filter_map(|(candidate, support)| (support == max_support).then_some(candidate))
        .collect::<Vec<_>>();

    if winners.len() != 1 {
        reset_vacancy_clock(world, event_log, tick, office, office_data)?;
        return Ok(());
    }

    install_office_holder(world, event_log, tick, office, office_data, winners[0])
}

fn resolve_force_succession(
    world: &mut World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    office: EntityId,
    office_data: &OfficeData,
) -> Result<(), SystemError> {
    let contenders = eligible_agents_at(office, office_data.jurisdiction, world);
    if contenders.len() != 1 {
        return Ok(());
    }

    install_office_holder(world, event_log, tick, office, office_data, contenders[0])
}

fn install_office_holder(
    world: &mut World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    office: EntityId,
    office_data: &OfficeData,
    holder: EntityId,
) -> Result<(), SystemError> {
    let mut txn = new_political_txn(world, tick, Some(office_data.jurisdiction));
    let mut next = office_data.clone();
    next.vacancy_since = None;
    txn.set_component_office_data(office, next)
        .map_err(|error| SystemError::new(error.to_string()))?;
    txn.assign_office(office, holder)
        .map_err(|error| SystemError::new(error.to_string()))?;
    txn.clear_support_declarations_for_office(office)
        .map_err(|error| SystemError::new(error.to_string()))?;
    txn.add_target(office).add_target(holder);
    let _ = txn.commit(event_log);
    Ok(())
}

fn reset_vacancy_clock(
    world: &mut World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    office: EntityId,
    office_data: &OfficeData,
) -> Result<(), SystemError> {
    let mut next = office_data.clone();
    next.vacancy_since = Some(tick);
    commit_hidden_office_update(world, event_log, tick, office, next)
}

fn commit_hidden_office_update(
    world: &mut World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    office: EntityId,
    office_data: OfficeData,
) -> Result<(), SystemError> {
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::System)
        .add_tag(EventTag::WorldMutation)
        .add_tag(EventTag::Political)
        .add_target(office);
    txn.set_component_office_data(office, office_data)
        .map_err(|error| SystemError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    Ok(())
}

fn new_political_txn(
    world: &mut World,
    tick: worldwake_core::Tick,
    place_id: Option<EntityId>,
) -> WorldTxn<'_> {
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        place_id,
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::System)
        .add_tag(EventTag::WorldMutation)
        .add_tag(EventTag::Political);
    txn
}

fn living_holder(world: &World, office: EntityId) -> Option<EntityId> {
    let holder = world.office_holder(office)?;
    (world.get_component_dead_at(holder).is_none()).then_some(holder)
}

fn present_factions_at(place: EntityId, world: &World) -> BTreeSet<EntityId> {
    world
        .entities_effectively_at(place)
        .into_iter()
        .filter(|entity| world.entity_kind(*entity) == Some(EntityKind::Agent))
        .flat_map(|entity| world.factions_of(entity))
        .collect()
}

fn factions_are_hostile(faction_a: EntityId, faction_b: EntityId, world: &World) -> bool {
    world.hostile_targets_of(faction_a).contains(&faction_b)
        || world.hostile_targets_of(faction_b).contains(&faction_a)
}

#[cfg(test)]
mod tests {
    use super::{
        candidate_is_eligible, count_present_hostile_faction_pairs_at, eligible_agents_at,
        office_is_vacant, offices_with_jurisdiction, public_order, succession_system,
    };
    use crate::dispatch_table;
    use std::collections::BTreeMap;
    use worldwake_core::{
        build_prototype_world, CauseRef, ControlSource, EntityId, EventLog, EventTag, EventView,
        OfficeData, Permille, Seed, Tick, UtilityProfile, VisibilitySpec, WitnessData, World,
        WorldTxn,
    };
    use worldwake_sim::{ActionDefRegistry, DeterministicRng, SystemExecutionContext, SystemId};

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        )
    }

    fn run_succession(world: &mut World, event_log: &mut EventLog, tick: u64) {
        let mut rng = DeterministicRng::new(Seed([tick as u8; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();
        succession_system(SystemExecutionContext {
            world,
            event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(tick),
            system_id: SystemId::Politics,
        })
        .unwrap();
    }

    struct Fixture {
        world: World,
        place: EntityId,
        office: EntityId,
        holder: EntityId,
        candidate_a: EntityId,
        candidate_b: EntityId,
        faction: EntityId,
    }

    impl Fixture {
        fn new(law: worldwake_core::SuccessionLaw) -> Self {
            let mut world = World::new(build_prototype_world()).unwrap();
            let place = world.topology().place_ids().next().unwrap();
            let (office, faction, holder, candidate_a, candidate_b) = {
                let mut txn = new_txn(&mut world, 1);
                let office = txn.create_office("Ruler").unwrap();
                let faction = txn.create_faction("Ward").unwrap();
                let holder = txn.create_agent("Holder", ControlSource::Ai).unwrap();
                let candidate_a = txn.create_agent("Alice", ControlSource::Ai).unwrap();
                let candidate_b = txn.create_agent("Bob", ControlSource::Ai).unwrap();
                for entity in [holder, candidate_a, candidate_b] {
                    txn.set_ground_location(entity, place).unwrap();
                }
                txn.add_member(candidate_a, faction).unwrap();
                txn.add_member(candidate_b, faction).unwrap();
                txn.assign_office(office, holder).unwrap();
                txn.set_component_office_data(
                    office,
                    OfficeData {
                        title: "Ruler".to_string(),
                        jurisdiction: place,
                        succession_law: law,
                        eligibility_rules: vec![worldwake_core::EligibilityRule::FactionMember(
                            faction,
                        )],
                        succession_period_ticks: 3,
                        vacancy_since: None,
                    },
                )
                .unwrap();
                txn.set_component_utility_profile(holder, UtilityProfile::default())
                    .unwrap();
                txn.set_component_utility_profile(candidate_a, UtilityProfile::default())
                    .unwrap();
                txn.set_component_utility_profile(candidate_b, UtilityProfile::default())
                    .unwrap();
                let mut log = EventLog::new();
                let _ = txn.commit(&mut log);
                (office, faction, holder, candidate_a, candidate_b)
            };

            Self {
                world,
                place,
                office,
                holder,
                candidate_a,
                candidate_b,
                faction,
            }
        }

        fn kill_holder(&mut self, tick: u64) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.set_component_dead_at(self.holder, worldwake_core::DeadAt(Tick(tick)))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn declare_support(&mut self, supporter: EntityId, candidate: EntityId, tick: u64) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.declare_support(supporter, self.office, candidate)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
    }

    #[test]
    fn office_helpers_reflect_current_authoritative_state() {
        let fx = Fixture::new(worldwake_core::SuccessionLaw::Support);

        assert_eq!(
            offices_with_jurisdiction(fx.place, &fx.world),
            vec![fx.office]
        );
        assert!(!office_is_vacant(fx.office, &fx.world));
        assert!(candidate_is_eligible(
            &fx.world,
            fx.world.get_component_office_data(fx.office).unwrap(),
            fx.candidate_a
        ));
        assert_eq!(
            eligible_agents_at(fx.office, fx.place, &fx.world),
            vec![fx.candidate_a, fx.candidate_b]
        );
    }

    #[test]
    fn vacancy_activation_sets_vacancy_since_clears_relation_and_emits_visible_event() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 3);

        let office_data = fx.world.get_component_office_data(fx.office).unwrap();
        assert_eq!(office_data.vacancy_since, Some(Tick(3)));
        assert_eq!(fx.world.office_holder(fx.office), None);
        let record = event_log
            .get(event_log.events_by_tag(EventTag::Political)[0])
            .unwrap();
        assert_eq!(record.place_id(), Some(fx.place));
        assert_eq!(record.visibility(), VisibilitySpec::SamePlace);
        assert!(record.target_ids().contains(&fx.office));
    }

    #[test]
    fn living_holder_clears_stale_vacancy_since() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        {
            let mut txn = new_txn(&mut fx.world, 2);
            let mut office = txn.get_component_office_data(fx.office).cloned().unwrap();
            office.vacancy_since = Some(Tick(1));
            txn.set_component_office_data(fx.office, office).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 3);

        assert_eq!(
            fx.world
                .get_component_office_data(fx.office)
                .unwrap()
                .vacancy_since,
            None
        );
        let record = event_log
            .get(event_log.events_by_tag(EventTag::Political)[0])
            .unwrap();
        assert_eq!(record.visibility(), VisibilitySpec::Hidden);
    }

    #[test]
    fn support_succession_installs_unique_top_supported_candidate_and_clears_declarations() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.declare_support(fx.candidate_a, fx.candidate_a, 4);
        fx.declare_support(fx.candidate_b, fx.candidate_a, 4);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 6);

        assert_eq!(fx.world.office_holder(fx.office), Some(fx.candidate_a));
        assert_eq!(
            fx.world
                .get_component_office_data(fx.office)
                .unwrap()
                .vacancy_since,
            None
        );
        assert!(fx
            .world
            .support_declarations_for_office(fx.office)
            .is_empty());
        let record = event_log
            .get(event_log.events_by_tag(EventTag::Political)[0])
            .unwrap();
        assert_eq!(record.place_id(), Some(fx.place));
        assert!(record.target_ids().contains(&fx.candidate_a));
    }

    #[test]
    fn support_succession_ignores_ineligible_declarations_and_resets_timer_on_no_valid_votes() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let outsider = {
            let mut txn = new_txn(&mut fx.world, 2);
            let outsider = txn.create_agent("Outsider", ControlSource::Ai).unwrap();
            let supporter = txn.create_agent("Supporter", ControlSource::Ai).unwrap();
            txn.set_ground_location(outsider, fx.place).unwrap();
            txn.set_ground_location(supporter, fx.place).unwrap();
            txn.set_component_utility_profile(outsider, UtilityProfile::default())
                .unwrap();
            txn.set_component_utility_profile(supporter, UtilityProfile::default())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            txn = new_txn(&mut fx.world, 3);
            txn.declare_support(supporter, fx.office, outsider).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            outsider
        };
        let _ = outsider;
        fx.kill_holder(4);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 5);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 8);

        assert_eq!(fx.world.office_holder(fx.office), None);
        assert_eq!(
            fx.world
                .get_component_office_data(fx.office)
                .unwrap()
                .vacancy_since,
            Some(Tick(8))
        );
        let record = event_log
            .get(event_log.events_by_tag(EventTag::Political)[0])
            .unwrap();
        assert_eq!(record.visibility(), VisibilitySpec::Hidden);
    }

    #[test]
    fn support_tie_resets_vacancy_clock_without_installing_anyone() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.declare_support(fx.candidate_a, fx.candidate_a, 4);
        fx.declare_support(fx.candidate_b, fx.candidate_b, 4);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 6);

        assert_eq!(fx.world.office_holder(fx.office), None);
        assert_eq!(
            fx.world
                .get_component_office_data(fx.office)
                .unwrap()
                .vacancy_since,
            Some(Tick(6))
        );
    }

    #[test]
    fn force_succession_installs_only_uncontested_eligible_present_agent() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        fx.kill_holder(2);
        let other_place = fx
            .world
            .topology()
            .place_ids()
            .find(|place| *place != fx.place)
            .unwrap();
        {
            let mut txn = new_txn(&mut fx.world, 3);
            txn.set_ground_location(fx.candidate_b, other_place)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 6);

        assert_eq!(fx.world.office_holder(fx.office), Some(fx.candidate_a));
    }

    #[test]
    fn force_succession_blocks_when_multiple_contenders_are_present() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 6);

        assert_eq!(fx.world.office_holder(fx.office), None);
        assert!(event_log.events_by_tag(EventTag::Political).is_empty());
    }

    #[test]
    fn public_order_baseline_is_stable_when_place_has_no_vacancy_or_hostility() {
        let fx = Fixture::new(worldwake_core::SuccessionLaw::Support);

        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(750)
        );
    }

    #[test]
    fn public_order_subtracts_vacant_office_penalties() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let office_two = {
            let mut txn = new_txn(&mut fx.world, 2);
            let office = txn.create_office("Captain").unwrap();
            txn.set_component_office_data(
                office,
                OfficeData {
                    title: "Captain".to_string(),
                    jurisdiction: fx.place,
                    succession_law: worldwake_core::SuccessionLaw::Support,
                    eligibility_rules: vec![worldwake_core::EligibilityRule::FactionMember(
                        fx.faction,
                    )],
                    succession_period_ticks: 3,
                    vacancy_since: Some(Tick(2)),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            office
        };
        let _ = office_two;
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);

        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(350)
        );
    }

    #[test]
    fn hostile_faction_pairs_count_one_way_hostility_once() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let faction_b = {
            let mut txn = new_txn(&mut fx.world, 2);
            let faction_b = txn.create_faction("Rivals").unwrap();
            let rival = txn.create_agent("Rival", ControlSource::Ai).unwrap();
            txn.set_ground_location(rival, fx.place).unwrap();
            txn.add_member(rival, faction_b).unwrap();
            txn.add_hostility(fx.faction, faction_b).unwrap();
            txn.set_component_utility_profile(rival, UtilityProfile::default())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            faction_b
        };

        assert_eq!(
            count_present_hostile_faction_pairs_at(fx.place, &fx.world),
            1
        );
        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(650)
        );

        {
            let mut txn = new_txn(&mut fx.world, 3);
            txn.add_hostility(faction_b, fx.faction).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        assert_eq!(
            count_present_hostile_faction_pairs_at(fx.place, &fx.world),
            1
        );
        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(650)
        );
    }

    #[test]
    fn hostile_pair_count_ignores_duplicate_members_from_same_faction() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        {
            let mut txn = new_txn(&mut fx.world, 2);
            let faction_b = txn.create_faction("Rivals").unwrap();
            let rival_a = txn.create_agent("Rival A", ControlSource::Ai).unwrap();
            let rival_b = txn.create_agent("Rival B", ControlSource::Ai).unwrap();
            for rival in [rival_a, rival_b] {
                txn.set_ground_location(rival, fx.place).unwrap();
                txn.add_member(rival, faction_b).unwrap();
                txn.set_component_utility_profile(rival, UtilityProfile::default())
                    .unwrap();
            }
            txn.add_hostility(fx.faction, faction_b).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        assert_eq!(
            count_present_hostile_faction_pairs_at(fx.place, &fx.world),
            1
        );
    }

    #[test]
    fn public_order_combines_vacancy_and_hostility_and_saturates_at_zero() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let extra_places = fx
            .world
            .topology()
            .place_ids()
            .filter(|place| *place != fx.place)
            .take(3)
            .collect::<Vec<_>>();
        let extra_places_len = extra_places.len();
        assert_eq!(extra_places_len, 3);
        {
            let mut txn = new_txn(&mut fx.world, 2);
            for index in 0..4 {
                let office = txn.create_office(&format!("Vacant {index}")).unwrap();
                txn.set_component_office_data(
                    office,
                    OfficeData {
                        title: format!("Vacant {index}"),
                        jurisdiction: fx.place,
                        succession_law: worldwake_core::SuccessionLaw::Support,
                        eligibility_rules: vec![worldwake_core::EligibilityRule::FactionMember(
                            fx.faction,
                        )],
                        succession_period_ticks: 3,
                        vacancy_since: Some(Tick(2)),
                    },
                )
                .unwrap();
            }

            let faction_b = txn.create_faction("F2").unwrap();
            let faction_c = txn.create_faction("F3").unwrap();
            let faction_d = txn.create_faction("F4").unwrap();
            for (name, faction) in [("B", faction_b), ("C", faction_c), ("D", faction_d)] {
                let agent = txn.create_agent(name, ControlSource::Ai).unwrap();
                txn.set_ground_location(agent, fx.place).unwrap();
                txn.add_member(agent, faction).unwrap();
                txn.set_component_utility_profile(agent, UtilityProfile::default())
                    .unwrap();
            }
            txn.add_hostility(fx.faction, faction_b).unwrap();
            txn.add_hostility(fx.faction, faction_c).unwrap();
            txn.add_hostility(fx.faction, faction_d).unwrap();
            txn.add_hostility(faction_b, faction_c).unwrap();
            txn.add_hostility(faction_b, faction_d).unwrap();
            txn.add_hostility(faction_c, faction_d).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);

        assert_eq!(
            count_present_hostile_faction_pairs_at(fx.place, &fx.world),
            6
        );
        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(0)
        );
    }

    #[test]
    fn dispatch_table_runs_real_politics_system() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let systems = dispatch_table();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([3; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        systems.get(SystemId::Politics)(SystemExecutionContext {
            world: &mut fx.world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(3),
            system_id: SystemId::Politics,
        })
        .unwrap();

        assert_eq!(
            fx.world
                .get_component_office_data(fx.office)
                .unwrap()
                .vacancy_since,
            Some(Tick(3))
        );
    }
}
