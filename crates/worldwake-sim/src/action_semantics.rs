use serde::{Deserialize, Serialize};
use worldwake_core::{CommodityKind, EntityId, EntityKind, Quantity};

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
    TargetExists(u8),
    TargetAtActorPlace(u8),
    TargetKind { target_index: u8, kind: EntityKind },
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct ReservationReq {
    pub target_index: u8,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum DurationExpr {
    Fixed(u32),
}

impl DurationExpr {
    #[must_use]
    pub const fn resolve(self) -> u32 {
        match self {
            Self::Fixed(ticks) => ticks,
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
        Constraint, DurationExpr, Interruptibility, Precondition, ReservationReq, TargetSpec,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::mem;
    use worldwake_core::{CommodityKind, EntityId, EntityKind, Quantity};

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

    const ALL_PRECONDITIONS: [Precondition; 4] = [
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(1),
        Precondition::TargetKind {
            target_index: 2,
            kind: EntityKind::Container,
        },
    ];

    const ALL_RESERVATION_REQS: [ReservationReq; 2] = [
        ReservationReq { target_index: 0 },
        ReservationReq { target_index: 3 },
    ];

    const ALL_DURATION_EXPRS: [DurationExpr; 2] = [DurationExpr::Fixed(0), DurationExpr::Fixed(5)];

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
    }

    #[test]
    fn duration_expr_resolves_fixed_ticks() {
        assert_eq!(DurationExpr::Fixed(5).resolve(), 5);
        assert_eq!(DurationExpr::Fixed(0).resolve(), 0);
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
    }

    #[test]
    fn serialized_indices_are_single_byte_fields() {
        assert_eq!(mem::size_of::<u8>(), 1);
        assert_eq!(mem::size_of::<ReservationReq>(), 1);
    }
}
