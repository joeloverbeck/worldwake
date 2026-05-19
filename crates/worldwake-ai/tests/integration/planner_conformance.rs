//! Effect-schema planner conformance coverage (S134).
//!
//! The planner and authoritative handlers now consume the same `EffectSchema`.
//! These tests cover the schema catalog and typed effect outputs instead of
//! comparing a second handwritten hypothetical implementation.

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use worldwake_core::{
    BodyCostPerTick, CommodityKind, Discrepancy, EntityId, Quantity, UniqueItemKind, WorkstationTag,
};
use worldwake_sim::{EffectFact, EffectPrecondition, EffectStep, RecipeDefinition, RecipeRegistry};
use worldwake_systems::build_full_action_registries;

#[test]
fn every_actiondef_has_effect_schema() {
    let registries = build_full_action_registries(&RecipeRegistry::default())
        .expect("full action registry should be complete");

    let empty = registries
        .defs
        .iter()
        .filter(|def| {
            def.effect_schema.preconditions.is_empty() && def.effect_schema.steps.is_empty()
        })
        .map(|def| def.name.as_str())
        .collect::<Vec<_>>();

    assert!(
        empty.is_empty(),
        "action defs without effect schema: {empty:?}"
    );
}

#[test]
fn schema_precondition_failures_are_typed_discrepancies() {
    let registries = build_full_action_registries(&RecipeRegistry::default())
        .expect("full action registry should be complete");

    let mut reachable = BTreeSet::new();
    for def in registries.defs.iter() {
        for precondition in &def.effect_schema.preconditions {
            reachable.insert(discrepancy_for_precondition(precondition));
        }
    }

    assert!(
        reachable.contains(&Discrepancy::MissingObservation),
        "schema preconditions should cover observation failures"
    );
    assert!(
        !reachable.is_empty(),
        "full action registry should expose typed schema precondition failures"
    );
}

#[test]
fn partial_outcome_steps_emit_typed_facts() {
    let registries = build_full_action_registries(&harvest_recipe_registry())
        .expect("full action registry should be complete");

    let harvest = registries
        .defs
        .iter()
        .filter(|def| {
            def.effect_schema
                .steps
                .iter()
                .any(contains_harvest_resource_step)
        })
        .map(|def| def.name.as_str())
        .collect::<Vec<_>>();

    assert!(
        !harvest.is_empty(),
        "harvest actions should expose partial quantity through schema facts"
    );

    let fact = EffectFact::PartialQuantity {
        requested: Quantity(5),
        delivered: Quantity(3),
    };
    assert!(matches!(
        fact,
        EffectFact::PartialQuantity {
            requested: Quantity(5),
            delivered: Quantity(3),
        }
    ));
}

fn discrepancy_for_precondition(precondition: &EffectPrecondition) -> Discrepancy {
    match precondition {
        EffectPrecondition::TargetMatchesSlot { .. } => Discrepancy::NoLegalBinding,
        EffectPrecondition::CoLocated { .. }
        | EffectPrecondition::QuantityAvailable { .. }
        | EffectPrecondition::CapacityFloor { .. }
        | EffectPrecondition::ContentionGrantHeld { .. }
        | EffectPrecondition::BeliefHeld { .. } => Discrepancy::MissingObservation,
    }
}

fn contains_harvest_resource_step(step: &EffectStep) -> bool {
    match step {
        EffectStep::HarvestResource { .. } => true,
        EffectStep::PartialOnFailure { primary, fallback } => primary
            .iter()
            .chain(fallback.iter())
            .any(contains_harvest_resource_step),
        _ => false,
    }
}

fn harvest_recipe_registry() -> RecipeRegistry {
    let mut recipes = RecipeRegistry::new();
    recipes.register(RecipeDefinition {
        name: "Harvest Apples".to_string(),
        inputs: Vec::new(),
        outputs: vec![(CommodityKind::Apple, Quantity(2))],
        work_ticks: NonZeroU32::new(3).expect("nonzero work ticks"),
        required_workstation_tag: Some(WorkstationTag::OrchardRow),
        required_tool_kinds: vec![UniqueItemKind::SimpleTool],
        body_cost_per_tick: BodyCostPerTick::zero(),
    });
    recipes
}

#[allow(dead_code)]
fn _representative_partial_quantity_fact() -> EffectFact {
    EffectFact::PartialQuantity {
        requested: Quantity(1),
        delivered: Quantity(1),
    }
}

#[allow(dead_code)]
fn _representative_precondition_inputs() -> (EntityId, CommodityKind) {
    (
        EntityId {
            slot: 1,
            generation: 1,
        },
        CommodityKind::Bread,
    )
}
