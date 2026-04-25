//! Reward reservation records for institution-funded social artifacts.

use crate::{CommodityKind, Component, EntityId, Quantity};
use serde::{Deserialize, Serialize};

/// A single active office-treasury reservation backing one bounty artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewardReservation {
    pub bounty_artifact: EntityId,
    pub commodity: CommodityKind,
    pub quantity: Quantity,
}

/// Records active office treasury rewards reserved for bounty artifacts.
///
/// Treasury funds remain conserved item lots. The canonical treasury convention
/// is a container owned by the office, with contained lots also owned by the
/// office, so existing office control queries continue to enumerate the funds.
/// This component is only the claim record against those funds. It is attached
/// to the office whose funds are reserved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewardEncumbrance {
    pub reservations: Vec<RewardReservation>,
}

impl RewardEncumbrance {
    pub fn from_reservation(reservation: RewardReservation) -> Self {
        Self {
            reservations: vec![reservation],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.reservations.is_empty()
    }

    pub fn reserved_quantity(&self, commodity: CommodityKind) -> Quantity {
        Quantity(
            self.reservations
                .iter()
                .filter(|reservation| reservation.commodity == commodity)
                .fold(0_u32, |total, reservation| {
                    total.saturating_add(reservation.quantity.0)
                }),
        )
    }

    pub fn reserve(&mut self, reservation: RewardReservation) {
        self.reservations.push(reservation);
        self.reservations
            .sort_by_key(|reservation| reservation.bounty_artifact);
    }

    pub fn release(&mut self, bounty_artifact: EntityId) -> bool {
        let original_len = self.reservations.len();
        self.reservations
            .retain(|reservation| reservation.bounty_artifact != bounty_artifact);
        self.reservations.len() != original_len
    }

    pub fn contains_bounty(&self, bounty_artifact: EntityId) -> bool {
        self.reservations
            .iter()
            .any(|reservation| reservation.bounty_artifact == bounty_artifact)
    }
}

impl Component for RewardEncumbrance {}

#[cfg(test)]
mod tests {
    use super::{RewardEncumbrance, RewardReservation};
    use crate::{
        CommodityKind, Component, EntityKind, Quantity, Tick, Topology, World, WorldError,
    };
    use serde::{Serialize, de::DeserializeOwned};

    fn assert_component_bounds<T: Component + Clone + std::fmt::Debug + PartialEq + Eq>() {}

    fn assert_value_bounds<T: Serialize + DeserializeOwned + Clone + PartialEq + Eq>() {}

    #[test]
    fn reward_encumbrance_component_bounds() {
        assert_component_bounds::<RewardEncumbrance>();
        assert_value_bounds::<RewardEncumbrance>();
        assert_value_bounds::<RewardReservation>();
    }

    #[test]
    fn reward_encumbrance_roundtrips_through_bincode() {
        let encumbrance = RewardEncumbrance {
            reservations: vec![RewardReservation {
                bounty_artifact: crate::test_utils::entity_id(2, 0),
                commodity: CommodityKind::Coin,
                quantity: Quantity(17),
            }],
        };

        let bytes = bincode::serialize(&encumbrance).unwrap();
        let roundtrip: RewardEncumbrance = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, encumbrance);
    }

    #[test]
    fn world_inserts_and_queries_reward_encumbrance_on_office() {
        let mut world = World::new(Topology::new()).unwrap();
        let office = world.create_entity(EntityKind::Office, Tick(1));
        let bounty_artifact = world.create_entity(EntityKind::SocialArtifact, Tick(1));
        let encumbrance = RewardEncumbrance {
            reservations: vec![RewardReservation {
                bounty_artifact,
                commodity: CommodityKind::Coin,
                quantity: Quantity(11),
            }],
        };

        world
            .insert_component_reward_encumbrance(office, encumbrance.clone())
            .unwrap();

        assert_eq!(
            world.get_component_reward_encumbrance(office),
            Some(&encumbrance)
        );
        assert!(world.has_component_reward_encumbrance(office));
        assert_eq!(world.count_with_reward_encumbrance(), 1);
        assert_eq!(
            world.query_reward_encumbrance().collect::<Vec<_>>(),
            vec![(office, &encumbrance)]
        );
    }

    #[test]
    fn reward_encumbrance_attachment_rejected_on_non_office_entity_kinds() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let bounty_artifact = world.create_entity(EntityKind::SocialArtifact, Tick(1));
        let encumbrance = RewardEncumbrance {
            reservations: vec![RewardReservation {
                bounty_artifact,
                commodity: CommodityKind::Coin,
                quantity: Quantity(5),
            }],
        };

        let error = world
            .insert_component_reward_encumbrance(agent, encumbrance)
            .unwrap_err();

        assert!(matches!(error, WorldError::InvalidOperation(_)));
        assert!(!world.has_component_reward_encumbrance(agent));
    }

    #[test]
    fn reward_encumbrance_sums_and_releases_multiple_reservations() {
        let first_bounty = crate::test_utils::entity_id(2, 0);
        let second_bounty = crate::test_utils::entity_id(3, 0);
        let mut encumbrance = RewardEncumbrance::from_reservation(RewardReservation {
            bounty_artifact: first_bounty,
            commodity: CommodityKind::Coin,
            quantity: Quantity(5),
        });
        encumbrance.reserve(RewardReservation {
            bounty_artifact: second_bounty,
            commodity: CommodityKind::Coin,
            quantity: Quantity(7),
        });

        assert_eq!(
            encumbrance.reserved_quantity(CommodityKind::Coin),
            Quantity(12)
        );
        assert!(encumbrance.contains_bounty(first_bounty));
        assert!(encumbrance.release(first_bounty));
        assert_eq!(
            encumbrance.reserved_quantity(CommodityKind::Coin),
            Quantity(7)
        );
        assert!(!encumbrance.contains_bounty(first_bounty));
    }
}
