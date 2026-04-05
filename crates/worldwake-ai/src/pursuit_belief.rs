//! Centralized pursuit-target belief extraction helper.
//!
//! Provides a single source of truth for "what does the agent believe about
//! where the target is?" — used by candidate generation, goal-model place
//! derivation, and invalidation.

use worldwake_core::{EntityId, PerceptionSource, Tick};
use worldwake_sim::GoalBeliefView;

/// Snapshot of an agent's belief about a pursuit target's remote location.
///
/// Contains only provenance fields (`source`, `observed_tick`).  Confidence
/// is always derived on demand via `belief_confidence()` — never stored here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PursuitTargetBelief {
    pub target: EntityId,
    pub believed_place: EntityId,
    pub source: PerceptionSource,
    pub observed_tick: Tick,
}

/// Extract a pursuit target's believed remote location from the actor's
/// belief state.
///
/// Returns `None` when:
/// - the target has no known place in the actor's beliefs
/// - the target is believed dead
/// - the target is believed co-located with the actor (not remote)
///
/// The caller is responsible for deriving confidence from the returned
/// provenance fields and checking it against `PursuitProfile` thresholds.
pub fn pursuit_target_belief(
    view: &dyn GoalBeliefView,
    actor: EntityId,
    target: EntityId,
) -> Option<PursuitTargetBelief> {
    let beliefs = view.known_entity_beliefs(actor);
    let (_, state) = beliefs.iter().find(|(id, _)| *id == target)?;

    if !state.alive {
        return None;
    }

    let believed_place = state.last_known_place?;

    let actor_place = view.effective_place(actor)?;
    if believed_place == actor_place {
        return None;
    }

    Some(PursuitTargetBelief {
        target,
        believed_place,
        source: state.source,
        observed_tick: state.observed_tick,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        BeliefConfidencePolicy, BelievedEntityState, CommodityConsumableProfile, CommodityKind,
        DemandObservation, DriveThresholds, EntityId, EntityKind, HomeostaticNeeds, LoadUnits,
        MerchandiseProfile, PerceptionSource, Quantity, ResourceSource, Tick, UniqueItemKind,
        Wound, WorkstationTag,
    };
    use worldwake_core::RecipeId;
    use worldwake_sim::GoalBeliefView;

    const ACTOR: EntityId = EntityId {
        slot: 1,
        generation: 0,
    };
    const TARGET: EntityId = EntityId {
        slot: 2,
        generation: 0,
    };
    const REMOTE_PLACE: EntityId = EntityId {
        slot: 10,
        generation: 0,
    };
    const ACTOR_PLACE: EntityId = EntityId {
        slot: 20,
        generation: 0,
    };
    const REPORTER: EntityId = EntityId {
        slot: 99,
        generation: 0,
    };

    fn alive_state(place: Option<EntityId>, observed: Tick) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: place,
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            observed_tick: observed,
            source: PerceptionSource::DirectObservation,
        }
    }

    fn dead_state(place: Option<EntityId>, observed: Tick) -> BelievedEntityState {
        let mut s = alive_state(place, observed);
        s.alive = false;
        s
    }

    /// Minimal `GoalBeliefView` stub for `pursuit_target_belief` tests.
    struct StubView {
        actor_place: Option<EntityId>,
        beliefs: Vec<(EntityId, BelievedEntityState)>,
    }

    impl GoalBeliefView for StubView {
        fn is_alive(&self, _entity: EntityId) -> bool {
            true
        }
        fn is_dead(&self, _entity: EntityId) -> bool {
            false
        }
        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            Some(EntityKind::Agent)
        }
        fn effective_place(&self, _entity: EntityId) -> Option<EntityId> {
            self.actor_place
        }
        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn known_entity_beliefs(
            &self,
            _agent: EntityId,
        ) -> Vec<(EntityId, BelievedEntityState)> {
            self.beliefs.clone()
        }
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
        }
        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }
        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }
        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
        }
        fn controlled_commodity_quantity_at_place(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }
        fn local_controlled_lots_for(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
            None
        }
        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            None
        }
        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }
        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }
        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }
        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }
        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }
        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }
        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }
        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            Vec::new()
        }
        fn hostile_targets_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn listed_sale_lots_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
        }
        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    #[test]
    fn returns_some_for_remote_alive_target() {
        let view = StubView {
            actor_place: Some(ACTOR_PLACE),
            beliefs: vec![(TARGET, alive_state(Some(REMOTE_PLACE), Tick(5)))],
        };

        let result = pursuit_target_belief(&view, ACTOR, TARGET);
        assert!(result.is_some());
        let ptb = result.unwrap();
        assert_eq!(ptb.target, TARGET);
        assert_eq!(ptb.believed_place, REMOTE_PLACE);
        assert_eq!(ptb.observed_tick, Tick(5));
        assert_eq!(ptb.source, PerceptionSource::DirectObservation);
    }

    #[test]
    fn returns_none_when_target_place_unknown() {
        let view = StubView {
            actor_place: Some(ACTOR_PLACE),
            beliefs: vec![(TARGET, alive_state(None, Tick(3)))],
        };

        assert!(pursuit_target_belief(&view, ACTOR, TARGET).is_none());
    }

    #[test]
    fn returns_none_when_target_believed_dead() {
        let view = StubView {
            actor_place: Some(ACTOR_PLACE),
            beliefs: vec![(TARGET, dead_state(Some(REMOTE_PLACE), Tick(3)))],
        };

        assert!(pursuit_target_belief(&view, ACTOR, TARGET).is_none());
    }

    #[test]
    fn returns_none_when_co_located() {
        let same_place = ACTOR_PLACE;

        let view = StubView {
            actor_place: Some(same_place),
            beliefs: vec![(TARGET, alive_state(Some(same_place), Tick(3)))],
        };

        assert!(pursuit_target_belief(&view, ACTOR, TARGET).is_none());
    }

    #[test]
    fn returns_none_when_target_not_in_beliefs() {
        let view = StubView {
            actor_place: Some(ACTOR_PLACE),
            beliefs: vec![],
        };

        assert!(pursuit_target_belief(&view, ACTOR, TARGET).is_none());
    }

    #[test]
    fn provenance_fields_match_underlying_belief() {
        let mut state = alive_state(Some(REMOTE_PLACE), Tick(42));
        state.source = PerceptionSource::Report {
            from: REPORTER,
            chain_len: 1,
        };

        let view = StubView {
            actor_place: Some(ACTOR_PLACE),
            beliefs: vec![(TARGET, state.clone())],
        };

        let ptb = pursuit_target_belief(&view, ACTOR, TARGET).unwrap();
        assert_eq!(ptb.source, state.source);
        assert_eq!(ptb.observed_tick, state.observed_tick);
    }
}
