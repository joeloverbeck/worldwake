use worldwake_sim::DurationExpr;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PlannerDurationDependency {
    TargetConsumable,
    ActorMetabolism,
    ActorTradeDisposition,
    ActorTheftDisposition,
    ActorInvestigationDisposition,
    ActorVerificationDisposition,
    ActorWitnessQueryDisposition,
    ActorDefendStance,
    CombatWeapon,
    TargetTreatment,
    ConsultRecord,
    TravelToTarget,
}

pub const PLANNER_DURATION_DEPENDENCIES: [PlannerDurationDependency; 12] = [
    PlannerDurationDependency::TargetConsumable,
    PlannerDurationDependency::ActorMetabolism,
    PlannerDurationDependency::ActorTradeDisposition,
    PlannerDurationDependency::ActorTheftDisposition,
    PlannerDurationDependency::ActorInvestigationDisposition,
    PlannerDurationDependency::ActorVerificationDisposition,
    PlannerDurationDependency::ActorWitnessQueryDisposition,
    PlannerDurationDependency::ActorDefendStance,
    PlannerDurationDependency::CombatWeapon,
    PlannerDurationDependency::TargetTreatment,
    PlannerDurationDependency::ConsultRecord,
    PlannerDurationDependency::TravelToTarget,
];

impl PlannerDurationDependency {
    pub const fn all() -> &'static [Self] {
        &PLANNER_DURATION_DEPENDENCIES
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::TargetConsumable => "TargetConsumable",
            Self::ActorMetabolism => "ActorMetabolism",
            Self::ActorTradeDisposition => "ActorTradeDisposition",
            Self::ActorTheftDisposition => "ActorTheftDisposition",
            Self::ActorInvestigationDisposition => "ActorInvestigationDisposition",
            Self::ActorVerificationDisposition => "ActorVerificationDisposition",
            Self::ActorWitnessQueryDisposition => "ActorWitnessQueryDisposition",
            Self::ActorDefendStance => "ActorDefendStance",
            Self::CombatWeapon => "CombatWeapon",
            Self::TargetTreatment => "TargetTreatment",
            Self::ConsultRecord => "ConsultRecord",
            Self::TravelToTarget => "TravelToTarget",
        }
    }

    pub const fn from_duration_expr(duration: DurationExpr) -> Option<Self> {
        match duration {
            DurationExpr::TargetConsumable { .. } => Some(Self::TargetConsumable),
            DurationExpr::ActorMetabolism { .. } => Some(Self::ActorMetabolism),
            DurationExpr::ActorTradeDisposition => Some(Self::ActorTradeDisposition),
            DurationExpr::ActorTheftDisposition => Some(Self::ActorTheftDisposition),
            DurationExpr::ActorInvestigationDisposition => {
                Some(Self::ActorInvestigationDisposition)
            }
            DurationExpr::ActorVerificationDisposition => {
                Some(Self::ActorVerificationDisposition)
            }
            DurationExpr::ActorWitnessQueryDisposition => {
                Some(Self::ActorWitnessQueryDisposition)
            }
            DurationExpr::Fixed(_) => None,
            DurationExpr::ActorDefendStance => Some(Self::ActorDefendStance),
            DurationExpr::CombatWeapon => Some(Self::CombatWeapon),
            DurationExpr::TargetTreatment { .. } => Some(Self::TargetTreatment),
            DurationExpr::ConsultRecord { .. } => Some(Self::ConsultRecord),
            DurationExpr::TravelToTarget { .. } => Some(Self::TravelToTarget),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PlannerDurationDependency;
    use crate::build_semantics_table;
    use std::collections::BTreeSet;
    use worldwake_sim::RecipeRegistry;
    use worldwake_systems::build_full_action_registries;

    #[test]
    fn planner_duration_inventory_matches_live_non_fixed_planner_surface() {
        let recipes = RecipeRegistry::new();
        let registries = build_full_action_registries(&recipes).unwrap();
        let semantics = build_semantics_table(&registries.defs);
        let mut observed = BTreeSet::new();

        for def_id in semantics.keys() {
            let def = registries
                .defs
                .get(*def_id)
                .expect("planner semantics should reference a registered action def");
            if def.duration.fixed_ticks().is_some() {
                continue;
            }

            let dependency = PlannerDurationDependency::from_duration_expr(def.duration)
                .unwrap_or_else(|| {
                    panic!(
                        "planner action '{}' uses unmapped non-fixed duration {:?}",
                        def.name, def.duration
                    )
                });
            observed.insert(dependency);
        }

        let expected = PlannerDurationDependency::all()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(observed, expected);
    }
}
