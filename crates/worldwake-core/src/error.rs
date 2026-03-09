//! Central error type for all Phase 1 legality failures.

use crate::EntityId;
use std::fmt;

/// Unified error type for world simulation failures.
#[derive(Debug, Clone)]
pub enum WorldError {
    EntityNotFound(EntityId),
    ArchivedEntity(EntityId),
    ComponentNotFound {
        entity: EntityId,
        component_type: &'static str,
    },
    DuplicateComponent {
        entity: EntityId,
        component_type: &'static str,
    },
    InvalidOperation(String),
    InvariantViolation(String),
    InsufficientQuantity {
        entity: EntityId,
        requested: u32,
        available: u32,
    },
    CapacityExceeded {
        container: EntityId,
        requested: u32,
        remaining: u32,
    },
    ContainmentCycle {
        entity: EntityId,
        container: EntityId,
    },
    ConflictingReservation {
        entity: EntityId,
    },
    PreconditionFailed(String),
    CommitFailed(String),
    DeterminismViolation(String),
    SerializationError(String),
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityNotFound(id) => write!(f, "entity not found: {id}"),
            Self::ArchivedEntity(id) => write!(f, "entity archived: {id}"),
            Self::ComponentNotFound {
                entity,
                component_type,
            } => write!(f, "component {component_type} not found on {entity}"),
            Self::DuplicateComponent {
                entity,
                component_type,
            } => write!(f, "duplicate component {component_type} on {entity}"),
            Self::InvalidOperation(msg) => write!(f, "invalid operation: {msg}"),
            Self::InvariantViolation(msg) => write!(f, "invariant violation: {msg}"),
            Self::InsufficientQuantity {
                entity,
                requested,
                available,
            } => write!(
                f,
                "insufficient quantity on {entity}: requested {requested}, available {available}"
            ),
            Self::CapacityExceeded {
                container,
                requested,
                remaining,
            } => write!(
                f,
                "capacity exceeded on {container}: requested {requested}, remaining {remaining}"
            ),
            Self::ContainmentCycle { entity, container } => {
                write!(f, "containment cycle: {entity} in {container}")
            }
            Self::ConflictingReservation { entity } => {
                write!(f, "conflicting reservation on {entity}")
            }
            Self::PreconditionFailed(msg) => write!(f, "precondition failed: {msg}"),
            Self::CommitFailed(msg) => write!(f, "commit failed: {msg}"),
            Self::DeterminismViolation(msg) => write!(f, "determinism violation: {msg}"),
            Self::SerializationError(msg) => write!(f, "serialization error: {msg}"),
        }
    }
}

impl std::error::Error for WorldError {}

// Send + Sync are auto-derived since all fields are Send + Sync.

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn world_error_is_send_sync() {
        assert_send_sync::<WorldError>();
    }

    #[test]
    fn each_variant_displays_non_empty() {
        let id = EntityId {
            slot: 1,
            generation: 0,
        };
        let variants: Vec<WorldError> = vec![
            WorldError::EntityNotFound(id),
            WorldError::ArchivedEntity(id),
            WorldError::ComponentNotFound {
                entity: id,
                component_type: "Health",
            },
            WorldError::DuplicateComponent {
                entity: id,
                component_type: "Health",
            },
            WorldError::InvalidOperation("test".into()),
            WorldError::InvariantViolation("test".into()),
            WorldError::InsufficientQuantity {
                entity: id,
                requested: 5,
                available: 2,
            },
            WorldError::CapacityExceeded {
                container: id,
                requested: 10,
                remaining: 3,
            },
            WorldError::ContainmentCycle {
                entity: id,
                container: id,
            },
            WorldError::ConflictingReservation { entity: id },
            WorldError::PreconditionFailed("test".into()),
            WorldError::CommitFailed("test".into()),
            WorldError::DeterminismViolation("test".into()),
            WorldError::SerializationError("test".into()),
        ];
        for v in &variants {
            let msg = v.to_string();
            assert!(!msg.is_empty(), "variant {v:?} produced empty Display");
        }
    }
}
