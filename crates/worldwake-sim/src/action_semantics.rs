use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use worldwake_core::{CommodityKind, EntityId, EntityKind, Quantity, World};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum Constraint {
    ActorAlive,
    ActorHasControl,
    ActorAtPlace(EntityId),
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
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum Precondition {
    ActorAlive,
    ActorCanControlTarget(u8),
    TargetExists(u8),
    TargetAtActorPlace(u8),
    TargetKind { target_index: u8, kind: EntityKind },
    TargetCommodity {
        target_index: u8,
        kind: CommodityKind,
    },
    TargetHasConsumableEffect {
        target_index: u8,
        effect: ConsumableEffect,
    },
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
    ActorMetabolism { kind: MetabolismDurationKind },
}

impl DurationExpr {
    #[must_use]
    pub const fn fixed_ticks(self) -> Option<u32> {
        match self {
            Self::Fixed(ticks) => Some(ticks.get()),
            Self::TargetConsumable { .. } | Self::ActorMetabolism { .. } => None,
        }
    }

    pub fn resolve_for(
        self,
        world: &World,
        actor: EntityId,
        targets: &[EntityId],
    ) -> Result<u32, String> {
        match self {
            Self::Fixed(ticks) => Ok(ticks.get()),
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
                Ok(profile.consumption_ticks_per_unit.get())
            }
            Self::ActorMetabolism { kind } => {
                let profile = world
                    .get_component_metabolism_profile(actor)
                    .ok_or_else(|| format!("actor {actor} lacks metabolism profile"))?;
                let ticks = match kind {
                    MetabolismDurationKind::Toilet => profile.toilet_ticks.get(),
                    MetabolismDurationKind::Wash => profile.wash_ticks.get(),
                };
                Ok(ticks)
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
    use serde::{de::DeserializeOwned, Serialize};
    use std::mem;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, ControlSource, EntityId, EntityKind,
        EventLog, HomeostaticNeeds, MetabolismProfile, Permille, Quantity, Tick, VisibilitySpec,
        WitnessData, World, WorldTxn,
    };

    const ENTITY_A: EntityId = EntityId {
        slot: 7,
        generation: 1,
    };
    const ENTITY_B: EntityId = EntityId {
        slot: 9,
        generation: 2,
    };

    const ALL_CONSTRAINTS: [Constraint; 5] = [
        Constraint::ActorAlive,
        Constraint::ActorHasControl,
        Constraint::ActorAtPlace(ENTITY_A),
        Constraint::ActorHasCommodity {
            kind: CommodityKind::Bread,
            min_qty: Quantity(3),
        },
        Constraint::ActorKind(EntityKind::Agent),
    ];

    const ALL_TARGET_SPECS: [TargetSpec; 2] = [
        TargetSpec::SpecificEntity(ENTITY_B),
        TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Facility,
        },
    ];

    const ALL_PRECONDITIONS: [Precondition; 7] = [
        Precondition::ActorAlive,
        Precondition::ActorCanControlTarget(6),
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(1),
        Precondition::TargetKind {
            target_index: 2,
            kind: EntityKind::Container,
        },
        Precondition::TargetCommodity {
            target_index: 3,
            kind: CommodityKind::Water,
        },
        Precondition::TargetHasConsumableEffect {
            target_index: 4,
            effect: ConsumableEffect::Thirst,
        },
    ];

    const ALL_RESERVATION_REQS: [ReservationReq; 2] = [
        ReservationReq { target_index: 0 },
        ReservationReq { target_index: 3 },
    ];

    const ALL_DURATION_EXPRS: [DurationExpr; 4] = [
        DurationExpr::Fixed(NonZeroU32::MIN),
        DurationExpr::Fixed(NonZeroU32::new(5).unwrap()),
        DurationExpr::TargetConsumable { target_index: 0 },
        DurationExpr::ActorMetabolism {
            kind: MetabolismDurationKind::Wash,
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
                .resolve_for(&world, actor, &[target])
                .unwrap(),
            CommodityKind::Bread
                .spec()
                .consumable_profile
                .unwrap()
                .consumption_ticks_per_unit
                .get()
        );
        assert_eq!(
            DurationExpr::ActorMetabolism {
                kind: MetabolismDurationKind::Toilet,
            }
            .resolve_for(&world, actor, &[target])
            .unwrap(),
            7
        );
        assert_eq!(
            DurationExpr::ActorMetabolism {
                kind: MetabolismDurationKind::Wash,
            }
            .resolve_for(&world, actor, &[target])
            .unwrap(),
            9
        );
    }

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
    }

    #[test]
    fn serialized_indices_are_single_byte_fields() {
        assert_eq!(mem::size_of::<u8>(), 1);
        assert_eq!(mem::size_of::<ReservationReq>(), 1);
    }
}
