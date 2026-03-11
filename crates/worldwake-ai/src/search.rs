use crate::{
    derive_danger_pressure, GoalKindPlannerExt, GroundedGoal, PlanTerminalKind, PlannedPlan,
    PlannedStep, PlannerOpKind, PlannerOpSemantics, PlanningBudget, PlanningSnapshot,
    PlanningState,
};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use worldwake_core::{
    CommodityKind, CommodityPurpose, EntityId, GoalKind, Permille, Quantity,
};
use worldwake_sim::{
    get_affordances, ActionDef, ActionDefId, ActionDefRegistry, ActionDuration,
    ActionHandlerRegistry, ActionPayload, BeliefView, CombatActionPayload, LootActionPayload,
    TradeActionPayload,
};

#[derive(Clone)]
struct SearchNode<'snapshot> {
    state: PlanningState<'snapshot>,
    steps: Vec<PlannedStep>,
    total_estimated_ticks: u32,
}

pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    budget: &PlanningBudget,
) -> Option<PlannedPlan> {
    if unsupported_goal(&goal.key.kind) {
        return None;
    }

    let actor = snapshot.actor();
    let mut frontier = vec![SearchNode {
        state: PlanningState::new(snapshot),
        steps: Vec::new(),
        total_estimated_ticks: 0,
    }];
    let mut expansions = 0u16;

    while let Some(node) = pop_next_node(&mut frontier) {
        if goal_is_satisfied(goal, &node.state) {
            return Some(PlannedPlan::new(
                goal.key,
                node.steps,
                PlanTerminalKind::GoalSatisfied,
            ));
        }
        if node.steps.len() >= usize::from(budget.max_plan_depth) {
            continue;
        }
        if expansions >= budget.max_node_expansions {
            return None;
        }
        expansions = expansions.saturating_add(1);

        let mut successors = get_affordances(&node.state, actor, registry, handlers)
            .into_iter()
            .filter_map(|affordance| {
                let def = registry.get(affordance.def_id)?;
                let semantics = semantics_table.get(&affordance.def_id)?;
                if !goal.key.kind.relevant_op_kinds().contains(&semantics.op_kind) {
                    return None;
                }

                let payload_override = build_payload_override(
                    affordance.payload_override.as_ref(),
                    &goal.key.kind,
                    &node.state,
                    &affordance.bound_targets,
                    def,
                    semantics,
                )
                .ok()?;
                let effective_payload = payload_override.as_ref().unwrap_or(&def.payload);
                let duration = node.state.estimate_duration(
                    actor,
                    &def.duration,
                    &affordance.bound_targets,
                    effective_payload,
                )?;
                let estimated_ticks = match duration {
                    ActionDuration::Finite(ticks) => ticks,
                    ActionDuration::Indefinite if semantics.may_appear_mid_plan => return None,
                    ActionDuration::Indefinite => 0,
                };

                let post_state = apply_step(
                    node.state.clone(),
                    &goal.key.kind,
                    semantics.op_kind,
                    &affordance.bound_targets,
                );
                let step = PlannedStep {
                    def_id: affordance.def_id,
                    targets: affordance.bound_targets,
                    payload_override,
                    op_kind: semantics.op_kind,
                    estimated_ticks,
                    is_materialization_barrier: semantics.is_materialization_barrier,
                };
                let terminal = terminal_kind(goal, &post_state, &step);
                if !semantics.may_appear_mid_plan && terminal.is_none() {
                    return None;
                }
                let total_estimated_ticks = node
                    .total_estimated_ticks
                    .checked_add(estimated_ticks)?;
                let mut steps = node.steps.clone();
                steps.push(step);

                Some((terminal, SearchNode {
                    state: post_state,
                    steps,
                    total_estimated_ticks,
                }))
            })
            .collect::<Vec<_>>();
        successors.sort_by(|left, right| compare_successors(&left.1, &right.1));
        successors.truncate(usize::from(budget.beam_width));

        for (terminal, successor) in successors {
            if let Some(terminal_kind) = terminal {
                return Some(PlannedPlan::new(goal.key, successor.steps, terminal_kind));
            }
            frontier.push(successor);
        }
    }

    None
}

fn unsupported_goal(goal: &GoalKind) -> bool {
    matches!(
        goal,
        GoalKind::SellCommodity { .. }
            | GoalKind::MoveCargo { .. }
            | GoalKind::BuryCorpse { .. }
    )
}

fn pop_next_node<'snapshot>(
    frontier: &mut Vec<SearchNode<'snapshot>>,
) -> Option<SearchNode<'snapshot>> {
    if frontier.is_empty() {
        return None;
    }
    frontier.sort_by(compare_search_nodes);
    Some(frontier.remove(0))
}

fn compare_search_nodes(left: &SearchNode<'_>, right: &SearchNode<'_>) -> Ordering {
    left.total_estimated_ticks
        .cmp(&right.total_estimated_ticks)
        .then_with(|| left.steps.len().cmp(&right.steps.len()))
        .then_with(|| left.steps.cmp(&right.steps))
}

fn compare_successors(
    left: &SearchNode<'_>,
    right: &SearchNode<'_>,
) -> Ordering {
    compare_search_nodes(left, right)
}

fn build_payload_override(
    affordance_payload: Option<&ActionPayload>,
    goal: &GoalKind,
    state: &PlanningState<'_>,
    targets: &[EntityId],
    def: &ActionDef,
    semantics: &PlannerOpSemantics,
) -> Result<Option<ActionPayload>, ()> {
    if let Some(payload) = affordance_payload {
        return Ok(Some(payload.clone()));
    }

    let actor = state.snapshot().actor();
    match semantics.op_kind {
        PlannerOpKind::Trade => {
            let Some(counterparty) = targets.first().copied() else {
                return Err(());
            };
            let requested_commodity = match goal {
                GoalKind::AcquireCommodity { commodity, .. }
                | GoalKind::RestockCommodity { commodity }
                | GoalKind::ConsumeOwnedCommodity { commodity } => *commodity,
                GoalKind::Heal { .. } => CommodityKind::Medicine,
                _ => return Err(()),
            };
            let Some(actor_place) = state.effective_place(actor) else {
                return Err(());
            };
            if !state
                .agents_selling_at(actor_place, requested_commodity)
                .contains(&counterparty)
            {
                return Err(());
            }
            if state.commodity_quantity(counterparty, requested_commodity) == Quantity(0) {
                return Err(());
            }
            if state.commodity_quantity(actor, CommodityKind::Coin) == Quantity(0) {
                return Err(());
            }
            Ok(Some(ActionPayload::Trade(TradeActionPayload {
                counterparty,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(1),
                requested_commodity,
                requested_quantity: Quantity(1),
            })))
        }
        PlannerOpKind::Attack => {
            let Some(target) = targets.first().copied() else {
                return Err(());
            };
            Ok(Some(ActionPayload::Combat(CombatActionPayload {
                target,
                weapon: worldwake_core::CombatWeaponRef::Unarmed,
            })))
        }
        PlannerOpKind::Loot => {
            let Some(target) = targets.first().copied() else {
                return Err(());
            };
            Ok(Some(ActionPayload::Loot(LootActionPayload { target })))
        }
        _ => Ok((!matches!(def.payload, ActionPayload::None)).then(|| def.payload.clone())),
    }
}

fn apply_step<'snapshot>(
    state: PlanningState<'snapshot>,
    goal: &GoalKind,
    op_kind: PlannerOpKind,
    targets: &[EntityId],
) -> PlanningState<'snapshot> {
    match op_kind {
        PlannerOpKind::Travel => {
            if let Some(destination) = targets.first().copied() {
                state.move_actor_to(destination)
            } else {
                state
            }
        }
        PlannerOpKind::Consume => match goal {
            GoalKind::ConsumeOwnedCommodity { commodity } | GoalKind::AcquireCommodity { commodity, .. } => {
                state.consume_commodity(*commodity)
            }
            _ => state,
        },
        PlannerOpKind::Sleep => update_actor_needs(state, |needs, thresholds| {
            needs.fatigue = below_medium(thresholds.fatigue.medium());
        }),
        PlannerOpKind::Relieve => update_actor_needs(state, |needs, thresholds| {
            needs.bladder = below_medium(thresholds.bladder.medium());
        }),
        PlannerOpKind::Wash => update_actor_needs(state, |needs, thresholds| {
            needs.dirtiness = below_medium(thresholds.dirtiness.medium());
        }),
        PlannerOpKind::Heal => match goal {
            GoalKind::Heal { target } => {
                let Some(thresholds) = state.drive_thresholds(*target) else {
                    return state;
                };
                state.with_pain(*target, below_medium(thresholds.pain.medium()))
            }
            _ => state,
        },
        _ => state,
    }
}

fn update_actor_needs(
    state: PlanningState<'_>,
    apply: impl FnOnce(&mut worldwake_core::HomeostaticNeeds, worldwake_core::DriveThresholds),
) -> PlanningState<'_> {
    let actor = state.snapshot().actor();
    let Some(mut needs) = state.homeostatic_needs(actor) else {
        return state;
    };
    let Some(thresholds) = state.drive_thresholds(actor) else {
        return state;
    };
    apply(&mut needs, thresholds);
    state.with_homeostatic_needs(actor, needs)
}

fn below_medium(medium: Permille) -> Permille {
    medium.saturating_sub(Permille::new(1).unwrap())
}

fn terminal_kind(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    step: &PlannedStep,
) -> Option<PlanTerminalKind> {
    if matches!(step.op_kind, PlannerOpKind::Attack | PlannerOpKind::Defend) {
        return Some(PlanTerminalKind::CombatCommitment);
    }
    if goal_is_satisfied(goal, state) {
        return Some(PlanTerminalKind::GoalSatisfied);
    }
    progress_barrier(goal, step).then_some(PlanTerminalKind::ProgressBarrier)
}

fn progress_barrier(goal: &GroundedGoal, step: &PlannedStep) -> bool {
    if !step.is_materialization_barrier {
        return false;
    }

    match goal.key.kind {
        GoalKind::AcquireCommodity { .. }
        | GoalKind::ProduceCommodity { .. }
        | GoalKind::RestockCommodity { .. }
        | GoalKind::LootCorpse { .. } => true,
        GoalKind::ConsumeOwnedCommodity { .. } => matches!(
            step.op_kind,
            PlannerOpKind::Trade | PlannerOpKind::Harvest | PlannerOpKind::Craft | PlannerOpKind::MoveCargo
        ),
        GoalKind::Heal { .. } => step.op_kind == PlannerOpKind::Trade,
        _ => false,
    }
}

fn goal_is_satisfied(goal: &GroundedGoal, state: &PlanningState<'_>) -> bool {
    let actor = state.snapshot().actor();
    match goal.key.kind {
        GoalKind::ConsumeOwnedCommodity { commodity } => {
            let Some(needs) = state.homeostatic_needs(actor) else {
                return false;
            };
            let Some(thresholds) = state.drive_thresholds(actor) else {
                return false;
            };
            match commodity {
                CommodityKind::Bread | CommodityKind::Apple | CommodityKind::Grain => {
                    needs.hunger < thresholds.hunger.medium()
                }
                CommodityKind::Water => needs.thirst < thresholds.thirst.medium(),
                _ => false,
            }
        }
        GoalKind::AcquireCommodity { commodity, purpose } => match purpose {
            CommodityPurpose::SelfConsume | CommodityPurpose::Restock | CommodityPurpose::Treatment => {
                state.commodity_quantity(actor, commodity) > Quantity(0)
            }
            CommodityPurpose::RecipeInput(_) => state.commodity_quantity(actor, commodity) > Quantity(0),
        },
        GoalKind::Sleep => state
            .homeostatic_needs(actor)
            .zip(state.drive_thresholds(actor))
            .is_some_and(|(needs, thresholds)| needs.fatigue < thresholds.fatigue.medium()),
        GoalKind::Relieve => state
            .homeostatic_needs(actor)
            .zip(state.drive_thresholds(actor))
            .is_some_and(|(needs, thresholds)| needs.bladder < thresholds.bladder.medium()),
        GoalKind::Wash => state
            .homeostatic_needs(actor)
            .zip(state.drive_thresholds(actor))
            .is_some_and(|(needs, thresholds)| needs.dirtiness < thresholds.dirtiness.medium()),
        GoalKind::ReduceDanger => state
            .drive_thresholds(actor)
            .is_some_and(|thresholds| derive_danger_pressure(state, actor) < thresholds.danger.high()),
        GoalKind::Heal { target } => state
            .drive_thresholds(target)
            .zip(state.pain_summary(target))
            .is_some_and(|(thresholds, pain)| pain < thresholds.pain.medium()),
        GoalKind::ProduceCommodity { .. }
        | GoalKind::RestockCommodity { .. }
        | GoalKind::LootCorpse { .. }
        | GoalKind::SellCommodity { .. }
        | GoalKind::MoveCargo { .. }
        | GoalKind::BuryCorpse { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::search_plan;
    use crate::{
        build_planning_snapshot, build_semantics_table, CommodityPurpose, GoalKey, GroundedGoal,
        PlanningBudget, PlanTerminalKind, PlannerOpKind,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        test_utils::sample_trade_disposition_profile, CombatProfile, CommodityConsumableProfile,
        CommodityKind, DemandObservation, DriveThresholds, EntityId, EntityKind,
        HomeostaticNeeds, InTransitOnEdge, MerchandiseProfile, MetabolismProfile, Permille,
        Quantity, RecipeId, ResourceSource, TickRange, TradeDispositionProfile, UniqueItemKind,
        WorkstationTag, Wound,
    };
    use worldwake_sim::{
        estimate_duration_from_beliefs, ActionDefRegistry, ActionPayload, BeliefView,
        DurationExpr, RecipeRegistry,
    };
    use worldwake_systems::{
        register_attack_action, register_craft_actions, register_defend_action, register_harvest_actions,
        register_heal_action, register_loot_action, register_needs_actions, register_trade_action,
        register_transport_actions, register_travel_actions,
    };

    #[derive(Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        controllable: BTreeSet<(EntityId, EntityId)>,
        adjacent: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        lot_commodities: BTreeMap<EntityId, CommodityKind>,
        consumable_profiles: BTreeMap<EntityId, CommodityConsumableProfile>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        trade_profiles: BTreeMap<EntityId, TradeDispositionProfile>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
    }

    impl BeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool { self.alive.contains(&entity) }
        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> { self.kinds.get(&entity).copied() }
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> { self.effective_places.get(&entity).copied() }
        fn is_in_transit(&self, _entity: EntityId) -> bool { false }
        fn entities_at(&self, place: EntityId) -> Vec<EntityId> { self.entities_at.get(&place).cloned().unwrap_or_default() }
        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> { self.direct_possessions.get(&holder).cloned().unwrap_or_default() }
        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places_with_travel_ticks(place).into_iter().map(|(place, _)| place).collect()
        }
        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool { false }
        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 { 0 }
        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodity_quantities.get(&(holder, kind)).copied().unwrap_or(Quantity(0))
        }
        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> { self.lot_commodities.get(&entity).copied() }
        fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile> { self.consumable_profiles.get(&entity).copied() }
        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> { None }
        fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> { self.direct_possessors.get(&entity).copied() }
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> { None }
        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> { None }
        fn has_production_job(&self, _entity: EntityId) -> bool { false }
        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool { self.controllable.contains(&(actor, entity)) }
        fn has_control(&self, entity: EntityId) -> bool { self.kinds.get(&entity) == Some(&EntityKind::Agent) }
        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool { false }
        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> { Vec::new() }
        fn is_dead(&self, entity: EntityId) -> bool { !self.is_alive(entity) }
        fn is_incapacitated(&self, _entity: EntityId) -> bool { false }
        fn has_wounds(&self, _entity: EntityId) -> bool { false }
        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> { self.needs.get(&agent).copied() }
        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> { self.thresholds.get(&agent).copied() }
        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> { Some(MetabolismProfile::default()) }
        fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> { self.trade_profiles.get(&agent).cloned() }
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            Some(CombatProfile::new(
                pm(1000),
                pm(700),
                pm(620),
                pm(580),
                pm(80),
                pm(25),
                pm(18),
                pm(120),
                pm(35),
                NonZeroU32::new(6).unwrap(),
            ))
        }
        fn wounds(&self, _agent: EntityId) -> Vec<Wound> { Vec::new() }
        fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> { self.hostiles.get(&agent).cloned().unwrap_or_default() }
        fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> { self.attackers.get(&agent).cloned().unwrap_or_default() }
        fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.entities_at(place).into_iter().filter(|entity| {
                self.merchandise_profiles.get(entity).is_some_and(|profile| profile.sale_kinds.contains(&commodity))
            }).collect()
        }
        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> { Vec::new() }
        fn matching_workstations_at(&self, _place: EntityId, _tag: WorkstationTag) -> Vec<EntityId> { Vec::new() }
        fn resource_sources_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> { Vec::new() }
        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> { Vec::new() }
        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> { self.merchandise_profiles.get(&agent).cloned() }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> { Vec::new() }
        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> { None }
        fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacent.get(&place).cloned().unwrap_or_default()
        }
        fn estimate_duration(
            &self,
            actor: EntityId,
            duration: &DurationExpr,
            targets: &[EntityId],
            payload: &ActionPayload,
        ) -> Option<worldwake_sim::ActionDuration> {
            estimate_duration_from_beliefs(self, actor, duration, targets, payload)
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId { slot, generation: 1 }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn build_registry() -> (ActionDefRegistry, worldwake_sim::ActionHandlerRegistry) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = worldwake_sim::ActionHandlerRegistry::new();
        let recipes = RecipeRegistry::new();
        register_needs_actions(&mut defs, &mut handlers);
        let _ = register_travel_actions(&mut defs, &mut handlers);
        let _ = register_transport_actions(&mut defs, &mut handlers);
        let _ = register_trade_action(&mut defs, &mut handlers);
        let _ = register_harvest_actions(&mut defs, &mut handlers, &recipes);
        let _ = register_craft_actions(&mut defs, &mut handlers, &recipes);
        let _ = register_attack_action(&mut defs, &mut handlers);
        let _ = register_defend_action(&mut defs, &mut handlers);
        let _ = register_loot_action(&mut defs, &mut handlers);
        let _ = register_heal_action(&mut defs, &mut handlers);
        (defs, handlers)
    }

    fn consume_goal(commodity: CommodityKind) -> GroundedGoal {
        GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity { commodity }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        }
    }

    #[test]
    fn search_returns_one_step_consume_plan_for_local_food() {
        let actor = entity(1);
        let town = entity(10);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, town);
        view.entities_at.insert(town, vec![actor, bread]);
        view.controllable.insert((actor, bread));
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
        )
        .unwrap();

        assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Consume);
    }

    #[test]
    fn search_returns_travel_then_consume_for_adjacent_food() {
        let actor = entity(1);
        let town = entity(10);
        let field = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, field, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(field, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, field);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(field, vec![bread]);
        view.controllable.insert((actor, bread));
        view.adjacent.insert(town, vec![(field, NonZeroU32::new(3).unwrap())]);
        view.adjacent.insert(field, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
        )
        .unwrap();

        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
        assert_eq!(plan.steps[1].op_kind, PlannerOpKind::Consume);
    }

    #[test]
    fn search_returns_travel_then_trade_barrier_for_reachable_seller() {
        let actor = entity(1);
        let town = entity(10);
        let market = entity(11);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town, market]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(market, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, market);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(market, vec![seller]);
        view.adjacent.insert(town, vec![(market, NonZeroU32::new(4).unwrap())]);
        view.adjacent.insert(market, vec![(town, NonZeroU32::new(4).unwrap())]);
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        view.trade_profiles
            .insert(actor, sample_trade_disposition_profile());
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );
        view.commodity_quantities.insert((actor, CommodityKind::Coin), Quantity(3));
        view.commodity_quantities.insert((seller, CommodityKind::Bread), Quantity(2));
        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::from([seller]),
            evidence_places: BTreeSet::from([market]),
        };
        let snapshot = build_planning_snapshot(&view, actor, &goal.evidence_entities, &goal.evidence_places, 1);
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
        )
        .unwrap();

        assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
        assert_eq!(plan.steps[1].op_kind, PlannerOpKind::Trade);
        assert!(matches!(plan.steps[1].payload_override, Some(ActionPayload::Trade(_))));
    }

    #[test]
    fn search_respects_plan_depth_budget() {
        let actor = entity(1);
        let town = entity(10);
        let field = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, field, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(field, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, field);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(field, vec![bread]);
        view.controllable.insert((actor, bread));
        view.adjacent.insert(town, vec![(field, NonZeroU32::new(3).unwrap())]);
        view.adjacent.insert(field, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut budget = PlanningBudget::default();
        budget.max_plan_depth = 1;
        let plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &budget,
        );

        assert_eq!(plan, None);
    }

    #[test]
    fn search_returns_none_when_node_expansion_budget_is_exhausted() {
        let actor = entity(1);
        let town = entity(10);
        let field = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, field, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(field, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, field);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(field, vec![bread]);
        view.controllable.insert((actor, bread));
        view.adjacent.insert(town, vec![(field, NonZeroU32::new(3).unwrap())]);
        view.adjacent.insert(field, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut budget = PlanningBudget::default();
        budget.max_node_expansions = 0;
        let plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &budget,
        );

        assert_eq!(plan, None);
    }

    #[test]
    fn search_rejects_branch_when_duration_estimation_fails() {
        let actor = entity(1);
        let town = entity(10);
        let market = entity(11);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town, market]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, market);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(market, vec![seller]);
        view.adjacent.insert(town, vec![(market, NonZeroU32::new(3).unwrap())]);
        view.adjacent.insert(market, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );
        view.commodity_quantities.insert((actor, CommodityKind::Coin), Quantity(3));
        view.commodity_quantities.insert((seller, CommodityKind::Bread), Quantity(2));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::from([seller]),
            evidence_places: BTreeSet::from([market]),
        };

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
        );

        assert_eq!(plan, None);
    }

    #[test]
    fn search_uses_hypothetical_movement_to_reduce_local_danger() {
        let actor = entity(1);
        let attacker = entity(2);
        let town = entity(10);
        let refuge = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, attacker, town, refuge]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(attacker, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(refuge, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(attacker, town);
        view.entities_at.insert(town, vec![actor, attacker]);
        view.entities_at.insert(refuge, Vec::new());
        view.adjacent.insert(town, vec![(refuge, NonZeroU32::new(2).unwrap())]);
        view.adjacent.insert(refuge, vec![(town, NonZeroU32::new(2).unwrap())]);
        view.thresholds.insert(actor, DriveThresholds::default());
        view.hostiles.insert(actor, vec![attacker]);
        view.attackers.insert(actor, vec![attacker]);
        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::ReduceDanger),
            evidence_entities: BTreeSet::from([attacker]),
            evidence_places: BTreeSet::from([town, refuge]),
        };
        let snapshot = build_planning_snapshot(&view, actor, &goal.evidence_entities, &goal.evidence_places, 1);
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
        )
        .unwrap();

        assert_eq!(plan.steps.len(), 1);
        assert!(matches!(
            (plan.steps[0].op_kind, plan.terminal_kind),
            (PlannerOpKind::Travel, PlanTerminalKind::GoalSatisfied)
                | (PlannerOpKind::Attack | PlannerOpKind::Defend, PlanTerminalKind::CombatCommitment)
        ));
    }

    #[test]
    fn search_marks_leaf_combat_as_combat_commitment() {
        let actor = entity(1);
        let attacker = entity(2);
        let town = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, attacker, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(attacker, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(attacker, town);
        view.entities_at.insert(town, vec![actor, attacker]);
        view.thresholds.insert(actor, DriveThresholds::default());
        view.hostiles.insert(actor, vec![attacker]);
        view.attackers.insert(actor, vec![attacker]);

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::ReduceDanger),
            evidence_entities: BTreeSet::from([attacker]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(&view, actor, &goal.evidence_entities, &goal.evidence_places, 0);
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
        )
        .unwrap();

        assert!(matches!(
            plan.steps[0].op_kind,
            PlannerOpKind::Attack | PlannerOpKind::Defend
        ));
        assert_eq!(plan.terminal_kind, PlanTerminalKind::CombatCommitment);
    }
}
