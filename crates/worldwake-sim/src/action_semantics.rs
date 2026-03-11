use crate::{ActionDuration, ActionPayload};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use worldwake_core::{
    CombatWeaponRef, CommodityKind, CommodityTreatmentProfile, EntityId, EntityKind, Quantity,
    RecipeId, UniqueItemKind, WorkstationTag, World,
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum Constraint {
    ActorAlive,
    ActorNotIncapacitated,
    ActorNotDead,
    ActorHasControl,
    ActorNotInTransit,
    ActorAtPlace(EntityId),
    ActorKnowsRecipe(RecipeId),
    ActorHasUniqueItemKind {
        kind: UniqueItemKind,
        min_count: u32,
    },
    ActorHasCommodity {
        kind: CommodityKind,
        min_qty: Quantity,
    },
    ActorKind(EntityKind),
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum TargetSpec {
    SpecificEntity(EntityId),
    EntityAtActorPlace { kind: EntityKind },
    EntityDirectlyPossessedByActor { kind: EntityKind },
    AdjacentPlace,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum Precondition {
    ActorAlive,
    ActorCanControlTarget(u8),
    TargetExists(u8),
    TargetAlive(u8),
    TargetDead(u8),
    TargetIsAgent(u8),
    TargetAtActorPlace(u8),
    TargetAdjacentToActor(u8),
    TargetKind {
        target_index: u8,
        kind: EntityKind,
    },
    TargetCommodity {
        target_index: u8,
        kind: CommodityKind,
    },
    TargetHasWorkstationTag {
        target_index: u8,
        tag: WorkstationTag,
    },
    TargetHasResourceSource {
        target_index: u8,
        commodity: CommodityKind,
        min_available: Quantity,
    },
    TargetNotInContainer(u8),
    TargetUnpossessed(u8),
    TargetDirectlyPossessedByActor(u8),
    TargetLacksProductionJob(u8),
    TargetHasConsumableEffect {
        target_index: u8,
        effect: ConsumableEffect,
    },
    TargetHasWounds(u8),
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct ReservationReq {
    pub target_index: u8,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum ConsumableEffect {
    Hunger,
    Thirst,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum MetabolismDurationKind {
    Toilet,
    Wash,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum DurationExpr {
    Fixed(NonZeroU32),
    TargetConsumable { target_index: u8 },
    TravelToTarget { target_index: u8 },
    ActorMetabolism { kind: MetabolismDurationKind },
    ActorTradeDisposition,
    Indefinite,
    CombatWeapon,
    TargetTreatment {
        target_index: u8,
        commodity: CommodityKind,
    },
}

impl DurationExpr {
    #[must_use]
    pub const fn fixed_ticks(self) -> Option<u32> {
        match self {
            Self::Fixed(ticks) => Some(ticks.get()),
            Self::TargetConsumable { .. }
            | Self::TravelToTarget { .. }
            | Self::ActorMetabolism { .. }
            | Self::ActorTradeDisposition
            | Self::Indefinite
            | Self::CombatWeapon
            | Self::TargetTreatment { .. } => None,
        }
    }

    pub fn resolve_for(
        self,
        world: &World,
        actor: EntityId,
        targets: &[EntityId],
        payload: &ActionPayload,
    ) -> Result<ActionDuration, String> {
        match self {
            Self::Fixed(ticks) => Ok(ActionDuration::Finite(ticks.get())),
            Self::TargetConsumable { target_index } => {
                let target = targets
                    .get(usize::from(target_index))
                    .copied()
                    .ok_or_else(|| format!("missing target at index {target_index}"))?;
                let lot = world
                    .get_component_item_lot(target)
                    .ok_or_else(|| format!("target {target} is not an item lot"))?;
                let profile = lot
                    .commodity
                    .spec()
                    .consumable_profile
                    .ok_or_else(|| format!("target {target} commodity is not consumable"))?;
                Ok(ActionDuration::Finite(
                    profile.consumption_ticks_per_unit.get(),
                ))
            }
            Self::TravelToTarget { target_index } => {
                let target = targets
                    .get(usize::from(target_index))
                    .copied()
                    .ok_or_else(|| format!("missing target at index {target_index}"))?;
                let origin = world
                    .effective_place(actor)
                    .ok_or_else(|| format!("actor {actor} has no effective place"))?;
                let edge = world
                    .topology()
                    .unique_direct_edge(origin, target)
                    .map_err(|err| err.to_string())?
                    .ok_or_else(|| {
                        format!("no directed travel edge connects {origin} -> {target}")
                    })?;
                Ok(ActionDuration::Finite(edge.travel_time_ticks()))
            }
            Self::ActorMetabolism { kind } => {
                let profile = world
                    .get_component_metabolism_profile(actor)
                    .ok_or_else(|| format!("actor {actor} lacks metabolism profile"))?;
                let ticks = match kind {
                    MetabolismDurationKind::Toilet => profile.toilet_ticks.get(),
                    MetabolismDurationKind::Wash => profile.wash_ticks.get(),
                };
                Ok(ActionDuration::Finite(ticks))
            }
            Self::ActorTradeDisposition => world
                .get_component_trade_disposition_profile(actor)
                .map(|profile| ActionDuration::Finite(profile.negotiation_round_ticks.get()))
                .ok_or_else(|| format!("actor {actor} lacks trade disposition profile")),
            Self::Indefinite => Ok(ActionDuration::Indefinite),
            Self::CombatWeapon => {
                let combat = payload.as_combat().ok_or_else(|| {
                    "combat weapon duration requires ActionPayload::Combat".to_string()
                })?;
                match combat.weapon {
                    CombatWeaponRef::Unarmed => world
                        .get_component_combat_profile(actor)
                        .map(|profile| ActionDuration::Finite(profile.unarmed_attack_ticks.get()))
                        .ok_or_else(|| format!("actor {actor} lacks combat profile")),
                    CombatWeaponRef::Commodity(kind) => kind
                        .spec()
                        .combat_weapon_profile
                        .map(|profile| ActionDuration::Finite(profile.attack_duration_ticks.get()))
                        .ok_or_else(|| format!("commodity {kind:?} is not a combat weapon")),
                }
            }
            Self::TargetTreatment {
                target_index,
                commodity,
            } => {
                if world.controlled_commodity_quantity(actor, commodity) == Quantity(0) {
                    return Err(format!("actor {actor} lacks treatment commodity {commodity:?}"));
                }
                let target = targets
                    .get(usize::from(target_index))
                    .copied()
                    .ok_or_else(|| format!("missing target at index {target_index}"))?;
                let wounds = world
                    .get_component_wound_list(target)
                    .ok_or_else(|| format!("target {target} lacks wounds"))?;
                let CommodityTreatmentProfile {
                    treatment_ticks_per_unit,
                    severity_reduction_per_tick,
                    ..
                } = commodity
                    .spec()
                    .treatment_profile
                    .ok_or_else(|| format!("commodity {commodity:?} has no treatment profile"))?;
                if wounds.wounds.is_empty() {
                    return Err(format!("target {target} has no wounds"));
                }

                let severity_per_tick = u32::from(severity_reduction_per_tick.value()).max(1);
                let wound_ticks = wounds.wound_load().div_ceil(severity_per_tick).max(1);
                Ok(ActionDuration::Finite(
                    treatment_ticks_per_unit.get().max(wound_ticks),
                ))
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum Interruptibility {
    NonInterruptible,
    InterruptibleWithPenalty,
    FreelyInterruptible,
}

#[cfg(test)]
mod tests {
    use super::{
        Constraint, ConsumableEffect, DurationExpr, Interruptibility, MetabolismDurationKind,
        Precondition, ReservationReq, TargetSpec,
    };
    use crate::{ActionDuration, ActionPayload, CombatActionPayload, TradeActionPayload};
    use serde::{de::DeserializeOwned, Serialize};
    use std::mem;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, CauseRef, CombatProfile, CombatWeaponRef, CommodityKind,
        ControlSource, EntityId, EntityKind, EventLog, HomeostaticNeeds, MetabolismProfile,
        Permille, Quantity, RecipeId, Tick, TradeDispositionProfile, UniqueItemKind,
        VisibilitySpec, WitnessData, WorkstationTag, World, WorldTxn,
    };

    const ENTITY_A: EntityId = EntityId {
        slot: 7,
        generation: 1,
    };
    const ENTITY_B: EntityId = EntityId {
        slot: 9,
        generation: 2,
    };

    const ALL_CONSTRAINTS: [Constraint; 10] = [
        Constraint::ActorAlive,
        Constraint::ActorNotIncapacitated,
        Constraint::ActorNotDead,
        Constraint::ActorHasControl,
        Constraint::ActorNotInTransit,
        Constraint::ActorAtPlace(ENTITY_A),
        Constraint::ActorKnowsRecipe(RecipeId(3)),
        Constraint::ActorHasUniqueItemKind {
            kind: UniqueItemKind::SimpleTool,
            min_count: 1,
        },
        Constraint::ActorHasCommodity {
            kind: CommodityKind::Bread,
            min_qty: Quantity(3),
        },
        Constraint::ActorKind(EntityKind::Agent),
    ];

    const ALL_TARGET_SPECS: [TargetSpec; 4] = [
        TargetSpec::SpecificEntity(ENTITY_B),
        TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Facility,
        },
        TargetSpec::EntityDirectlyPossessedByActor {
            kind: EntityKind::ItemLot,
        },
        TargetSpec::AdjacentPlace,
    ];

    const ALL_PRECONDITIONS: [Precondition; 18] = [
        Precondition::ActorAlive,
        Precondition::ActorCanControlTarget(6),
        Precondition::TargetExists(0),
        Precondition::TargetAlive(8),
        Precondition::TargetDead(9),
        Precondition::TargetIsAgent(10),
        Precondition::TargetAtActorPlace(1),
        Precondition::TargetAdjacentToActor(7),
        Precondition::TargetKind {
            target_index: 2,
            kind: EntityKind::Container,
        },
        Precondition::TargetCommodity {
            target_index: 3,
            kind: CommodityKind::Water,
        },
        Precondition::TargetHasWorkstationTag {
            target_index: 1,
            tag: WorkstationTag::Mill,
        },
        Precondition::TargetHasResourceSource {
            target_index: 2,
            commodity: CommodityKind::Apple,
            min_available: Quantity(2),
        },
        Precondition::TargetNotInContainer(4),
        Precondition::TargetUnpossessed(5),
        Precondition::TargetDirectlyPossessedByActor(6),
        Precondition::TargetLacksProductionJob(3),
        Precondition::TargetHasConsumableEffect {
            target_index: 4,
            effect: ConsumableEffect::Thirst,
        },
        Precondition::TargetHasWounds(2),
    ];

    const ALL_RESERVATION_REQS: [ReservationReq; 2] = [
        ReservationReq { target_index: 0 },
        ReservationReq { target_index: 3 },
    ];

    const ALL_DURATION_EXPRS: [DurationExpr; 9] = [
        DurationExpr::Fixed(NonZeroU32::MIN),
        DurationExpr::Fixed(NonZeroU32::new(5).unwrap()),
        DurationExpr::TargetConsumable { target_index: 0 },
        DurationExpr::TravelToTarget { target_index: 1 },
        DurationExpr::ActorMetabolism {
            kind: MetabolismDurationKind::Wash,
        },
        DurationExpr::ActorTradeDisposition,
        DurationExpr::Indefinite,
        DurationExpr::CombatWeapon,
        DurationExpr::TargetTreatment {
            target_index: 2,
            commodity: CommodityKind::Medicine,
        },
    ];

    const ALL_INTERRUPTIBILITY: [Interruptibility; 3] = [
        Interruptibility::NonInterruptible,
        Interruptibility::InterruptibleWithPenalty,
        Interruptibility::FreelyInterruptible,
    ];

    fn assert_traits<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn action_semantic_types_satisfy_required_traits() {
        assert_traits::<Constraint>();
        assert_traits::<TargetSpec>();
        assert_traits::<Precondition>();
        assert_traits::<ReservationReq>();
        assert_traits::<DurationExpr>();
        assert_traits::<Interruptibility>();
        assert_traits::<ConsumableEffect>();
        assert_traits::<MetabolismDurationKind>();
    }

    #[test]
    fn fixed_duration_expr_exposes_embedded_ticks() {
        assert_eq!(DurationExpr::Fixed(NonZeroU32::MIN).fixed_ticks(), Some(1));
        assert_eq!(
            DurationExpr::Fixed(NonZeroU32::new(5).unwrap()).fixed_ticks(),
            Some(5)
        );
        assert_eq!(
            DurationExpr::TargetConsumable { target_index: 0 }.fixed_ticks(),
            None
        );
        assert_eq!(
            DurationExpr::TravelToTarget { target_index: 0 }.fixed_ticks(),
            None
        );
        assert_eq!(
            DurationExpr::ActorMetabolism {
                kind: MetabolismDurationKind::Toilet,
            }
            .fixed_ticks(),
            None
        );
        assert_eq!(DurationExpr::ActorTradeDisposition.fixed_ticks(), None);
        assert_eq!(DurationExpr::Indefinite.fixed_ticks(), None);
        assert_eq!(DurationExpr::CombatWeapon.fixed_ticks(), None);
        assert_eq!(
            DurationExpr::TargetTreatment {
                target_index: 0,
                commodity: CommodityKind::Medicine,
            }
            .fixed_ticks(),
            None
        );
    }

    #[test]
    fn zero_duration_is_unrepresentable() {
        assert!(NonZeroU32::new(0).is_none());
    }

    #[test]
    fn constraint_bincode_roundtrip_covers_every_variant() {
        for constraint in ALL_CONSTRAINTS {
            let bytes = bincode::serialize(&constraint).unwrap();
            let roundtrip: Constraint = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, constraint);
        }
    }

    #[test]
    fn target_spec_bincode_roundtrip_covers_every_variant() {
        for spec in ALL_TARGET_SPECS {
            let bytes = bincode::serialize(&spec).unwrap();
            let roundtrip: TargetSpec = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, spec);
        }
    }

    #[test]
    fn precondition_bincode_roundtrip_covers_every_variant() {
        for precondition in ALL_PRECONDITIONS {
            let bytes = bincode::serialize(&precondition).unwrap();
            let roundtrip: Precondition = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, precondition);
        }
    }

    #[test]
    fn reservation_req_bincode_roundtrip_covers_every_variant() {
        for req in ALL_RESERVATION_REQS {
            let bytes = bincode::serialize(&req).unwrap();
            let roundtrip: ReservationReq = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, req);
        }
    }

    #[test]
    fn duration_expr_bincode_roundtrip_covers_every_variant() {
        for expr in ALL_DURATION_EXPRS {
            let bytes = bincode::serialize(&expr).unwrap();
            let roundtrip: DurationExpr = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, expr);
        }
    }

    #[test]
    fn interruptibility_bincode_roundtrip_covers_every_variant() {
        for interruptibility in ALL_INTERRUPTIBILITY {
            let bytes = bincode::serialize(&interruptibility).unwrap();
            let roundtrip: Interruptibility = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, interruptibility);
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    #[test]
    fn duration_expr_resolves_consumable_and_metabolism_driven_ticks() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, target) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_component_metabolism_profile(
                actor,
                MetabolismProfile::new(
                    pm(1),
                    pm(1),
                    pm(1),
                    pm(1),
                    pm(1),
                    pm(10),
                    NonZeroU32::new(5).unwrap(),
                    NonZeroU32::new(5).unwrap(),
                    NonZeroU32::new(5).unwrap(),
                    NonZeroU32::new(5).unwrap(),
                    NonZeroU32::new(7).unwrap(),
                    NonZeroU32::new(9).unwrap(),
                ),
            )
            .unwrap();
            txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::default())
                .unwrap();
            let target = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            commit_txn(txn);
            (actor, target)
        };

        assert_eq!(
            DurationExpr::TargetConsumable { target_index: 0 }
                .resolve_for(&world, actor, &[target], &ActionPayload::None)
                .unwrap(),
            ActionDuration::Finite(
                CommodityKind::Bread
                    .spec()
                    .consumable_profile
                    .unwrap()
                    .consumption_ticks_per_unit
                    .get()
            )
        );
        assert_eq!(
            DurationExpr::ActorMetabolism {
                kind: MetabolismDurationKind::Toilet,
            }
            .resolve_for(&world, actor, &[target], &ActionPayload::None)
            .unwrap(),
            ActionDuration::Finite(7)
        );
        assert_eq!(
            DurationExpr::ActorMetabolism {
                kind: MetabolismDurationKind::Wash,
            }
            .resolve_for(&world, actor, &[target], &ActionPayload::None)
            .unwrap(),
            ActionDuration::Finite(9)
        );
    }

    #[test]
    fn duration_expr_resolves_travel_ticks_from_directed_edge() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, places[0]).unwrap();
            commit_txn(txn);
            actor
        };
        let destination = world.topology().neighbors(places[0])[0];
        let expected = world
            .topology()
            .unique_direct_edge(places[0], destination)
            .unwrap()
            .unwrap()
            .travel_time_ticks();

        assert_eq!(
            DurationExpr::TravelToTarget { target_index: 0 }
                .resolve_for(&world, actor, &[destination], &ActionPayload::None)
                .unwrap(),
            ActionDuration::Finite(expected)
        );
    }

    #[test]
    fn duration_expr_resolves_trade_and_combat_driven_ticks_from_authoritative_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_component_trade_disposition_profile(
                actor,
                TradeDispositionProfile {
                    negotiation_round_ticks: nz(11),
                    initial_offer_bias: pm(40),
                    concession_rate: pm(25),
                    demand_memory_retention_ticks: 180,
                },
            )
            .unwrap();
            txn.set_component_combat_profile(
                actor,
                CombatProfile::new(
                    pm(1000),
                    pm(700),
                    pm(600),
                    pm(550),
                    pm(75),
                    pm(20),
                    pm(15),
                    pm(120),
                    pm(30),
                    nz(6),
                ),
            )
            .unwrap();
            commit_txn(txn);
            actor
        };

        assert_eq!(
            DurationExpr::ActorTradeDisposition
                .resolve_for(&world, actor, &[], &ActionPayload::None)
                .unwrap(),
            ActionDuration::Finite(11)
        );
        assert_eq!(
            DurationExpr::CombatWeapon
                .resolve_for(
                    &world,
                    actor,
                    &[],
                    &ActionPayload::Combat(CombatActionPayload {
                        target: ENTITY_B,
                        weapon: CombatWeaponRef::Unarmed,
                    }),
                )
                .unwrap(),
            ActionDuration::Finite(6)
        );
        assert_eq!(
            DurationExpr::CombatWeapon
                .resolve_for(
                    &world,
                    actor,
                    &[],
                    &ActionPayload::Combat(CombatActionPayload {
                        target: ENTITY_B,
                        weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
                    }),
                )
                .unwrap(),
            ActionDuration::Finite(
                CommodityKind::Sword
                    .spec()
                    .combat_weapon_profile
                    .unwrap()
                    .attack_duration_ticks
                    .get()
            )
        );
    }

    #[test]
    fn duration_expr_reports_clear_errors_for_invalid_dynamic_durations() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            actor
        };

        assert_eq!(
            DurationExpr::Indefinite
                .resolve_for(&world, actor, &[], &ActionPayload::None)
                .unwrap(),
            ActionDuration::Indefinite
        );
        assert_eq!(
            DurationExpr::CombatWeapon
                .resolve_for(
                    &world,
                    actor,
                    &[],
                    &ActionPayload::Trade(TradeActionPayload {
                        counterparty: ENTITY_B,
                        offered_commodity: CommodityKind::Bread,
                        offered_quantity: Quantity(1),
                        requested_commodity: CommodityKind::Water,
                        requested_quantity: Quantity(1),
                    }),
                )
                .unwrap_err(),
            "combat weapon duration requires ActionPayload::Combat"
        );
        assert_eq!(
            DurationExpr::CombatWeapon
                .resolve_for(
                    &world,
                    actor,
                    &[],
                    &ActionPayload::Combat(CombatActionPayload {
                        target: ENTITY_B,
                        weapon: CombatWeaponRef::Unarmed,
                    }),
                )
                .unwrap_err(),
            format!("actor {actor} lacks combat profile")
        );
        assert_eq!(
            DurationExpr::CombatWeapon
                .resolve_for(
                    &world,
                    actor,
                    &[],
                    &ActionPayload::Combat(CombatActionPayload {
                        target: ENTITY_B,
                        weapon: CombatWeaponRef::Commodity(CommodityKind::Bread),
                    }),
                )
                .unwrap_err(),
            "commodity Bread is not a combat weapon"
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn target_and_precondition_indices_use_fixed_width_integers() {
        let reservation = ReservationReq { target_index: 4 };
        let _: u8 = reservation.target_index;

        match Precondition::TargetExists(2) {
            Precondition::TargetExists(index) => {
                let _: u8 = index;
            }
            _ => unreachable!(),
        }

        match Precondition::TargetAlive(3) {
            Precondition::TargetAlive(index) => {
                let _: u8 = index;
            }
            _ => unreachable!(),
        }

        match Precondition::TargetDead(4) {
            Precondition::TargetDead(index) => {
                let _: u8 = index;
            }
            _ => unreachable!(),
        }

        match Precondition::TargetIsAgent(5) {
            Precondition::TargetIsAgent(index) => {
                let _: u8 = index;
            }
            _ => unreachable!(),
        }

        match Precondition::ActorCanControlTarget(7) {
            Precondition::ActorCanControlTarget(index) => {
                let _: u8 = index;
            }
            _ => unreachable!(),
        }

        match Precondition::TargetAtActorPlace(3) {
            Precondition::TargetAtActorPlace(index) => {
                let _: u8 = index;
            }
            _ => unreachable!(),
        }

        match Precondition::TargetAdjacentToActor(4) {
            Precondition::TargetAdjacentToActor(index) => {
                let _: u8 = index;
            }
            _ => unreachable!(),
        }

        match (Precondition::TargetKind {
            target_index: 5,
            kind: EntityKind::Rumor,
        }) {
            Precondition::TargetKind { target_index, .. } => {
                let _: u8 = target_index;
            }
            _ => unreachable!(),
        }

        match (Precondition::TargetCommodity {
            target_index: 8,
            kind: CommodityKind::Apple,
        }) {
            Precondition::TargetCommodity { target_index, .. } => {
                let _: u8 = target_index;
            }
            _ => unreachable!(),
        }

        match (Precondition::TargetHasConsumableEffect {
            target_index: 9,
            effect: ConsumableEffect::Hunger,
        }) {
            Precondition::TargetHasConsumableEffect { target_index, .. } => {
                let _: u8 = target_index;
            }
            _ => unreachable!(),
        }

        match Precondition::TargetNotInContainer(10) {
            Precondition::TargetNotInContainer(target_index) => {
                let _: u8 = target_index;
            }
            _ => unreachable!(),
        }

        match Precondition::TargetUnpossessed(11) {
            Precondition::TargetUnpossessed(target_index) => {
                let _: u8 = target_index;
            }
            _ => unreachable!(),
        }

        match Precondition::TargetDirectlyPossessedByActor(12) {
            Precondition::TargetDirectlyPossessedByActor(target_index) => {
                let _: u8 = target_index;
            }
            _ => unreachable!(),
        }

        match Precondition::TargetLacksProductionJob(13) {
            Precondition::TargetLacksProductionJob(target_index) => {
                let _: u8 = target_index;
            }
            _ => unreachable!(),
        }

        match Precondition::TargetHasWounds(14) {
            Precondition::TargetHasWounds(target_index) => {
                let _: u8 = target_index;
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn serialized_indices_are_single_byte_fields() {
        assert_eq!(mem::size_of::<u8>(), 1);
        assert_eq!(mem::size_of::<ReservationReq>(), 1);
    }
}
