//! Reward reservation records for institution-funded social artifacts.

use crate::{CommodityKind, Component, EntityId, Quantity};
use serde::{Deserialize, Serialize};

/// Records an office treasury reward reserved for a specific bounty artifact.
///
/// Treasury funds remain conserved item lots. The canonical treasury convention
/// is a container owned by the office, with contained lots also owned by the
/// office, so existing office control queries continue to enumerate the funds.
/// This component is only the claim record against those funds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewardEncumbrance {
    pub bounty_artifact: EntityId,
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub office: EntityId,
}

impl Component for RewardEncumbrance {}

#[cfg(test)]
mod tests {
    use super::RewardEncumbrance;
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
    }

    #[test]
    fn reward_encumbrance_roundtrips_through_bincode() {
        let encumbrance = RewardEncumbrance {
            bounty_artifact: crate::test_utils::entity_id(2, 0),
            commodity: CommodityKind::Coin,
            quantity: Quantity(17),
            office: crate::test_utils::entity_id(1, 0),
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
            bounty_artifact,
            commodity: CommodityKind::Coin,
            quantity: Quantity(11),
            office,
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
        let office = world.create_entity(EntityKind::Office, Tick(1));
        let bounty_artifact = world.create_entity(EntityKind::SocialArtifact, Tick(1));
        let encumbrance = RewardEncumbrance {
            bounty_artifact,
            commodity: CommodityKind::Coin,
            quantity: Quantity(5),
            office,
        };

        let error = world
            .insert_component_reward_encumbrance(agent, encumbrance)
            .unwrap_err();

        assert!(matches!(error, WorldError::InvalidOperation(_)));
        assert!(!world.has_component_reward_encumbrance(agent));
    }
}
