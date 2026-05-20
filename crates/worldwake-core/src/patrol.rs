//! Authoritative patrol route state and per-agent patrol configuration.

use crate::{Component, EntityId, Permille, Tick};
use serde::{Deserialize, Serialize};

/// Office-issued patrol obligation carried by the assigned guard.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OfficePatrolDuty {
    pub issuing_office: EntityId,
    pub delegate: Option<EntityId>,
    pub assignee: EntityId,
    pub assigned_places: Vec<EntityId>,
    pub created_tick: Tick,
    pub renewal_due_tick: Tick,
    pub grace_ticks: u32,
    pub lifecycle: OfficePatrolDutyLifecycle,
    pub provenance: OfficePatrolDutyProvenance,
}

impl OfficePatrolDuty {
    #[must_use]
    pub const fn is_actionable(&self) -> bool {
        matches!(self.lifecycle, OfficePatrolDutyLifecycle::Active)
    }
}

impl Component for OfficePatrolDuty {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OfficePatrolDutyLifecycle {
    Active,
    Degraded { since: Tick },
    Lapsed { since: Tick },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OfficePatrolDutyProvenance {
    IssuedByOffice { tick: Tick },
    RenewedByOffice { tick: Tick },
    LapsedByVacancy { tick: Tick },
}

/// Ordered patrol route assignment plus persistent waypoint progress.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatrolRoute {
    pub assigned_places: Vec<EntityId>,
    pub current_index: usize,
}

impl Component for PatrolRoute {}

/// Stable per-agent parameters that shape patrol behavior.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatrolProfile {
    /// Minimum ticks the agent dwells at each patrol waypoint.
    pub base_dwell_ticks: u32,
    /// Additional dwell ticks scaled by vigilance level.
    pub dwell_vigilance_scale_ticks: u32,
    /// Baseline vigilance level during patrol; affects observation thoroughness and dwell time.
    pub vigilance: Permille,
    /// How quickly the agent adapts patrol routes in response to observed threats.
    pub route_adaptation_sensitivity: Permille,
    /// Base motive weight for patrol-driven goals.
    pub patrol_motive_weight: Permille,
}

impl Component for PatrolProfile {}

#[cfg(test)]
mod tests {
    use super::{
        OfficePatrolDuty, OfficePatrolDutyLifecycle, OfficePatrolDutyProvenance, PatrolProfile,
        PatrolRoute,
    };
    use crate::{EntityId, Permille, Tick, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn sample_patrol_route() -> PatrolRoute {
        PatrolRoute {
            assigned_places: vec![entity(3), entity(5), entity(8)],
            current_index: 1,
        }
    }

    fn sample_office_patrol_duty() -> OfficePatrolDuty {
        OfficePatrolDuty {
            issuing_office: entity(20),
            delegate: Some(entity(21)),
            assignee: entity(22),
            assigned_places: vec![entity(3), entity(5)],
            created_tick: Tick(7),
            renewal_due_tick: Tick(12),
            grace_ticks: 4,
            lifecycle: OfficePatrolDutyLifecycle::Active,
            provenance: OfficePatrolDutyProvenance::IssuedByOffice { tick: Tick(7) },
        }
    }

    fn sample_patrol_profile() -> PatrolProfile {
        PatrolProfile {
            base_dwell_ticks: 12,
            dwell_vigilance_scale_ticks: 12,
            vigilance: pm(700),
            route_adaptation_sensitivity: pm(450),
            patrol_motive_weight: pm(550),
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn patrol_components_satisfy_required_traits() {
        assert_component_bounds::<PatrolRoute>();
        assert_component_bounds::<PatrolProfile>();
        assert_component_bounds::<OfficePatrolDuty>();
        assert_value_bounds::<PatrolRoute>();
        assert_value_bounds::<PatrolProfile>();
        assert_value_bounds::<OfficePatrolDuty>();
    }

    #[test]
    fn patrol_route_roundtrips_through_bincode() {
        let route = sample_patrol_route();

        let bytes = bincode::serialize(&route).unwrap();
        let roundtrip: PatrolRoute = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, route);
    }

    #[test]
    fn patrol_profile_roundtrips_through_bincode() {
        let profile = sample_patrol_profile();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: PatrolProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn office_patrol_duty_roundtrips_through_bincode() {
        let duty = sample_office_patrol_duty();

        let bytes = bincode::serialize(&duty).unwrap();
        let roundtrip: OfficePatrolDuty = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, duty);
        assert!(roundtrip.is_actionable());
    }
}
