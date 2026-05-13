#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use crate::agent_tick::portfolio::FeasibilityVerdict;
use crate::failure_handling::place_has_local_commodity_support;
use crate::goal_model::{AgendaEntry, RootCandidateSynthesis};
use crate::{GoalKindPlannerExt, PlannerOpKind, PlannerOpSemantics};
use worldwake_core::{
    ActionDefId, BlockerKey, BlockerMemory, Discrepancy, DiscrepancyMemory, EntityId, GoalKind,
    OpportunityAnchor, Tick,
};
use worldwake_sim::{
    ActionDefRegistry, ActionHandlerRegistry, RuntimeBeliefView, get_affordances_for_defs,
};

pub(crate) struct ProbeContext<'a> {
    pub(crate) belief_view: &'a dyn RuntimeBeliefView,
    pub(crate) discrepancy_memory: &'a DiscrepancyMemory,
    pub(crate) blocker_memory: &'a BlockerMemory,
    pub(crate) semantics_table: &'a BTreeMap<ActionDefId, PlannerOpSemantics>,
    pub(crate) action_defs: &'a ActionDefRegistry,
    pub(crate) action_handlers: &'a ActionHandlerRegistry,
    pub(crate) current_tick: Tick,
    pub(crate) agent: EntityId,
    pub(crate) agent_place: Option<EntityId>,
}

pub(crate) fn probe(ranked: &AgendaEntry, context: &ProbeContext<'_>) -> FeasibilityVerdict {
    let blocker_key = blocker_key_for_probe(ranked);
    if let Some(entry) = context
        .discrepancy_memory
        .entries
        .get(&blocker_key)
        .filter(|entry| entry.expires_tick > context.current_tick)
    {
        return FeasibilityVerdict::RejectedBeforeSearch {
            reason: entry.discrepancy,
        };
    }

    if context.blocker_memory.is_blocked(
        &blocker_key.goal_key,
        blocker_key.place,
        blocker_key.target,
        blocker_key.action_def,
        context.current_tick,
    ) {
        return FeasibilityVerdict::RejectedBeforeSearch {
            reason: Discrepancy::PartialExecutionDrift,
        };
    }

    if let Some(reason) = known_target_failure(ranked, context) {
        return FeasibilityVerdict::RejectedBeforeSearch { reason };
    }

    if let Some(reason) = current_place_support_failure(ranked, context) {
        return FeasibilityVerdict::RejectedBeforeSearch { reason };
    }

    if !has_relevant_affordance(ranked, context) && !has_synthesized_root_candidate(ranked, context)
    {
        return FeasibilityVerdict::RejectedBeforeSearch {
            reason: Discrepancy::MissingObservation,
        };
    }

    FeasibilityVerdict::Plausible
}

fn blocker_key_for_probe(ranked: &AgendaEntry) -> BlockerKey {
    let anchor_place = match ranked.offer.anchor {
        OpportunityAnchor::Place(place) => Some(place),
        OpportunityAnchor::Entity(_) | OpportunityAnchor::None => None,
    };
    let anchor_target = match ranked.offer.anchor {
        OpportunityAnchor::Entity(target) => Some(target),
        OpportunityAnchor::Place(_) | OpportunityAnchor::None => None,
    };

    BlockerKey {
        goal_key: ranked.offer.key,
        place: anchor_place.or(ranked.offer.key.place),
        target: anchor_target.or(ranked.offer.key.entity),
        // The standalone probe runs before root candidate expansion, so it can
        // only honestly query the goal/anchor-scoped blocker lane here.
        action_def: None,
    }
}

fn known_target_failure(ranked: &AgendaEntry, context: &ProbeContext<'_>) -> Option<Discrepancy> {
    let target = match ranked.offer.anchor {
        OpportunityAnchor::Entity(target) => Some(target),
        OpportunityAnchor::Place(_) | OpportunityAnchor::None => ranked.offer.key.entity,
    };

    if let Some(target) = target {
        if requires_exact_identity_target(ranked, context) {
            let envelope = context
                .belief_view
                .believed_target_location(context.agent, target);
            match envelope.status {
                worldwake_sim::belief_view::BeliefStatus::Stale => {
                    return Some(Discrepancy::BeliefStale);
                }
                worldwake_sim::belief_view::BeliefStatus::Contradicted => {
                    return Some(Discrepancy::BeliefContradicted);
                }
                worldwake_sim::belief_view::BeliefStatus::Certain
                | worldwake_sim::belief_view::BeliefStatus::Probable
                | worldwake_sim::belief_view::BeliefStatus::Disputed => {}
            }
        }

        let target_known = context.belief_view.entity_kind(target).is_some()
            || context.belief_view.effective_place(target).is_some()
            || context.belief_view.is_alive(target)
            || context.belief_view.is_dead(target);
        if !target_known {
            return Some(Discrepancy::MissingObservation);
        }

        if let (Some(agent_place), Some(target_place)) = (
            context.agent_place,
            context.belief_view.effective_place(target),
        ) && agent_place != target_place
            && !context.belief_view.route_exists(agent_place, target_place)
        {
            return Some(Discrepancy::RouteUnknown);
        }
    }

    let place = match ranked.offer.anchor {
        OpportunityAnchor::Place(place) => Some(place),
        OpportunityAnchor::Entity(_) | OpportunityAnchor::None => ranked.offer.key.place,
    };
    if let (Some(agent_place), Some(target_place)) = (context.agent_place, place)
        && agent_place != target_place
        && !context.belief_view.route_exists(agent_place, target_place)
    {
        return Some(Discrepancy::RouteUnknown);
    }

    None
}

fn requires_exact_identity_target(ranked: &AgendaEntry, context: &ProbeContext<'_>) -> bool {
    relevant_action_defs(ranked, context.semantics_table)
        .into_iter()
        .filter_map(|def_id| context.action_defs.get(def_id))
        .any(|def| def.binding_strictness == worldwake_sim::BindingStrictness::ExactIdentity)
}

fn current_place_support_failure(
    ranked: &AgendaEntry,
    context: &ProbeContext<'_>,
) -> Option<Discrepancy> {
    let (GoalKind::AcquireCommodity { commodity, .. } | GoalKind::RestockCommodity { commodity }) =
        ranked.offer.key.kind
    else {
        return None;
    };
    let place = match ranked.offer.anchor {
        OpportunityAnchor::Place(place) => Some(place),
        OpportunityAnchor::Entity(target) => context
            .belief_view
            .effective_place(target)
            .or(ranked.offer.key.place),
        OpportunityAnchor::None => ranked.offer.key.place.or_else(|| {
            let agent_place = context.agent_place?;
            let has_local_evidence = ranked.offer.evidence_places.contains(&agent_place)
                || ranked
                    .offer
                    .evidence_entities
                    .iter()
                    .copied()
                    .any(|entity| context.belief_view.effective_place(entity) == Some(agent_place));
            has_local_evidence.then_some(agent_place)
        }),
    }?;

    if context.agent_place != Some(place) {
        return None;
    }

    let local_evidence_entities = ranked
        .offer
        .evidence_entities
        .iter()
        .copied()
        .filter(|entity| context.belief_view.effective_place(*entity) == Some(place))
        .collect::<Vec<_>>();
    if !local_evidence_entities.is_empty()
        && !local_evidence_entities
            .iter()
            .copied()
            .any(|entity| entity_supports_commodity(context, entity, commodity))
    {
        return Some(Discrepancy::MissingObservation);
    }
    if !has_local_goal_affordance(ranked, context, &local_evidence_entities) {
        return Some(Discrepancy::MissingObservation);
    }

    (!place_has_local_commodity_support(context.belief_view, context.agent, place, commodity, None))
        .then_some(Discrepancy::MissingObservation)
}

fn entity_supports_commodity(
    context: &ProbeContext<'_>,
    entity: EntityId,
    commodity: worldwake_core::CommodityKind,
) -> bool {
    context
        .belief_view
        .resource_source(entity)
        .is_some_and(|resource| {
            resource.commodity == commodity
                && resource.available_quantity > worldwake_core::Quantity(0)
        })
        || (context.belief_view.item_lot_commodity(entity) == Some(commodity)
            && context.belief_view.commodity_quantity(entity, commodity)
                > worldwake_core::Quantity(0)
            && context.belief_view.direct_container(entity).is_none()
            && context.belief_view.direct_possessor(entity).is_none())
        || context.belief_view.commodity_quantity(entity, commodity) > worldwake_core::Quantity(0)
        || context
            .belief_view
            .listed_sale_lots_at(context.agent_place.unwrap_or(entity), commodity)
            .into_iter()
            .any(|lot| lot == entity)
}

fn has_local_goal_affordance(
    ranked: &AgendaEntry,
    context: &ProbeContext<'_>,
    local_evidence_entities: &[EntityId],
) -> bool {
    let relevant_defs = relevant_action_defs(ranked, context.semantics_table);
    if relevant_defs.is_empty() {
        return false;
    }

    get_affordances_for_defs(
        context.belief_view,
        context.agent,
        context.action_defs,
        context.action_handlers,
        &relevant_defs,
    )
    .into_iter()
    .filter(|affordance| {
        context
            .semantics_table
            .get(&affordance.def_id)
            .is_some_and(|semantics| semantics.op_kind != PlannerOpKind::Travel)
    })
    .any(|affordance| {
        local_evidence_entities.is_empty()
            || affordance
                .bound_targets
                .iter()
                .any(|target| local_evidence_entities.contains(target))
    })
}

fn has_relevant_affordance(ranked: &AgendaEntry, context: &ProbeContext<'_>) -> bool {
    let relevant_defs = relevant_action_defs(ranked, context.semantics_table);
    if relevant_defs.is_empty() {
        return false;
    }

    !get_affordances_for_defs(
        context.belief_view,
        context.agent,
        context.action_defs,
        context.action_handlers,
        &relevant_defs,
    )
    .is_empty()
}

fn has_synthesized_root_candidate(ranked: &AgendaEntry, context: &ProbeContext<'_>) -> bool {
    relevant_action_defs(ranked, context.semantics_table)
        .into_iter()
        .filter_map(|def_id| {
            Some((
                context.action_defs.get(def_id)?,
                *context.semantics_table.get(&def_id)?,
            ))
        })
        .any(|(def, semantics)| {
            matches!(
                ranked.offer.synthesized_root_candidate_targets(
                    def,
                    semantics,
                    context.agent_place
                ),
                RootCandidateSynthesis::Targets(_)
            )
        })
}

fn relevant_action_defs(
    ranked: &AgendaEntry,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> BTreeSet<ActionDefId> {
    let relevant_ops = ranked.offer.key.kind.relevant_op_kinds();
    semantics_table
        .iter()
        .filter(|(_, semantics)| relevant_ops.contains(&semantics.op_kind))
        .map(|(def_id, _)| *def_id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ProbeContext, blocker_key_for_probe, probe};
    use crate::{
        AgendaEntry, GoalOffer, GoalPriorityClass, PlannerOpSemantics,
        agent_tick::portfolio::FeasibilityVerdict, build_semantics_table,
        feasibility::FeasibilityHint,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, ActionDomain, AgentBeliefStore, ArtifactPostingContext,
        BeliefConfidencePolicy, Blocker, BlockerClearingCondition, BlockerMemory, BlockingFact,
        BodyCostPerTick, CombatProfile, CommodityConsumableProfile, CommodityKind,
        DemandObservation, Discrepancy, DiscrepancyClearing, DiscrepancyEntry, DiscrepancyMemory,
        DriveThresholds, EntityId, EntityKind, GoalKind, HomeostaticNeeds, InTransitOnEdge,
        IntentionDispositionProfile, LoadUnits, MerchandiseProfile, MetabolismProfile, NoticeTopic,
        OpportunityAnchor, Permille, Quantity, RecipeId, ResourceSource, Tick, TickRange,
        TradeDispositionProfile, UniqueItemKind, VisibilitySpec, WorkstationTag, Wound,
    };
    use worldwake_sim::{
        ActionDef, ActionDefRegistry, ActionDuration, ActionError, ActionExecutionContext,
        ActionHandler, ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress,
        ActionState, CombatBeliefView, CommitOutcome, Constraint, ControlBeliefView, DurationExpr,
        EconomicBeliefView, EntityBeliefView, FacilityBeliefView, Interruptibility,
        InventoryBeliefView, PoliticalBeliefView, ProfileBeliefView, RuntimeBeliefView,
        SocialBeliefView, SpatialBeliefView, TargetSpec, TemporalBeliefView,
        belief_view::{BeliefStatus, BeliefValue},
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn ranked_goal(kind: GoalKind, anchor: OpportunityAnchor) -> AgendaEntry {
        AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: kind.into(),
                anchor,
            },
            offer: GoalOffer {
                key: kind.into(),
                anchor,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            },
            priority_class: GoalPriorityClass::High,
            motive_score: 500,
            motive_source_contributions: Vec::new(),
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            source_composite: None,
            feasibility: FeasibilityHint::Uncertain,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        }
    }

    struct ProbeHarness {
        defs: ActionDefRegistry,
        handlers: ActionHandlerRegistry,
        semantics: BTreeMap<ActionDefId, PlannerOpSemantics>,
    }

    impl ProbeHarness {
        fn sleep_only() -> Self {
            let mut handlers = ActionHandlerRegistry::new();
            let handler = handlers.register(ActionHandler::new(
                noop_start,
                noop_tick,
                noop_commit,
                noop_abort,
            ));

            let mut defs = ActionDefRegistry::new();
            defs.register(ActionDef {
                id: ActionDefId(0),
                name: "sleep".to_owned(),
                domain: ActionDomain::Needs,
                actor_constraints: vec![Constraint::ActorAlive],
                targets: Vec::new(),
                preconditions: Vec::new(),
                reservation_requirements: Vec::new(),
                duration: DurationExpr::Fixed(NonZeroU32::new(1).expect("nonzero")),
                body_cost_per_tick: BodyCostPerTick::zero(),
                attention_cost: Permille::ZERO,
                interruptibility: Interruptibility::FreelyInterruptible,
                commit_conditions: Vec::new(),
                visibility: VisibilitySpec::SamePlace,
                causal_event_tags: BTreeSet::new(),
                payload: ActionPayload::None,
                handler,
                binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
                guard_template: None,
                expectation_template: vec![],
                effect_schema: worldwake_sim::EffectSchema::empty(),
            });

            Self {
                semantics: build_semantics_table(&defs),
                defs,
                handlers,
            }
        }

        fn heal_only() -> Self {
            let mut handlers = ActionHandlerRegistry::new();
            let handler = handlers.register(ActionHandler::new(
                noop_start,
                noop_tick,
                noop_commit,
                noop_abort,
            ));

            let mut defs = ActionDefRegistry::new();
            defs.register(ActionDef {
                id: ActionDefId(0),
                name: "heal".to_owned(),
                domain: ActionDomain::Care,
                actor_constraints: vec![Constraint::ActorAlive],
                targets: vec![worldwake_sim::TargetSpec::SpecificEntity(entity(999))],
                preconditions: Vec::new(),
                reservation_requirements: Vec::new(),
                duration: DurationExpr::Fixed(NonZeroU32::new(1).expect("nonzero")),
                body_cost_per_tick: BodyCostPerTick::zero(),
                attention_cost: Permille::ZERO,
                interruptibility: Interruptibility::FreelyInterruptible,
                commit_conditions: Vec::new(),
                visibility: VisibilitySpec::SamePlace,
                causal_event_tags: BTreeSet::new(),
                payload: ActionPayload::None,
                handler,
                binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
                guard_template: None,
                expectation_template: vec![],
                effect_schema: worldwake_sim::EffectSchema::empty(),
            });

            Self {
                semantics: build_semantics_table(&defs),
                defs,
                handlers,
            }
        }

        fn post_notice_only() -> Self {
            let mut handlers = ActionHandlerRegistry::new();
            let handler = handlers.register(ActionHandler::new(
                noop_start,
                noop_tick,
                noop_commit,
                noop_abort,
            ));

            let mut defs = ActionDefRegistry::new();
            defs.register(ActionDef {
                id: ActionDefId(0),
                name: "post_notice".to_owned(),
                domain: ActionDomain::Social,
                actor_constraints: vec![
                    Constraint::ActorAlive,
                    Constraint::ActorHasControl,
                    Constraint::ActorNotInTransit,
                ],
                targets: vec![TargetSpec::ActorPlace],
                preconditions: Vec::new(),
                reservation_requirements: Vec::new(),
                duration: DurationExpr::Fixed(NonZeroU32::new(1).expect("nonzero")),
                body_cost_per_tick: BodyCostPerTick::zero(),
                attention_cost: Permille::ZERO,
                interruptibility: Interruptibility::FreelyInterruptible,
                commit_conditions: Vec::new(),
                visibility: VisibilitySpec::SamePlace,
                causal_event_tags: BTreeSet::new(),
                payload: ActionPayload::None,
                handler,
                binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
                guard_template: None,
                expectation_template: vec![],
                effect_schema: worldwake_sim::EffectSchema::empty(),
            });

            Self {
                semantics: build_semantics_table(&defs),
                defs,
                handlers,
            }
        }
    }

    fn target_location_belief(
        place: Option<EntityId>,
        status: BeliefStatus,
    ) -> BeliefValue<Option<EntityId>> {
        BeliefValue {
            value: place,
            confidence: Permille::new_unchecked(900),
            acquired_tick: Tick(4),
            claimed_event_tick: Some(Tick(4)),
            status,
        }
    }

    fn probe_context<'a>(
        harness: &'a ProbeHarness,
        view: &'a MockView,
        agent: EntityId,
        agent_place: Option<EntityId>,
        discrepancy_memory: &'a DiscrepancyMemory,
        blocker_memory: &'a BlockerMemory,
        current_tick: Tick,
    ) -> ProbeContext<'a> {
        ProbeContext {
            belief_view: view,
            discrepancy_memory,
            blocker_memory,
            semantics_table: &harness.semantics,
            action_defs: &harness.defs,
            action_handlers: &harness.handlers,
            current_tick,
            agent,
            agent_place,
        }
    }

    #[test]
    fn probe_rejects_on_discrepancy_memory_hit() {
        let harness = ProbeHarness::sleep_only();
        let agent = entity(1);
        let place = entity(2);
        let ranked = ranked_goal(GoalKind::Sleep, OpportunityAnchor::None);
        let mut view = MockView::default();
        view.alive.insert(agent);
        view.places.insert(agent, place);

        let key = blocker_key_for_probe(&ranked);
        let discrepancy_memory = DiscrepancyMemory {
            entries: BTreeMap::from([(
                key,
                DiscrepancyEntry {
                    blocker_key: key,
                    discrepancy: Discrepancy::RouteUnknown,
                    observed_tick: Tick(4),
                    expires_tick: Tick(10),
                    clearing_condition: DiscrepancyClearing::TtlExpiry,
                },
            )]),
        };

        let verdict = probe(
            &ranked,
            &probe_context(
                &harness,
                &view,
                agent,
                Some(place),
                &discrepancy_memory,
                &BlockerMemory::default(),
                Tick(5),
            ),
        );

        assert_eq!(
            verdict,
            FeasibilityVerdict::RejectedBeforeSearch {
                reason: Discrepancy::RouteUnknown,
            }
        );
    }

    #[test]
    fn probe_rejects_on_blocker_memory_hit() {
        let harness = ProbeHarness::sleep_only();
        let agent = entity(1);
        let place = entity(2);
        let ranked = ranked_goal(GoalKind::Sleep, OpportunityAnchor::None);
        let mut view = MockView::default();
        view.alive.insert(agent);
        view.places.insert(agent, place);

        let key = blocker_key_for_probe(&ranked);
        let blocker_memory = BlockerMemory {
            intents: BTreeMap::from([(
                key,
                Blocker {
                    blocker_key: key,
                    blocking_fact: BlockingFact::PatienceExhausted,
                    diagnostic_context: None,
                    observed_tick: Tick(4),
                    expires_tick: Tick(10),
                    clearing_condition: BlockerClearingCondition::TtlOnly,
                    baseline_snapshot: None,
                },
            )]),
        };

        let verdict = probe(
            &ranked,
            &probe_context(
                &harness,
                &view,
                agent,
                Some(place),
                &DiscrepancyMemory::default(),
                &blocker_memory,
                Tick(5),
            ),
        );

        assert_eq!(
            verdict,
            FeasibilityVerdict::RejectedBeforeSearch {
                reason: Discrepancy::PartialExecutionDrift,
            }
        );
    }

    #[test]
    fn probe_rejects_on_missing_target() {
        let harness = ProbeHarness::sleep_only();
        let agent = entity(1);
        let place = entity(2);
        let patient = entity(3);
        let ranked = ranked_goal(
            GoalKind::TreatWounds { patient },
            OpportunityAnchor::Entity(patient),
        );
        let mut view = MockView::default();
        view.alive.insert(agent);
        view.places.insert(agent, place);

        let verdict = probe(
            &ranked,
            &probe_context(
                &harness,
                &view,
                agent,
                Some(place),
                &DiscrepancyMemory::default(),
                &BlockerMemory::default(),
                Tick(5),
            ),
        );

        assert_eq!(
            verdict,
            FeasibilityVerdict::RejectedBeforeSearch {
                reason: Discrepancy::MissingObservation,
            }
        );
    }

    #[test]
    fn probe_passes_when_belief_satisfied() {
        let harness = ProbeHarness::sleep_only();
        let agent = entity(1);
        let place = entity(2);
        let ranked = ranked_goal(GoalKind::Sleep, OpportunityAnchor::None);
        let mut view = MockView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.places.insert(agent, place);

        let verdict = probe(
            &ranked,
            &probe_context(
                &harness,
                &view,
                agent,
                Some(place),
                &DiscrepancyMemory::default(),
                &BlockerMemory::default(),
                Tick(5),
            ),
        );

        assert_eq!(verdict, FeasibilityVerdict::Plausible);
    }

    #[test]
    fn probe_rejects_stale_exact_target_belief_before_search() {
        let harness = ProbeHarness::heal_only();
        let agent = entity(1);
        let place = entity(2);
        let patient = entity(3);
        let ranked = ranked_goal(
            GoalKind::TreatWounds { patient },
            OpportunityAnchor::Entity(patient),
        );
        let mut view = MockView::default();
        view.alive.extend([agent, patient]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(patient, EntityKind::Agent);
        view.places.insert(agent, place);
        view.places.insert(patient, place);
        view.believed_target_locations.insert(
            (agent, patient),
            target_location_belief(Some(place), BeliefStatus::Stale),
        );

        let verdict = probe(
            &ranked,
            &probe_context(
                &harness,
                &view,
                agent,
                Some(place),
                &DiscrepancyMemory::default(),
                &BlockerMemory::default(),
                Tick(5),
            ),
        );

        assert_eq!(
            verdict,
            FeasibilityVerdict::RejectedBeforeSearch {
                reason: Discrepancy::BeliefStale,
            }
        );
    }

    #[test]
    fn probe_rejects_contradicted_exact_target_belief_before_search() {
        let harness = ProbeHarness::heal_only();
        let agent = entity(1);
        let place = entity(2);
        let patient = entity(3);
        let ranked = ranked_goal(
            GoalKind::TreatWounds { patient },
            OpportunityAnchor::Entity(patient),
        );
        let mut view = MockView::default();
        view.alive.extend([agent, patient]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(patient, EntityKind::Agent);
        view.places.insert(agent, place);
        view.places.insert(patient, place);
        view.believed_target_locations.insert(
            (agent, patient),
            target_location_belief(Some(place), BeliefStatus::Contradicted),
        );

        let verdict = probe(
            &ranked,
            &probe_context(
                &harness,
                &view,
                agent,
                Some(place),
                &DiscrepancyMemory::default(),
                &BlockerMemory::default(),
                Tick(5),
            ),
        );

        assert_eq!(
            verdict,
            FeasibilityVerdict::RejectedBeforeSearch {
                reason: Discrepancy::BeliefContradicted,
            }
        );
    }

    #[test]
    fn probe_rejects_place_anchored_current_place_acquire_without_local_support() {
        let harness = ProbeHarness::sleep_only();
        let agent = entity(1);
        let place = entity(2);
        let ranked = ranked_goal(
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: crate::CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            },
            OpportunityAnchor::Place(place),
        );
        let mut view = MockView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.places.insert(agent, place);

        let verdict = probe(
            &ranked,
            &probe_context(
                &harness,
                &view,
                agent,
                Some(place),
                &DiscrepancyMemory::default(),
                &BlockerMemory::default(),
                Tick(5),
            ),
        );

        assert_eq!(
            verdict,
            FeasibilityVerdict::RejectedBeforeSearch {
                reason: Discrepancy::MissingObservation,
            }
        );
    }

    #[test]
    fn probe_accepts_post_notice_via_synthesized_root_candidate_without_affordance() {
        let harness = ProbeHarness::post_notice_only();
        let agent = entity(1);
        let place = entity(2);
        let ranked = ranked_goal(
            GoalKind::PostNotice {
                posting: ArtifactPostingContext {
                    posting_place: place,
                    issuing_authority: None,
                    expires_at: Some(Tick(12)),
                    jurisdiction: Some(place),
                },
                topic: NoticeTopic::ThreatWarning { place },
            },
            OpportunityAnchor::Place(place),
        );
        let mut view = MockView::default();
        view.alive.extend([agent, place]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(place, EntityKind::Place);
        view.places.insert(agent, place);

        let verdict = probe(
            &ranked,
            &probe_context(
                &harness,
                &view,
                agent,
                Some(place),
                &DiscrepancyMemory::default(),
                &BlockerMemory::default(),
                Tick(5),
            ),
        );

        assert_eq!(verdict, FeasibilityVerdict::Plausible);
    }

    #[test]
    fn probe_rejects_unanchored_current_place_acquire_without_local_support() {
        let harness = ProbeHarness::sleep_only();
        let agent = entity(1);
        let place = entity(2);
        let ranked = AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: crate::CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                }
                .into(),
                anchor: OpportunityAnchor::None,
            },
            offer: GoalOffer {
                key: GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: crate::CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                }
                .into(),
                anchor: OpportunityAnchor::None,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::from([place]),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            },
            priority_class: GoalPriorityClass::High,
            motive_score: 500,
            motive_source_contributions: Vec::new(),
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            source_composite: None,
            feasibility: FeasibilityHint::Uncertain,

            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        };
        let mut view = MockView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.places.insert(agent, place);

        let verdict = probe(
            &ranked,
            &probe_context(
                &harness,
                &view,
                agent,
                Some(place),
                &DiscrepancyMemory::default(),
                &BlockerMemory::default(),
                Tick(5),
            ),
        );

        assert_eq!(
            verdict,
            FeasibilityVerdict::RejectedBeforeSearch {
                reason: Discrepancy::MissingObservation,
            }
        );
    }

    #[test]
    fn probe_rejects_entity_anchored_current_place_acquire_without_local_support() {
        let harness = ProbeHarness::sleep_only();
        let agent = entity(1);
        let place = entity(2);
        let source = entity(3);
        let ranked = ranked_goal(
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: crate::CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            },
            OpportunityAnchor::Entity(source),
        );
        let mut view = MockView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(source, EntityKind::Facility);
        view.places.insert(agent, place);
        view.places.insert(source, place);

        let verdict = probe(
            &ranked,
            &probe_context(
                &harness,
                &view,
                agent,
                Some(place),
                &DiscrepancyMemory::default(),
                &BlockerMemory::default(),
                Tick(5),
            ),
        );

        assert_eq!(
            verdict,
            FeasibilityVerdict::RejectedBeforeSearch {
                reason: Discrepancy::MissingObservation,
            }
        );
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_start(
        _def: &ActionDef,
        _instance: &mut ActionInstance,
        _ctx: &ActionExecutionContext<'_>,
        _rng: &mut worldwake_sim::DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_tick(
        _def: &ActionDef,
        _instance: &mut ActionInstance,
        _ctx: &ActionExecutionContext<'_>,
        _rng: &mut worldwake_sim::DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Complete)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_commit(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _ctx: &ActionExecutionContext<'_>,
        _event_log: &worldwake_core::EventLog,
        _rng: &mut worldwake_sim::DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<CommitOutcome, ActionError> {
        Ok(CommitOutcome::default())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_abort(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _ctx: &ActionExecutionContext<'_>,
        _reason: &worldwake_sim::AbortReason,
        _event_log: &worldwake_core::EventLog,
        _rng: &mut worldwake_sim::DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    #[derive(Default)]
    struct MockView {
        alive: BTreeSet<EntityId>,
        entity_kinds: BTreeMap<EntityId, EntityKind>,
        places: BTreeMap<EntityId, EntityId>,
        routes: BTreeSet<(EntityId, EntityId)>,
        believed_target_locations: BTreeMap<(EntityId, EntityId), BeliefValue<Option<EntityId>>>,
    }

    impl ControlBeliefView for MockView {
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl worldwake_sim::BelievedAuthorityView for MockView {}

    impl EntityBeliefView for MockView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.entity_kinds.get(&entity).copied()
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn believed_target_location(
            &self,
            agent: EntityId,
            target: EntityId,
        ) -> BeliefValue<Option<EntityId>> {
            self.believed_target_locations
                .get(&(agent, target))
                .copied()
                .unwrap_or_else(|| worldwake_sim::belief_view::stale_default_value(None))
        }
    }

    impl ProfileBeliefView for MockView {
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
    }

    impl SpatialBeliefView for MockView {
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.places
                .iter()
                .filter_map(|(entity, entity_place)| (*entity_place == place).then_some(*entity))
                .collect()
        }

        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn route_exists(&self, from: EntityId, to: EntityId) -> bool {
            self.routes.contains(&(from, to))
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
        }
    }

    impl TemporalBeliefView for MockView {
        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            _duration: &DurationExpr,
            _targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            None
        }
    }

    impl InventoryBeliefView for MockView {
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
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

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
    }

    impl CombatBeliefView for MockView {
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }

        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl EconomicBeliefView for MockView {
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
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

        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }
    }

    impl SocialBeliefView for MockView {
        fn agent_belief_store(&self, _agent: EntityId) -> Option<&AgentBeliefStore> {
            None
        }

        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<IntentionDispositionProfile> {
            None
        }
    }

    impl PoliticalBeliefView for MockView {}

    impl FacilityBeliefView for MockView {
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
    }

    impl RuntimeBeliefView for MockView {}
}
