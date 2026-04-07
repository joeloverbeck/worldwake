//! Cheap feasibility pre-check for goal candidates.
//!
//! Estimates whether a goal is locally actionable before committing full GOAP
//! search budget. Uses only `GoalBeliefView`, `BlockedIntentMemory`, and
//! `IntentionFrame` — never authoritative world state.

use serde::{Deserialize, Serialize};
use worldwake_core::{
    BlockedIntentMemory, CommodityKind, EntityId, FrameState, GoalKind, IntentionFrame, Quantity,
    Tick,
};
use worldwake_sim::GoalBeliefView;

use crate::{
    FeasibilityStrategy, GoalDispatchKey, enterprise::merchant_home_place, goal_model::RankedGoal,
};

/// Cheap pre-GOAP estimate of whether a goal is locally actionable.
/// Used to reorder candidates within the same `GoalPriorityClass` —
/// never to exclude goals from search.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum FeasibilityHint {
    /// Direct affordance exists at current location, or one-step plan is obvious.
    Likely,
    /// Cannot determine feasibility cheaply — needs full GOAP search.
    Uncertain,
    /// Blocker memory, exhausted frame, or missing prerequisites strongly suggest infeasibility.
    Unlikely,
}

/// Derive a cheap feasibility estimate for a ranked goal using only the
/// agent's beliefs, blocker memory, and intention frame state.
/// Never touches authoritative world state.
pub fn feasibility_hint(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    goal: &RankedGoal,
    blocked_memory: &BlockedIntentMemory,
    current_frame: Option<&IntentionFrame>,
    current_tick: Tick,
) -> FeasibilityHint {
    // Phase 1: Shared checks (short-circuit on first conclusive result)
    if let Some(hint) = check_exhausted_frame(goal, current_frame) {
        return hint;
    }
    if let Some(hint) = check_blocker_memory(goal, blocked_memory, current_tick) {
        return hint;
    }

    // Phase 2: Per-GoalKind dispatch
    if let Some(hint) = goal_specific_feasibility(view, agent, goal) {
        return hint;
    }

    // Phase 3: Default
    FeasibilityHint::Uncertain
}

/// Phase 1, Check 1: If the current intention frame is exhausted for this
/// exact goal, return `Unlikely`. Suspended frames are NOT treated as Unlikely.
fn check_exhausted_frame(
    goal: &RankedGoal,
    current_frame: Option<&IntentionFrame>,
) -> Option<FeasibilityHint> {
    let frame = current_frame?;
    if frame.state == FrameState::Exhausted && frame.goal == goal.grounded.key {
        return Some(FeasibilityHint::Unlikely);
    }
    None
}

/// Phase 1, Check 2: If any live blocker exists in memory for this goal,
/// return `Unlikely`.
fn check_blocker_memory(
    goal: &RankedGoal,
    blocked_memory: &BlockedIntentMemory,
    current_tick: Tick,
) -> Option<FeasibilityHint> {
    let has_live_blocker = blocked_memory.intents.values().any(|intent| {
        intent.blocker_key.goal_key == goal.grounded.key && intent.expires_tick > current_tick
    });
    if has_live_blocker {
        return Some(FeasibilityHint::Unlikely);
    }
    None
}

/// Phase 2: Per-GoalKind dispatch table. Returns `None` when no goal-specific
/// opinion exists (falls through to `Uncertain`).
fn goal_specific_feasibility(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    goal: &RankedGoal,
) -> Option<FeasibilityHint> {
    let strategy = GoalDispatchKey::from_goal_kind(&goal.grounded.key.kind)
        .declaration()
        .feasibility_strategy;

    match (strategy, &goal.grounded.key.kind) {
        (
            FeasibilityStrategy::OwnedCommodityCheck,
            GoalKind::ConsumeOwnedCommodity { commodity },
        ) => {
            if view.commodity_quantity(agent, *commodity) > Quantity(0) {
                Some(FeasibilityHint::Likely)
            } else {
                None // Uncertain — might acquire
            }
        }
        (FeasibilityStrategy::EvidencePlaceLocal, _) => {
            check_evidence_places_local(view, agent, goal)
        }
        (FeasibilityStrategy::AlwaysLikely, GoalKind::Sleep | GoalKind::Relieve) => {
            Some(FeasibilityHint::Likely)
        }
        (FeasibilityStrategy::CommodityPresenceCheck, GoalKind::Wash) => {
            if view.commodity_quantity(agent, CommodityKind::Water) > Quantity(0) {
                Some(FeasibilityHint::Likely)
            } else {
                None // Uncertain
            }
        }
        (
            FeasibilityStrategy::ColocationOrDead,
            GoalKind::EngageHostile { target } | GoalKind::RaidTarget { target },
        ) => check_colocated_or_dead(view, agent, *target),
        (FeasibilityStrategy::ColocationOrDead, GoalKind::TreatWounds { patient }) => {
            check_colocated_or_dead(view, agent, *patient)
        }
        (FeasibilityStrategy::ColocationOrDead, GoalKind::ShareBelief { listener, .. }) => {
            check_colocated_or_dead(view, agent, *listener)
        }
        (
            FeasibilityStrategy::ColocationOrDead,
            GoalKind::SupportCandidateForOffice { candidate, .. },
        ) => check_colocated_or_dead(view, agent, *candidate),
        (
            FeasibilityStrategy::ColocationOrDead,
            GoalKind::Accuse { accused, .. } | GoalKind::PunishAccused { accused, .. },
        ) => check_colocated_or_dead(view, agent, *accused),
        (
            FeasibilityStrategy::NoOpinion,
            GoalKind::ReduceDanger
            | GoalKind::RegroupWithFaction { .. }
            | GoalKind::EstablishBanditCamp { .. }
            | GoalKind::SearchForMissing { .. }
            | GoalKind::ReportMissing { .. }
            | GoalKind::EscortToSafety { .. }
            | GoalKind::FulfillBounty { .. }
            | GoalKind::PostBounty { .. }
            | GoalKind::PostNotice { .. }
            | GoalKind::StealItem { .. },
        ) => None,
        (FeasibilityStrategy::SellCheck, GoalKind::SellCommodity { commodity }) => {
            let has_commodity = view.commodity_quantity(agent, *commodity) > Quantity(0);
            if !has_commodity {
                return Some(FeasibilityHint::Unlikely);
            }
            let Some(profile) = view.merchandise_profile(agent) else {
                return Some(FeasibilityHint::Unlikely);
            };
            let Some(_home_facility) = profile.home_facility else {
                return Some(FeasibilityHint::Unlikely);
            };
            let Some(home_place) = merchant_home_place(view, agent, None) else {
                return Some(FeasibilityHint::Unlikely);
            };
            let agent_place = view.effective_place(agent);
            if agent_place == Some(home_place) {
                return Some(FeasibilityHint::Likely);
            }
            // Check if the home facility's place is reachable (adjacent).
            if let Some(place) = agent_place {
                let adjacent = view.adjacent_places_with_travel_ticks(place);
                if adjacent.iter().any(|(p, _)| *p == home_place) {
                    return Some(FeasibilityHint::Likely);
                }
            }
            None
        }
        (FeasibilityStrategy::CargoDestinationCheck, GoalKind::MoveCargo { commodity, .. }) => {
            let has_commodity = view.commodity_quantity(agent, *commodity) > Quantity(0);
            if has_commodity && let Some(agent_place) = view.effective_place(agent) {
                let destination = goal.grounded.key.place;
                if destination == Some(agent_place) {
                    return Some(FeasibilityHint::Likely);
                }
                let adjacent = view.adjacent_places_with_travel_ticks(agent_place);
                if destination.is_some_and(|d| adjacent.iter().any(|(p, _)| *p == d)) {
                    return Some(FeasibilityHint::Likely);
                }
            }
            None
        }
        (FeasibilityStrategy::CorpseBurialCheck, GoalKind::BuryCorpse { corpse, .. }) => {
            let agent_place = view.effective_place(agent)?;
            let corpse_place = view.effective_place(*corpse)?;
            if agent_place != corpse_place {
                return None;
            }
            let burial_site = goal.grounded.key.place;
            if burial_site == Some(agent_place) {
                return Some(FeasibilityHint::Likely);
            }
            if let Some(site) = burial_site {
                let adjacent = view.adjacent_places_with_travel_ticks(agent_place);
                if adjacent.iter().any(|(p, _)| *p == site) {
                    return Some(FeasibilityHint::Likely);
                }
            }
            None
        }
        (
            FeasibilityStrategy::PlaceMatch,
            GoalKind::InvestigateViolation { place, .. } | GoalKind::Patrol { place },
        ) => {
            let agent_place = view.effective_place(agent)?;
            if agent_place == *place {
                Some(FeasibilityHint::Likely)
            } else {
                None
            }
        }
        (strategy, goal_kind) => {
            unreachable!("feasibility strategy {strategy:?} does not match goal kind {goal_kind:?}")
        }
    }
}

/// Co-location check pattern: target at same place → Likely, target dead → Unlikely.
fn check_colocated_or_dead(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    target: EntityId,
) -> Option<FeasibilityHint> {
    let agent_place = view.effective_place(agent)?;
    let target_place = view.effective_place(target)?;
    if agent_place == target_place {
        return Some(FeasibilityHint::Likely);
    }
    if view.is_dead(target) {
        return Some(FeasibilityHint::Unlikely);
    }
    None
}

/// Evidence-place check pattern: agent's place in `evidence_places` → Likely.
fn check_evidence_places_local(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    goal: &RankedGoal,
) -> Option<FeasibilityHint> {
    let agent_place = view.effective_place(agent)?;
    if goal.grounded.evidence_places.contains(&agent_place) {
        return Some(FeasibilityHint::Likely);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        GoalDispatchKey,
        goal_model::{GoalPriorityClass, GroundedGoal, RankedGoal},
    };
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::{
        ArtifactPostingContext, BeliefConfidencePolicy, BlockedIntent, BlockerKey, BlockingFact,
        BountyTarget, BountyTerms, CommodityConsumableProfile, CommodityKind, CommodityPurpose,
        DriveThresholds, EntityId, EntityKind, GoalKey, GoalKind, HomeostaticNeeds,
        IntentionDomain, IntentionFrame, LoadUnits, MerchandiseProfile, NoticeTopic,
        PunishmentKind, Quantity, RecipeId, ResourceSource, RewardSource, TellTopic, Tick,
        UniqueItemKind, ViolationId, WorkstationTag,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    /// Minimal mock implementation of `GoalBeliefView` for feasibility tests.
    #[derive(Default)]
    struct MockView {
        agent_place: Option<EntityId>,
        /// Map from entity to place for non-agent entities.
        entity_places: Vec<(EntityId, EntityId)>,
        /// Commodity quantities for the agent.
        commodities: Vec<(CommodityKind, Quantity)>,
        /// Entities considered dead.
        dead: Vec<EntityId>,
        /// Adjacent places from agent's current place.
        adjacent: Vec<(EntityId, NonZeroU32)>,
        /// Merchandise profile for the agent.
        merchandise: Option<MerchandiseProfile>,
    }

    impl GoalBeliefView for MockView {
        fn is_alive(&self, entity: EntityId) -> bool {
            !self.dead.contains(&entity)
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            self.dead.contains(&entity)
        }

        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            if let Some(place) = self.agent_place {
                // Agent entity check: assume slot 1 is the agent
                if entity.slot == 1 {
                    return Some(place);
                }
            }
            self.entity_places
                .iter()
                .find(|(e, _)| *e == entity)
                .map(|(_, p)| *p)
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacent.clone()
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

        fn commodity_quantity(&self, _holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodities
                .iter()
                .find(|(k, _)| *k == kind)
                .map_or(Quantity(0), |(_, q)| *q)
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

        fn merchandise_profile(&self, _entity: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise.clone()
        }

        fn wounds(&self, _agent: EntityId) -> Vec<worldwake_core::Wound> {
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

        fn demand_memory(&self, _agent: EntityId) -> Vec<worldwake_core::DemandObservation> {
            Vec::new()
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    // -- Helper to build RankedGoal for tests --

    const AGENT: EntityId = EntityId {
        slot: 1,
        generation: 0,
    };

    fn ranked_goal(kind: GoalKind) -> RankedGoal {
        RankedGoal {
            grounded: GroundedGoal {
                anchor: worldwake_core::OpportunityAnchor::None,
                key: GoalKey::from(kind),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            },
            priority_class: GoalPriorityClass::Medium,
            motive_score: 500,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            feasibility: FeasibilityHint::Uncertain,
        }
    }

    fn ranked_goal_with_evidence_places(kind: GoalKind, places: &[EntityId]) -> RankedGoal {
        let mut g = ranked_goal(kind);
        g.grounded.evidence_places = places.iter().copied().collect();
        g
    }

    fn empty_blocked_memory() -> BlockedIntentMemory {
        BlockedIntentMemory::default()
    }

    fn make_frame(goal_key: GoalKey, state: FrameState) -> IntentionFrame {
        IntentionFrame {
            goal: goal_key,
            domain: IntentionDomain::Generic,
            assumptions: Vec::new(),
            state,
            established_at: Tick(1),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 10,
        }
    }

    fn make_blocker(goal_key: GoalKey, expires: Tick) -> BlockedIntent {
        BlockedIntent {
            blocker_key: BlockerKey {
                goal_key,
                place: None,
                target: None,
                action_def: None,
            },
            blocking_fact: BlockingFact::NoKnownPath,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: expires,
            clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
        }
    }

    fn goal_specific_feasibility_strategy(goal: &RankedGoal) -> crate::FeasibilityStrategy {
        GoalDispatchKey::from_goal_kind(&goal.grounded.key.kind)
            .declaration()
            .feasibility_strategy
    }

    // ── Test 1: Exhausted frame → Unlikely ──

    #[test]
    fn test_exhausted_frame_returns_unlikely() {
        let view = MockView::default();
        let goal = ranked_goal(GoalKind::Sleep);
        let frame = make_frame(goal.grounded.key, FrameState::Exhausted);
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, Some(&frame), Tick(10));
        assert_eq!(hint, FeasibilityHint::Unlikely);
    }

    // ── Test 2: Suspended frame not unlikely ──

    #[test]
    fn test_suspended_frame_not_unlikely() {
        let view = MockView::default();
        let goal = ranked_goal(GoalKind::Sleep);
        let frame = make_frame(
            goal.grounded.key,
            FrameState::Suspended {
                reason: worldwake_core::SuspensionReason::PriorityInterrupt,
                suspended_at: Tick(5),
            },
        );
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, Some(&frame), Tick(10));
        // Sleep is always Likely, and suspended frame should NOT make it Unlikely
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 3: Active blocker → Unlikely ──

    #[test]
    fn test_active_blocker_returns_unlikely() {
        let view = MockView::default();
        let goal = ranked_goal(GoalKind::Sleep);
        let mut blocked = empty_blocked_memory();
        blocked.record(make_blocker(goal.grounded.key, Tick(20)));

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(10));
        assert_eq!(hint, FeasibilityHint::Unlikely);
    }

    // ── Test 4: Expired blocker → no effect ──

    #[test]
    fn test_expired_blocker_not_unlikely() {
        let view = MockView::default();
        let goal = ranked_goal(GoalKind::Sleep);
        let mut blocked = empty_blocked_memory();
        blocked.record(make_blocker(goal.grounded.key, Tick(5)));

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(10));
        // Blocker expired at tick 5, current tick is 10 → no effect → Sleep → Likely
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 5: ConsumeOwnedCommodity with commodity → Likely ──

    #[test]
    fn test_consume_owned_with_commodity_likely() {
        let view = MockView {
            commodities: vec![(CommodityKind::Bread, Quantity(3))],
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 6: ConsumeOwnedCommodity without commodity → Uncertain ──

    #[test]
    fn test_consume_owned_without_commodity_uncertain() {
        let view = MockView::default();
        let goal = ranked_goal(GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Uncertain);
    }

    // ── Test 7: Sleep → Likely ──

    #[test]
    fn test_sleep_always_likely() {
        let view = MockView::default();
        let goal = ranked_goal(GoalKind::Sleep);
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 8: Relieve → Likely ──

    #[test]
    fn test_relieve_always_likely() {
        let view = MockView::default();
        let goal = ranked_goal(GoalKind::Relieve);
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 9: Wash with Water → Likely ──

    #[test]
    fn test_wash_with_water_likely() {
        let view = MockView {
            commodities: vec![(CommodityKind::Water, Quantity(1))],
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::Wash);
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 10: Wash without Water → Uncertain ──

    #[test]
    fn test_wash_without_water_uncertain() {
        let view = MockView::default();
        let goal = ranked_goal(GoalKind::Wash);
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Uncertain);
    }

    #[test]
    fn test_s59_no_opinion_goals_fall_through_to_uncertain() {
        let view = MockView::default();
        let blocked = empty_blocked_memory();
        let goals = [
            ranked_goal(GoalKind::SearchForMissing {
                subject: entity(5),
                last_seen: None,
            }),
            ranked_goal(GoalKind::ReportMissing {
                subject: entity(6),
                to_office: None,
                expectation_id: None,
            }),
            ranked_goal(GoalKind::EscortToSafety {
                subject: entity(7),
                destination: entity(8),
            }),
        ];

        for goal in goals {
            assert_eq!(
                feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1)),
                FeasibilityHint::Uncertain,
                "{:?} should fall through to NoOpinion/Uncertain",
                goal.grounded.key.kind
            );
            assert_eq!(
                goal_specific_feasibility_strategy(&goal),
                crate::FeasibilityStrategy::NoOpinion
            );
        }
    }

    // ── Test 11: EngageHostile co-located → Likely ──

    #[test]
    fn test_engage_hostile_colocated_likely() {
        let target = entity(5);
        let place = entity(10);
        let view = MockView {
            agent_place: Some(place),
            entity_places: vec![(target, place)],
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::EngageHostile { target });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 12: EngageHostile target dead → Unlikely ──

    #[test]
    fn test_engage_hostile_target_dead_unlikely() {
        let target = entity(5);
        let place = entity(10);
        let other_place = entity(11);
        let view = MockView {
            agent_place: Some(place),
            entity_places: vec![(target, other_place)],
            dead: vec![target],
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::EngageHostile { target });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Unlikely);
    }

    // ── Test 13: TreatWounds co-located → Likely ──

    #[test]
    fn test_treat_wounds_colocated_likely() {
        let patient = entity(5);
        let place = entity(10);
        let view = MockView {
            agent_place: Some(place),
            entity_places: vec![(patient, place)],
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::TreatWounds { patient });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 14: TreatWounds patient dead → Unlikely ──

    #[test]
    fn test_treat_wounds_patient_dead_unlikely() {
        let patient = entity(5);
        let place = entity(10);
        let other_place = entity(11);
        let view = MockView {
            agent_place: Some(place),
            entity_places: vec![(patient, other_place)],
            dead: vec![patient],
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::TreatWounds { patient });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Unlikely);
    }

    // ── Test 15: ShareBelief co-located → Likely ──

    #[test]
    fn test_share_belief_colocated_likely() {
        let listener = entity(5);
        let subject = entity(6);
        let place = entity(10);
        let view = MockView {
            agent_place: Some(place),
            entity_places: vec![(listener, place)],
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 16: ShareBelief listener dead → Unlikely ──

    #[test]
    fn test_share_belief_listener_dead_unlikely() {
        let listener = entity(5);
        let subject = entity(6);
        let place = entity(10);
        let other_place = entity(11);
        let view = MockView {
            agent_place: Some(place),
            entity_places: vec![(listener, other_place)],
            dead: vec![listener],
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Unlikely);
    }

    // ── Test 17: ClaimOffice with evidence at current place → Likely ──

    #[test]
    fn test_claim_office_evidence_local_likely() {
        let office = entity(20);
        let place = entity(10);
        let view = MockView {
            agent_place: Some(place),
            ..Default::default()
        };
        let goal = ranked_goal_with_evidence_places(GoalKind::ClaimOffice { office }, &[place]);
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 18: SellCommodity no commodity → Unlikely ──

    #[test]
    fn test_sell_commodity_no_commodity_unlikely() {
        let view = MockView {
            agent_place: Some(entity(10)),
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Unlikely);
    }

    // ── Test 19: SellCommodity with stock and local evidence → Likely ──

    #[test]
    fn test_sell_commodity_with_stock_and_at_home_facility_likely() {
        let place = entity(10);
        let facility = entity(11);
        let view = MockView {
            agent_place: Some(place),
            entity_places: vec![(facility, place)],
            commodities: vec![(CommodityKind::Bread, Quantity(5))],
            merchandise: Some(MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            }),
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test 20: ReduceDanger → Uncertain ──

    #[test]
    fn test_reduce_danger_returns_uncertain() {
        let view = MockView::default();
        let goal = ranked_goal(GoalKind::ReduceDanger);
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Uncertain);
    }

    #[test]
    fn posting_goals_with_no_opinion_strategy_return_uncertain_instead_of_panicking() {
        let view = MockView::default();
        let blocked = empty_blocked_memory();
        let place = entity(30);
        let office = entity(31);
        let target = entity(32);

        let post_bounty = ranked_goal(GoalKind::PostBounty {
            posting: ArtifactPostingContext {
                posting_place: place,
                issuing_authority: Some(office),
                expires_at: None,
                jurisdiction: Some(place),
            },
            terms: BountyTerms {
                target: BountyTarget::EliminateEntity { target },
                proof_requirement: worldwake_core::ProofRequirement::PhysicalEvidence,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(5),
                reward_source: RewardSource::InstitutionalTreasury {
                    treasury_entity: office,
                },
                claim_place: place,
            },
        });
        let post_notice = ranked_goal(GoalKind::PostNotice {
            posting: ArtifactPostingContext {
                posting_place: place,
                issuing_authority: None,
                expires_at: None,
                jurisdiction: Some(place),
            },
            topic: NoticeTopic::ThreatWarning { place },
        });

        assert_eq!(
            feasibility_hint(&view, AGENT, &post_bounty, &blocked, None, Tick(1)),
            FeasibilityHint::Uncertain
        );
        assert_eq!(
            feasibility_hint(&view, AGENT, &post_notice, &blocked, None, Tick(1)),
            FeasibilityHint::Uncertain
        );
    }

    // ── Test 21: Default uncertain ──

    #[test]
    fn test_default_uncertain() {
        // AcquireCommodity with no evidence_places and no agent place → Uncertain
        let view = MockView::default();
        let goal = ranked_goal(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Uncertain);
    }

    // ── Test: InvestigateViolation co-located → Likely ──

    #[test]
    fn test_investigate_missing_colocated_likely() {
        let place = entity(10);
        let view = MockView {
            agent_place: Some(place),
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::InvestigateViolation {
            violation_id: worldwake_core::ViolationId(1),
            place,
        });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    // ── Test: InvestigateViolation not co-located → Uncertain ──

    #[test]
    fn test_investigate_missing_not_colocated_uncertain() {
        let place = entity(10);
        let other_place = entity(11);
        let view = MockView {
            agent_place: Some(other_place),
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::InvestigateViolation {
            violation_id: worldwake_core::ViolationId(2),
            place,
        });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Uncertain);
    }

    #[test]
    fn test_patrol_colocated_likely() {
        let place = entity(12);
        let view = MockView {
            agent_place: Some(place),
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::Patrol { place });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Likely);
    }

    #[test]
    fn test_patrol_not_colocated_uncertain() {
        let place = entity(12);
        let other_place = entity(13);
        let view = MockView {
            agent_place: Some(other_place),
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::Patrol { place });
        let blocked = empty_blocked_memory();

        let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
        assert_eq!(hint, FeasibilityHint::Uncertain);
    }

    #[test]
    fn deferred_crime_and_justice_goals_default_to_uncertain() {
        let view = MockView::default();
        let blocked = empty_blocked_memory();

        for goal in [
            ranked_goal(GoalKind::StealItem {
                target_item: entity(12),
            }),
            ranked_goal(GoalKind::Accuse {
                crime_register: entity(17),
                accused: entity(13),
                violation_id: ViolationId(1),
            }),
            ranked_goal(GoalKind::PunishAccused {
                office: entity(16),
                accused: entity(14),
                accusation_entry: worldwake_core::RecordEntryId(1),
                punishment: PunishmentKind::Exile {
                    from_faction: entity(15),
                },
            }),
        ] {
            let hint = feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1));
            assert_eq!(hint, FeasibilityHint::Uncertain);
        }
    }

    #[test]
    fn declaration_routing_assigns_expected_feasibility_strategies() {
        assert_eq!(
            goal_specific_feasibility_strategy(&ranked_goal(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            })),
            crate::FeasibilityStrategy::EvidencePlaceLocal
        );
        assert_eq!(
            goal_specific_feasibility_strategy(&ranked_goal(GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination: entity(40),
            })),
            crate::FeasibilityStrategy::CargoDestinationCheck
        );
        assert_eq!(
            goal_specific_feasibility_strategy(&ranked_goal(GoalKind::BuryCorpse {
                corpse: entity(41),
                burial_site: entity(42),
            })),
            crate::FeasibilityStrategy::CorpseBurialCheck
        );
        assert_eq!(
            goal_specific_feasibility_strategy(&ranked_goal(GoalKind::PunishAccused {
                office: entity(43),
                accused: entity(44),
                accusation_entry: worldwake_core::RecordEntryId(2),
                punishment: PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(5),
                },
            })),
            crate::FeasibilityStrategy::ColocationOrDead
        );
    }

    #[test]
    fn move_cargo_destination_strategy_marks_local_and_adjacent_destinations_likely() {
        let destination = entity(11);
        let view = MockView {
            agent_place: Some(destination),
            commodities: vec![(CommodityKind::Bread, Quantity(2))],
            adjacent: vec![(entity(12), NonZeroU32::new(3).unwrap())],
            ..Default::default()
        };
        let blocked = empty_blocked_memory();
        let local_goal = ranked_goal(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination,
        });
        let adjacent_goal = ranked_goal(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination: entity(12),
        });

        assert_eq!(
            feasibility_hint(&view, AGENT, &local_goal, &blocked, None, Tick(1)),
            FeasibilityHint::Likely
        );
        assert_eq!(
            feasibility_hint(&view, AGENT, &adjacent_goal, &blocked, None, Tick(1)),
            FeasibilityHint::Likely
        );
    }

    // ── Invariant: Ord gives Likely < Uncertain < Unlikely ──

    #[test]
    fn feasibility_hint_ord_is_likely_uncertain_unlikely() {
        assert!(FeasibilityHint::Likely < FeasibilityHint::Uncertain);
        assert!(FeasibilityHint::Uncertain < FeasibilityHint::Unlikely);
        assert!(FeasibilityHint::Likely < FeasibilityHint::Unlikely);
    }

    // ── SellCheck feasibility ──

    #[test]
    fn sell_check_likely_when_has_commodity_and_at_home_facility() {
        let market = entity(10);
        let facility = entity(11);
        let view = MockView {
            agent_place: Some(market),
            entity_places: vec![(facility, market)],
            commodities: vec![(CommodityKind::Bread, Quantity(3))],
            merchandise: Some(MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            }),
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        });
        let blocked = empty_blocked_memory();

        assert_eq!(
            feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1)),
            FeasibilityHint::Likely
        );
    }

    #[test]
    fn sell_check_likely_when_has_commodity_and_home_facility_adjacent() {
        let current = entity(9);
        let market = entity(10);
        let facility = entity(11);
        let view = MockView {
            agent_place: Some(current),
            entity_places: vec![(facility, market)],
            commodities: vec![(CommodityKind::Bread, Quantity(3))],
            adjacent: vec![(market, NonZeroU32::new(2).unwrap())],
            merchandise: Some(MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            }),
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        });
        let blocked = empty_blocked_memory();

        assert_eq!(
            feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1)),
            FeasibilityHint::Likely
        );
    }

    #[test]
    fn sell_check_unlikely_when_no_commodity() {
        let market = entity(10);
        let view = MockView {
            agent_place: Some(market),
            merchandise: Some(MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(market),
            }),
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        });
        let blocked = empty_blocked_memory();

        assert_eq!(
            feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1)),
            FeasibilityHint::Unlikely
        );
    }

    #[test]
    fn sell_check_unlikely_when_no_merchandise_profile() {
        let market = entity(10);
        let view = MockView {
            agent_place: Some(market),
            commodities: vec![(CommodityKind::Bread, Quantity(3))],
            ..Default::default()
        };
        let goal = ranked_goal(GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        });
        let blocked = empty_blocked_memory();

        assert_eq!(
            feasibility_hint(&view, AGENT, &goal, &blocked, None, Tick(1)),
            FeasibilityHint::Unlikely
        );
    }
}
