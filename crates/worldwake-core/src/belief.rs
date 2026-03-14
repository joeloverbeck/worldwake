//! Authoritative belief and perception state for E14.

use crate::{
    CommodityKind, Component, EntityId, Permille, Quantity, ResourceSource, Tick, WorkstationTag,
    World, Wound,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Per-agent subjective view of observed entities and social evidence.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentBeliefStore {
    pub known_entities: BTreeMap<EntityId, BelievedEntityState>,
    pub social_observations: Vec<SocialObservation>,
}

impl AgentBeliefStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_entity(&mut self, id: EntityId, state: BelievedEntityState) {
        match self.known_entities.get(&id) {
            Some(existing) if existing.observed_tick > state.observed_tick => {}
            _ => {
                self.known_entities.insert(id, state);
            }
        }
    }

    #[must_use]
    pub fn get_entity(&self, id: &EntityId) -> Option<&BelievedEntityState> {
        self.known_entities.get(id)
    }

    pub fn record_social_observation(&mut self, observation: SocialObservation) {
        self.social_observations.push(observation);
    }

    pub fn enforce_capacity(&mut self, profile: &PerceptionProfile, current_tick: Tick) {
        self.known_entities.retain(|_, state| {
            within_retention_window(
                state.observed_tick,
                current_tick,
                profile.memory_retention_ticks,
            )
        });
        self.social_observations.retain(|observation| {
            within_retention_window(
                observation.observed_tick,
                current_tick,
                profile.memory_retention_ticks,
            )
        });

        if profile.memory_capacity == 0 {
            self.known_entities.clear();
            return;
        }

        let excess = self
            .known_entities
            .len()
            .saturating_sub(profile.memory_capacity as usize);
        if excess == 0 {
            return;
        }

        let mut eviction_order = self
            .known_entities
            .iter()
            .map(|(entity, state)| (state.observed_tick, *entity))
            .collect::<Vec<_>>();
        eviction_order.sort_unstable();

        for (_, entity) in eviction_order.into_iter().take(excess) {
            self.known_entities.remove(&entity);
        }
    }
}

impl Component for AgentBeliefStore {}

/// Snapshot of what an agent believes about a specific entity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedEntityState {
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub workstation_tag: Option<WorkstationTag>,
    pub resource_source: Option<ResourceSource>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
    pub observed_tick: Tick,
    pub source: PerceptionSource,
}

#[must_use]
pub fn build_believed_entity_state(
    world: &World,
    entity: EntityId,
    observed_tick: Tick,
    source: PerceptionSource,
) -> Option<BelievedEntityState> {
    world.entity_kind(entity)?;

    let mut inventory = BTreeMap::new();
    for commodity in CommodityKind::ALL {
        let quantity = world.controlled_commodity_quantity(entity, commodity);
        if quantity > Quantity(0) {
            inventory.insert(commodity, quantity);
        }
    }

    Some(BelievedEntityState {
        last_known_place: world.effective_place(entity),
        last_known_inventory: inventory,
        workstation_tag: world
            .get_component_workstation_marker(entity)
            .map(|marker| marker.0),
        resource_source: world.get_component_resource_source(entity).cloned(),
        alive: world.get_component_dead_at(entity).is_none(),
        wounds: world
            .get_component_wound_list(entity)
            .map(|wounds| wounds.wounds.clone())
            .unwrap_or_default(),
        observed_tick,
        source,
    })
}

/// How the agent acquired a belief snapshot.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PerceptionSource {
    DirectObservation,
    Report { from: EntityId, chain_len: u8 },
    Rumor { chain_len: u8 },
    Inference,
}

/// A witnessed social fact retained in belief memory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SocialObservation {
    pub kind: SocialObservationKind,
    pub subjects: (EntityId, EntityId),
    pub place: EntityId,
    pub observed_tick: Tick,
    pub source: PerceptionSource,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SocialObservationKind {
    WitnessedCooperation,
    WitnessedConflict,
    WitnessedObligation,
    WitnessedTelling,
    CoPresence,
}

/// Per-agent parameters controlling belief retention and observation quality.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PerceptionProfile {
    pub memory_capacity: u32,
    pub memory_retention_ticks: u64,
    pub observation_fidelity: Permille,
}

impl Component for PerceptionProfile {}

impl Default for PerceptionProfile {
    fn default() -> Self {
        Self {
            memory_capacity: 12,
            memory_retention_ticks: 48,
            observation_fidelity: Permille::new(875).unwrap(),
        }
    }
}

fn within_retention_window(observed_tick: Tick, current_tick: Tick, retention_ticks: u64) -> bool {
    current_tick.0.saturating_sub(observed_tick.0) <= retention_ticks
}

#[cfg(test)]
mod tests {
    use super::{
        build_believed_entity_state, AgentBeliefStore, BelievedEntityState, PerceptionProfile,
        PerceptionSource, SocialObservation, SocialObservationKind,
    };
    use crate::{
        build_prototype_world, traits::Component, BodyPart, CommodityKind, ControlSource, DeadAt,
        EntityId, Permille, Quantity, Tick, World, Wound, WoundCause, WoundId, WoundList,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeMap;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn profile(memory_capacity: u32, memory_retention_ticks: u64) -> PerceptionProfile {
        PerceptionProfile {
            memory_capacity,
            memory_retention_ticks,
            observation_fidelity: Permille::new(750).unwrap(),
        }
    }

    fn sample_wound(id: u64, observed_tick: u64) -> Wound {
        Wound {
            id: WoundId(id),
            body_part: BodyPart::Torso,
            cause: WoundCause::Combat {
                attacker: entity(99),
                weapon: crate::CombatWeaponRef::Unarmed,
            },
            severity: Permille::new(125).unwrap(),
            inflicted_at: Tick(observed_tick),
            bleed_rate_per_tick: Permille::new(10).unwrap(),
        }
    }

    fn sample_state(observed_tick: u64, commodity_qty: u32) -> BelievedEntityState {
        let mut inventory = BTreeMap::new();
        inventory.insert(CommodityKind::Apple, Quantity(commodity_qty));
        BelievedEntityState {
            last_known_place: Some(entity(10)),
            last_known_inventory: inventory,
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: vec![sample_wound(1, observed_tick)],
            observed_tick: Tick(observed_tick),
            source: PerceptionSource::DirectObservation,
        }
    }

    fn sample_social_observation(observed_tick: u64) -> SocialObservation {
        SocialObservation {
            kind: SocialObservationKind::WitnessedConflict,
            subjects: (entity(1), entity(2)),
            place: entity(10),
            observed_tick: Tick(observed_tick),
            source: PerceptionSource::DirectObservation,
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_serde_bounds<T: Eq + Clone + Serialize + DeserializeOwned>() {}

    #[test]
    fn new_creates_empty_store() {
        let store = AgentBeliefStore::new();

        assert!(store.known_entities.is_empty());
        assert!(store.social_observations.is_empty());
    }

    #[test]
    fn update_entity_inserts_new_snapshot() {
        let mut store = AgentBeliefStore::new();
        let target = entity(3);
        let state = sample_state(7, 4);

        store.update_entity(target, state.clone());

        assert_eq!(store.get_entity(&target), Some(&state));
    }

    #[test]
    fn update_entity_replaces_with_equal_or_newer_snapshot_only() {
        let mut store = AgentBeliefStore::new();
        let target = entity(4);

        store.update_entity(target, sample_state(8, 2));
        store.update_entity(target, sample_state(7, 9));
        assert_eq!(
            store.get_entity(&target).unwrap().last_known_inventory[&CommodityKind::Apple],
            Quantity(2)
        );

        store.update_entity(target, sample_state(8, 5));
        assert_eq!(
            store.get_entity(&target).unwrap().last_known_inventory[&CommodityKind::Apple],
            Quantity(5)
        );

        store.update_entity(target, sample_state(9, 6));
        assert_eq!(
            store.get_entity(&target).unwrap().last_known_inventory[&CommodityKind::Apple],
            Quantity(6)
        );
    }

    #[test]
    fn get_entity_returns_none_for_unknown_entity() {
        let store = AgentBeliefStore::new();

        assert_eq!(store.get_entity(&entity(404)), None);
    }

    #[test]
    fn record_social_observation_appends_to_list() {
        let mut store = AgentBeliefStore::new();
        let first = sample_social_observation(3);
        let second = SocialObservation {
            kind: SocialObservationKind::CoPresence,
            ..sample_social_observation(4)
        };

        store.record_social_observation(first.clone());
        store.record_social_observation(second.clone());

        assert_eq!(store.social_observations, vec![first, second]);
    }

    #[test]
    fn enforce_capacity_evicts_oldest_entities_deterministically() {
        let mut store = AgentBeliefStore::new();
        store.update_entity(entity(3), sample_state(5, 1));
        store.update_entity(entity(2), sample_state(5, 2));
        store.update_entity(entity(4), sample_state(6, 3));

        store.enforce_capacity(&profile(2, 100), Tick(20));

        assert_eq!(store.known_entities.len(), 2);
        assert!(!store.known_entities.contains_key(&entity(2)));
        assert!(store.known_entities.contains_key(&entity(3)));
        assert!(store.known_entities.contains_key(&entity(4)));
    }

    #[test]
    fn enforce_capacity_removes_stale_entities_and_social_observations() {
        let mut store = AgentBeliefStore::new();
        store.update_entity(entity(1), sample_state(2, 1));
        store.update_entity(entity(2), sample_state(9, 2));
        store.record_social_observation(sample_social_observation(3));
        store.record_social_observation(sample_social_observation(9));

        store.enforce_capacity(&profile(10, 3), Tick(12));

        assert!(!store.known_entities.contains_key(&entity(1)));
        assert!(store.known_entities.contains_key(&entity(2)));
        assert_eq!(
            store.social_observations,
            vec![sample_social_observation(9)]
        );
    }

    #[test]
    fn enforce_capacity_clears_entities_when_capacity_is_zero() {
        let mut store = AgentBeliefStore::new();
        store.update_entity(entity(1), sample_state(10, 1));
        store.update_entity(entity(2), sample_state(11, 2));

        store.enforce_capacity(&profile(0, 100), Tick(12));

        assert!(store.known_entities.is_empty());
    }

    #[test]
    fn believed_entity_state_roundtrips_through_bincode() {
        let state = sample_state(11, 7);

        let bytes = bincode::serialize(&state).unwrap();
        let roundtrip: BelievedEntityState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, state);
    }

    #[test]
    fn perception_source_roundtrips_and_compares() {
        let source = PerceptionSource::Report {
            from: entity(7),
            chain_len: 2,
        };

        let bytes = bincode::serialize(&source).unwrap();
        let roundtrip: PerceptionSource = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, source);
        assert_ne!(source, PerceptionSource::Inference);
    }

    #[test]
    fn social_observation_kind_roundtrips_and_compares() {
        let kind = SocialObservationKind::WitnessedTelling;

        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: SocialObservationKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, kind);
        assert_ne!(kind, SocialObservationKind::WitnessedConflict);
        assert_ne!(kind, SocialObservationKind::WitnessedCooperation);
    }

    #[test]
    fn perception_profile_roundtrips_through_bincode() {
        let profile = profile(12, 34);

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: PerceptionProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn belief_types_satisfy_component_and_serde_bounds() {
        assert_component_bounds::<AgentBeliefStore>();
        assert_component_bounds::<PerceptionProfile>();
        assert_serde_bounds::<BelievedEntityState>();
        assert_serde_bounds::<SocialObservation>();
    }

    #[test]
    fn build_believed_entity_state_projects_authoritative_snapshot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let holder = world
            .create_agent("Holder", ControlSource::Ai, Tick(1))
            .unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();
        let water = world
            .create_item_lot(CommodityKind::Water, Quantity(3), Tick(1))
            .unwrap();
        let wound = sample_wound(4, 2);

        world.set_ground_location(holder, place).unwrap();
        world.set_ground_location(bread, place).unwrap();
        world.set_ground_location(water, place).unwrap();
        world.set_possessor(bread, holder).unwrap();
        world.set_possessor(water, holder).unwrap();
        world
            .insert_component_wound_list(
                holder,
                WoundList {
                    wounds: vec![wound.clone()],
                },
            )
            .unwrap();

        let snapshot = build_believed_entity_state(
            &world,
            holder,
            Tick(9),
            PerceptionSource::Report {
                from: entity(8),
                chain_len: 2,
            },
        )
        .unwrap();

        assert_eq!(snapshot.last_known_place, Some(place));
        assert_eq!(
            snapshot.last_known_inventory,
            BTreeMap::from([
                (CommodityKind::Bread, Quantity(2)),
                (CommodityKind::Water, Quantity(3)),
            ])
        );
        assert!(snapshot.alive);
        assert_eq!(snapshot.wounds, vec![wound]);
        assert_eq!(snapshot.observed_tick, Tick(9));
        assert_eq!(
            snapshot.source,
            PerceptionSource::Report {
                from: entity(8),
                chain_len: 2,
            }
        );
    }

    #[test]
    fn build_believed_entity_state_handles_dead_or_missing_entities() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let dead = world
            .create_agent("Dead", ControlSource::Ai, Tick(1))
            .unwrap();

        world.set_ground_location(dead, place).unwrap();
        world
            .insert_component_dead_at(dead, DeadAt(Tick(5)))
            .unwrap();

        let dead_snapshot = build_believed_entity_state(
            &world,
            dead,
            Tick(7),
            PerceptionSource::Rumor { chain_len: 1 },
        )
        .unwrap();
        assert!(!dead_snapshot.alive);
        assert_eq!(dead_snapshot.last_known_place, Some(place));
        assert_eq!(
            dead_snapshot.source,
            PerceptionSource::Rumor { chain_len: 1 }
        );

        assert_eq!(
            build_believed_entity_state(
                &world,
                entity(999),
                Tick(7),
                PerceptionSource::DirectObservation,
            ),
            None
        );
    }
}
