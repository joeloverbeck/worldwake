use crate::GoalPriorityClass;
use worldwake_core::{EntityId, Permille, ThresholdBand};
use worldwake_sim::BeliefView;

pub fn derive_pain_pressure(view: &dyn BeliefView, agent: EntityId) -> Permille {
    view.wounds(agent)
        .into_iter()
        .fold(Permille::new_unchecked(0), |pressure, wound| {
            pressure.saturating_add(wound.severity)
        })
}

pub fn derive_danger_pressure(view: &dyn BeliefView, agent: EntityId) -> Permille {
    let Some(thresholds) = view.drive_thresholds(agent) else {
        return Permille::new_unchecked(0);
    };

    let attackers = view.current_attackers_of(agent);
    let hostiles = view.visible_hostiles_for(agent);

    if attackers.is_empty() && hostiles.is_empty() {
        Permille::new_unchecked(0)
    } else if attackers.len() >= 2
        || (!attackers.is_empty() && (view.has_wounds(agent) || view.is_incapacitated(agent)))
    {
        thresholds.danger.critical()
    } else if !attackers.is_empty() {
        thresholds.danger.high()
    } else {
        thresholds.danger.medium()
    }
}

pub fn classify_band(value: Permille, band: &ThresholdBand) -> GoalPriorityClass {
    if value >= band.critical() {
        GoalPriorityClass::Critical
    } else if value >= band.high() {
        GoalPriorityClass::High
    } else if value >= band.medium() {
        GoalPriorityClass::Medium
    } else if value >= band.low() {
        GoalPriorityClass::Low
    } else {
        GoalPriorityClass::Background
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_band, derive_danger_pressure, derive_pain_pressure};
    use crate::GoalPriorityClass;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BodyPart, CombatProfile, CommodityConsumableProfile, CommodityKind, DemandObservation,
        DeprivationKind, DriveThresholds, EntityId, EntityKind, HomeostaticNeeds, InTransitOnEdge,
        LoadUnits, MerchandiseProfile, MetabolismProfile, Permille, Quantity, RecipeId,
        ResourceSource, ThresholdBand, Tick, TickRange, TradeDispositionProfile, UniqueItemKind,
        WorkstationTag, Wound, WoundCause, WoundId,
    };
    use worldwake_sim::{ActionDuration, ActionPayload, BeliefView, DurationExpr};

    #[derive(Default)]
    struct TestBeliefView {
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
        incapacitated: BTreeSet<EntityId>,
    }

    impl BeliefView for TestBeliefView {
        fn is_alive(&self, _entity: EntityId) -> bool {
            true
        }
        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }
        fn effective_place(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }
        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
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
        fn controlled_commodity_quantity_at_place(
            &self,
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }
        fn local_controlled_lots_for(
            &self,
            _actor: EntityId,
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
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }
        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }
        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }
        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }
        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }
        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }
        fn is_dead(&self, _entity: EntityId) -> bool {
            false
        }
        fn is_incapacitated(&self, entity: EntityId) -> bool {
            self.incapacitated.contains(&entity)
        }
        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }
        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.thresholds.get(&agent).copied()
        }
        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }
        fn wounds(&self, agent: EntityId) -> Vec<Wound> {
            self.wounds.get(&agent).cloned().unwrap_or_default()
        }
        fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
            self.hostiles.get(&agent).cloned().unwrap_or_default()
        }
        fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
            self.attackers.get(&agent).cloned().unwrap_or_default()
        }
        fn agents_selling_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
            Vec::new()
        }
        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
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
        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }
        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
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

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn wound(severity: u16) -> Wound {
        Wound {
            id: WoundId(u64::from(severity)),
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(DeprivationKind::Starvation),
            severity: pm(severity),
            inflicted_at: Tick(1),
            bleed_rate_per_tick: pm(0),
        }
    }

    #[test]
    fn pain_pressure_is_zero_without_wounds() {
        assert_eq!(
            derive_pain_pressure(&TestBeliefView::default(), entity(1)),
            pm(0)
        );
    }

    #[test]
    fn pain_pressure_sums_wound_severity_and_caps_at_one_thousand() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.wounds.insert(agent, vec![wound(300), wound(300)]);
        assert_eq!(derive_pain_pressure(&view, agent), pm(600));

        view.wounds.insert(agent, vec![wound(700), wound(450)]);
        assert_eq!(derive_pain_pressure(&view, agent), pm(1000));
    }

    #[test]
    fn danger_pressure_is_zero_without_thresholds_or_threats() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        assert_eq!(derive_danger_pressure(&view, agent), pm(0));

        view.thresholds.insert(agent, DriveThresholds::default());
        assert_eq!(derive_danger_pressure(&view, agent), pm(0));
    }

    #[test]
    fn danger_pressure_uses_threshold_bands_monotonically() {
        let agent = entity(1);
        let attacker_a = entity(10);
        let attacker_b = entity(11);
        let thresholds = DriveThresholds::default();
        let mut view = TestBeliefView::default();
        view.thresholds.insert(agent, thresholds);

        view.hostiles.insert(agent, vec![attacker_a]);
        assert_eq!(
            derive_danger_pressure(&view, agent),
            thresholds.danger.medium()
        );

        view.hostiles.clear();
        view.attackers.insert(agent, vec![attacker_a]);
        assert_eq!(
            derive_danger_pressure(&view, agent),
            thresholds.danger.high()
        );

        view.attackers.insert(agent, vec![attacker_a, attacker_b]);
        assert_eq!(
            derive_danger_pressure(&view, agent),
            thresholds.danger.critical()
        );
    }

    #[test]
    fn danger_pressure_promotes_single_attacker_when_wounded_or_incapacitated() {
        let agent = entity(1);
        let attacker = entity(10);
        let thresholds = DriveThresholds::default();
        let mut wounded_view = TestBeliefView::default();
        wounded_view.thresholds.insert(agent, thresholds);
        wounded_view.attackers.insert(agent, vec![attacker]);
        wounded_view.wounds.insert(agent, vec![wound(50)]);

        assert_eq!(
            derive_danger_pressure(&wounded_view, agent),
            thresholds.danger.critical()
        );

        let mut incapacitated_view = TestBeliefView::default();
        incapacitated_view.thresholds.insert(agent, thresholds);
        incapacitated_view.attackers.insert(agent, vec![attacker]);
        incapacitated_view.incapacitated.insert(agent);

        assert_eq!(
            derive_danger_pressure(&incapacitated_view, agent),
            thresholds.danger.critical()
        );
    }

    #[test]
    fn classify_band_maps_threshold_ranges_to_priority_classes() {
        let band = ThresholdBand::new(pm(100), pm(300), pm(600), pm(850)).unwrap();

        assert_eq!(classify_band(pm(0), &band), GoalPriorityClass::Background);
        assert_eq!(classify_band(pm(100), &band), GoalPriorityClass::Low);
        assert_eq!(classify_band(pm(300), &band), GoalPriorityClass::Medium);
        assert_eq!(classify_band(pm(600), &band), GoalPriorityClass::High);
        assert_eq!(classify_band(pm(850), &band), GoalPriorityClass::Critical);
    }
}
