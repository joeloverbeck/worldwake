use crate::{GoalBeliefView, RecipeRegistry};
use std::collections::BTreeMap;
use worldwake_core::{CommodityKind, EntityId};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct CommodityOpportunityBreakdown {
    pub direct_survival_score: u32,
    pub treatment_score: u32,
    pub enterprise_score: u32,
    pub indirect_recipe_score: u32,
}

#[must_use]
pub fn commodity_opportunity_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    _recipes: &RecipeRegistry,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
) -> CommodityOpportunityBreakdown {
    CommodityOpportunityBreakdown {
        direct_survival_score: direct_survival_score(
            actor,
            commodity,
            belief,
            holdings,
            local_alternatives,
        ),
        treatment_score: treatment_score(actor, commodity, belief, holdings, local_alternatives),
        enterprise_score: enterprise_score(actor, commodity, belief, holdings, local_alternatives),
        indirect_recipe_score: 0,
    }
}

fn direct_survival_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
) -> u32 {
    let Some(profile) = commodity.spec().consumable_profile else {
        return 0;
    };
    let Some(needs) = belief.homeostatic_needs(actor) else {
        return 0;
    };

    let quantity = accessible_quantity(holdings, local_alternatives, commodity);
    let hunger_relief = quantity * u64::from(profile.hunger_relief_per_unit.value());
    let thirst_relief = quantity * u64::from(profile.thirst_relief_per_unit.value());

    saturating_u64_to_u32(
        hunger_relief.min(u64::from(needs.hunger.value()))
            + thirst_relief.min(u64::from(needs.thirst.value())),
    )
}

fn treatment_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
) -> u32 {
    if commodity.spec().treatment_profile.is_none() {
        return 0;
    }

    let wounds = belief.wounds(actor);
    if wounds.is_empty() {
        return 0;
    }

    let total_severity = wounds
        .iter()
        .map(|wound| u64::from(wound.severity.value()))
        .sum::<u64>();
    let wound_count = wounds.len() as u64;
    let accessible_medicine = accessible_quantity(holdings, local_alternatives, commodity);

    saturating_u64_to_u32(accessible_medicine.min(wound_count) * total_severity)
}

fn enterprise_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
) -> u32 {
    let remembered_quantity = belief
        .demand_memory(actor)
        .into_iter()
        .filter(|observation| observation.commodity == commodity)
        .map(|observation| u64::from(observation.quantity.0))
        .sum::<u64>();

    if remembered_quantity == 0 {
        return 0;
    }

    saturating_u64_to_u32(
        accessible_quantity(holdings, local_alternatives, commodity).min(remembered_quantity),
    )
}

fn accessible_quantity(
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
    commodity: CommodityKind,
) -> u64 {
    let held = u64::from(holdings.get(&commodity).copied().unwrap_or(0));
    let alternatives = if commodity == CommodityKind::Coin {
        0
    } else {
        u64::from(local_alternatives.get(&commodity).copied().unwrap_or(0))
    };
    held + alternatives
}

fn saturating_u64_to_u32(value: u64) -> u32 {
    value.try_into().unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{commodity_opportunity_score, CommodityOpportunityBreakdown};
    use crate::{GoalBeliefView, RecipeRegistry};
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        BeliefConfidencePolicy, BodyPart, CommodityKind, DemandObservation,
        DemandObservationReason, DriveThresholds, EntityId, EntityKind, HomeostaticNeeds,
        LoadUnits, MerchandiseProfile, Permille, Quantity, RecipeId, ResourceSource, Tick,
        Wound, WoundCause,
    };

    #[derive(Default)]
    struct StubBeliefView {
        needs: Option<HomeostaticNeeds>,
        wounds: Vec<Wound>,
        demand_memory: Vec<DemandObservation>,
    }

    impl GoalBeliefView for StubBeliefView {
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
            None
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
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: worldwake_core::UniqueItemKind) -> u32 {
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
        ) -> Option<worldwake_core::CommodityConsumableProfile> {
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

        fn workstation_tag(&self, _entity: EntityId) -> Option<worldwake_core::WorkstationTag> {
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
            _tag: worldwake_core::WorkstationTag,
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
            !self.wounds.is_empty()
        }

        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs
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
            self.wounds.clone()
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

        fn listed_sale_lots_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
            Vec::new()
        }

        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
        }

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.clone()
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    fn actor() -> EntityId {
        EntityId {
            slot: 1,
            generation: 0,
        }
    }

    fn holdings(entries: &[(CommodityKind, u32)]) -> BTreeMap<CommodityKind, u32> {
        entries.iter().copied().collect()
    }

    fn demand_observation(commodity: CommodityKind, quantity: u32) -> DemandObservation {
        DemandObservation {
            commodity,
            quantity: Quantity(quantity),
            place: EntityId {
                slot: 9,
                generation: 0,
            },
            tick: Tick(12),
            counterparty: None,
            reason: DemandObservationReason::WantedToBuyButNoSeller,
        }
    }

    fn wound(severity: u16) -> Wound {
        Wound {
            id: worldwake_core::WoundId(1),
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
            severity: Permille::new(severity).unwrap(),
            inflicted_at: Tick(5),
            bleed_rate_per_tick: Permille::new(0).unwrap(),
        }
    }

    fn recipes() -> RecipeRegistry {
        RecipeRegistry::new()
    }

    #[test]
    fn commodity_opportunity_survival_score_tracks_need_pressure_for_consumables() {
        let view = StubBeliefView {
            needs: Some(HomeostaticNeeds::new(
                Permille::new(300).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            )),
            ..Default::default()
        };

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Bread,
            &view,
            &recipes(),
            &holdings(&[(CommodityKind::Bread, 1)]),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.direct_survival_score, 260);
        assert_eq!(breakdown.treatment_score, 0);
        assert_eq!(breakdown.enterprise_score, 0);
    }

    #[test]
    fn commodity_opportunity_non_consumable_has_no_survival_score() {
        let view = StubBeliefView {
            needs: Some(HomeostaticNeeds::new(
                Permille::new(800).unwrap(),
                Permille::new(600).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            )),
            ..Default::default()
        };

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Firewood,
            &view,
            &recipes(),
            &holdings(&[(CommodityKind::Firewood, 2)]),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.direct_survival_score, 0);
    }

    #[test]
    fn commodity_opportunity_treatment_score_tracks_wound_severity_for_medicine() {
        let view = StubBeliefView {
            wounds: vec![wound(300), wound(200)],
            ..Default::default()
        };

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Medicine,
            &view,
            &recipes(),
            &holdings(&[(CommodityKind::Medicine, 1)]),
            &holdings(&[(CommodityKind::Medicine, 1)]),
        );

        assert_eq!(breakdown.treatment_score, 1000);
        assert_eq!(breakdown.direct_survival_score, 0);
    }

    #[test]
    fn commodity_opportunity_treatment_score_zero_without_wounds() {
        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Medicine,
            &StubBeliefView::default(),
            &recipes(),
            &holdings(&[(CommodityKind::Medicine, 3)]),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.treatment_score, 0);
    }

    #[test]
    fn commodity_opportunity_enterprise_score_tracks_remembered_demand() {
        let view = StubBeliefView {
            demand_memory: vec![
                demand_observation(CommodityKind::Bread, 3),
                demand_observation(CommodityKind::Apple, 4),
            ],
            ..Default::default()
        };

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Bread,
            &view,
            &recipes(),
            &holdings(&[(CommodityKind::Bread, 1)]),
            &holdings(&[(CommodityKind::Bread, 1)]),
        );

        assert_eq!(breakdown.enterprise_score, 2);
    }

    #[test]
    fn commodity_opportunity_indirect_recipe_score_is_stub_zero() {
        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Grain,
            &StubBeliefView::default(),
            &recipes(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.indirect_recipe_score, 0);
    }

    #[test]
    fn commodity_opportunity_is_deterministic_for_identical_inputs() {
        let view = StubBeliefView {
            needs: Some(HomeostaticNeeds::new(
                Permille::new(400).unwrap(),
                Permille::new(150).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            )),
            wounds: vec![wound(250)],
            demand_memory: vec![demand_observation(CommodityKind::Bread, 2)],
        };
        let held = holdings(&[(CommodityKind::Bread, 1), (CommodityKind::Medicine, 1)]);
        let alternatives = holdings(&[(CommodityKind::Bread, 1)]);
        let recipes = recipes();

        let first = commodity_opportunity_score(
            actor(),
            CommodityKind::Bread,
            &view,
            &recipes,
            &held,
            &alternatives,
        );
        let second = commodity_opportunity_score(
            actor(),
            CommodityKind::Bread,
            &view,
            &recipes,
            &held,
            &alternatives,
        );

        assert_eq!(first, second);
        assert_eq!(
            first,
            CommodityOpportunityBreakdown {
                direct_survival_score: 400,
                treatment_score: 0,
                enterprise_score: 2,
                indirect_recipe_score: 0,
            }
        );
    }
}
