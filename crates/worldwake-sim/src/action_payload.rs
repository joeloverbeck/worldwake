use serde::{Deserialize, Serialize};
use worldwake_core::{
    ActionDefId, CombatWeaponRef, CommodityKind, EntityId, Quantity, RecipeId, UniqueItemKind,
    WorkstationTag,
};

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ActionPayload {
    #[default]
    None,
    Tell(TellActionPayload),
    Bribe(BribeActionPayload),
    Threaten(ThreatenActionPayload),
    DeclareSupport(DeclareSupportActionPayload),
    Transport(TransportActionPayload),
    Harvest(HarvestActionPayload),
    Craft(CraftActionPayload),
    Trade(TradeActionPayload),
    Combat(CombatActionPayload),
    Loot(LootActionPayload),
    QueueForFacilityUse(QueueForFacilityUsePayload),
}

impl ActionPayload {
    #[must_use]
    pub const fn as_bribe(&self) -> Option<&BribeActionPayload> {
        match self {
            Self::Bribe(payload) => Some(payload),
            Self::None
            | Self::Tell(_)
            | Self::Threaten(_)
            | Self::DeclareSupport(_)
            | Self::Transport(_)
            | Self::Harvest(_)
            | Self::Craft(_)
            | Self::Trade(_)
            | Self::Combat(_)
            | Self::Loot(_)
            | Self::QueueForFacilityUse(_) => None,
        }
    }

    #[must_use]
    pub const fn as_threaten(&self) -> Option<&ThreatenActionPayload> {
        match self {
            Self::Threaten(payload) => Some(payload),
            Self::None
            | Self::Tell(_)
            | Self::Bribe(_)
            | Self::DeclareSupport(_)
            | Self::Transport(_)
            | Self::Harvest(_)
            | Self::Craft(_)
            | Self::Trade(_)
            | Self::Combat(_)
            | Self::Loot(_)
            | Self::QueueForFacilityUse(_) => None,
        }
    }

    #[must_use]
    pub const fn as_declare_support(&self) -> Option<&DeclareSupportActionPayload> {
        match self {
            Self::DeclareSupport(payload) => Some(payload),
            Self::None
            | Self::Tell(_)
            | Self::Bribe(_)
            | Self::Threaten(_)
            | Self::Transport(_)
            | Self::Harvest(_)
            | Self::Craft(_)
            | Self::Trade(_)
            | Self::Combat(_)
            | Self::Loot(_)
            | Self::QueueForFacilityUse(_) => None,
        }
    }

    #[must_use]
    pub const fn as_harvest(&self) -> Option<&HarvestActionPayload> {
        match self {
            Self::Harvest(payload) => Some(payload),
            Self::None
            | Self::Tell(_)
            | Self::Bribe(_)
            | Self::Threaten(_)
            | Self::DeclareSupport(_)
            | Self::Transport(_)
            | Self::Craft(_)
            | Self::Trade(_)
            | Self::Combat(_)
            | Self::Loot(_)
            | Self::QueueForFacilityUse(_) => None,
        }
    }

    #[must_use]
    pub const fn as_transport(&self) -> Option<&TransportActionPayload> {
        match self {
            Self::Transport(payload) => Some(payload),
            Self::None
            | Self::Tell(_)
            | Self::Bribe(_)
            | Self::Threaten(_)
            | Self::DeclareSupport(_)
            | Self::Harvest(_)
            | Self::Craft(_)
            | Self::Trade(_)
            | Self::Combat(_)
            | Self::Loot(_)
            | Self::QueueForFacilityUse(_) => None,
        }
    }

    #[must_use]
    pub const fn as_craft(&self) -> Option<&CraftActionPayload> {
        match self {
            Self::Craft(payload) => Some(payload),
            Self::None
            | Self::Tell(_)
            | Self::Bribe(_)
            | Self::Threaten(_)
            | Self::DeclareSupport(_)
            | Self::Transport(_)
            | Self::Harvest(_)
            | Self::Trade(_)
            | Self::Combat(_)
            | Self::Loot(_)
            | Self::QueueForFacilityUse(_) => None,
        }
    }

    #[must_use]
    pub const fn as_trade(&self) -> Option<&TradeActionPayload> {
        match self {
            Self::Trade(payload) => Some(payload),
            Self::None
            | Self::Tell(_)
            | Self::Bribe(_)
            | Self::Threaten(_)
            | Self::DeclareSupport(_)
            | Self::Transport(_)
            | Self::Harvest(_)
            | Self::Craft(_)
            | Self::Combat(_)
            | Self::Loot(_)
            | Self::QueueForFacilityUse(_) => None,
        }
    }

    #[must_use]
    pub const fn as_combat(&self) -> Option<&CombatActionPayload> {
        match self {
            Self::Combat(payload) => Some(payload),
            Self::None
            | Self::Tell(_)
            | Self::Bribe(_)
            | Self::Threaten(_)
            | Self::DeclareSupport(_)
            | Self::Transport(_)
            | Self::Harvest(_)
            | Self::Craft(_)
            | Self::Trade(_)
            | Self::Loot(_)
            | Self::QueueForFacilityUse(_) => None,
        }
    }

    #[must_use]
    pub const fn as_loot(&self) -> Option<&LootActionPayload> {
        match self {
            Self::Loot(payload) => Some(payload),
            Self::None
            | Self::Tell(_)
            | Self::Bribe(_)
            | Self::Threaten(_)
            | Self::DeclareSupport(_)
            | Self::Transport(_)
            | Self::Harvest(_)
            | Self::Craft(_)
            | Self::Trade(_)
            | Self::Combat(_)
            | Self::QueueForFacilityUse(_) => None,
        }
    }

    #[must_use]
    pub const fn as_queue_for_facility_use(&self) -> Option<&QueueForFacilityUsePayload> {
        match self {
            Self::QueueForFacilityUse(payload) => Some(payload),
            Self::None
            | Self::Tell(_)
            | Self::Bribe(_)
            | Self::Threaten(_)
            | Self::DeclareSupport(_)
            | Self::Transport(_)
            | Self::Harvest(_)
            | Self::Craft(_)
            | Self::Trade(_)
            | Self::Combat(_)
            | Self::Loot(_) => None,
        }
    }

    #[must_use]
    pub const fn as_tell(&self) -> Option<&TellActionPayload> {
        match self {
            Self::Tell(payload) => Some(payload),
            Self::None
            | Self::Bribe(_)
            | Self::Threaten(_)
            | Self::DeclareSupport(_)
            | Self::Transport(_)
            | Self::Harvest(_)
            | Self::Craft(_)
            | Self::Trade(_)
            | Self::Combat(_)
            | Self::Loot(_)
            | Self::QueueForFacilityUse(_) => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TellActionPayload {
    pub listener: EntityId,
    pub subject_entity: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BribeActionPayload {
    pub target: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ThreatenActionPayload {
    pub target: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DeclareSupportActionPayload {
    pub office: EntityId,
    pub candidate: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TransportActionPayload {
    pub quantity: Quantity,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HarvestActionPayload {
    pub recipe_id: RecipeId,
    pub required_workstation_tag: WorkstationTag,
    pub output_commodity: CommodityKind,
    pub output_quantity: Quantity,
    pub required_tool_kinds: Vec<UniqueItemKind>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CraftActionPayload {
    pub recipe_id: RecipeId,
    pub required_workstation_tag: WorkstationTag,
    pub inputs: Vec<(CommodityKind, Quantity)>,
    pub outputs: Vec<(CommodityKind, Quantity)>,
    pub required_tool_kinds: Vec<UniqueItemKind>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TradeActionPayload {
    pub counterparty: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
    pub requested_commodity: CommodityKind,
    pub requested_quantity: Quantity,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CombatActionPayload {
    pub target: EntityId,
    pub weapon: CombatWeaponRef,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LootActionPayload {
    pub target: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct QueueForFacilityUsePayload {
    pub intended_action: ActionDefId,
}

#[cfg(test)]
mod tests {
    use super::{
        ActionPayload, BribeActionPayload, CombatActionPayload, CraftActionPayload,
        DeclareSupportActionPayload, HarvestActionPayload, LootActionPayload,
        QueueForFacilityUsePayload, TellActionPayload, ThreatenActionPayload,
        TradeActionPayload, TransportActionPayload,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{
        ActionDefId, CombatWeaponRef, CommodityKind, EntityId, Quantity, RecipeId, UniqueItemKind,
        WorkstationTag,
    };

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn sample_payload() -> HarvestActionPayload {
        HarvestActionPayload {
            recipe_id: RecipeId(4),
            required_workstation_tag: WorkstationTag::OrchardRow,
            output_commodity: CommodityKind::Apple,
            output_quantity: Quantity(2),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
        }
    }

    fn sample_tell_payload() -> TellActionPayload {
        TellActionPayload {
            listener: EntityId {
                slot: 5,
                generation: 0,
            },
            subject_entity: EntityId {
                slot: 8,
                generation: 2,
            },
        }
    }

    fn sample_bribe_payload() -> BribeActionPayload {
        BribeActionPayload {
            target: EntityId {
                slot: 9,
                generation: 1,
            },
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(7),
        }
    }

    fn sample_threaten_payload() -> ThreatenActionPayload {
        ThreatenActionPayload {
            target: EntityId {
                slot: 12,
                generation: 3,
            },
        }
    }

    fn sample_declare_support_payload() -> DeclareSupportActionPayload {
        DeclareSupportActionPayload {
            office: EntityId {
                slot: 14,
                generation: 0,
            },
            candidate: EntityId {
                slot: 16,
                generation: 1,
            },
        }
    }

    fn sample_craft_payload() -> CraftActionPayload {
        CraftActionPayload {
            recipe_id: RecipeId(7),
            required_workstation_tag: WorkstationTag::Mill,
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
        }
    }

    fn sample_trade_payload() -> TradeActionPayload {
        TradeActionPayload {
            counterparty: EntityId {
                slot: 11,
                generation: 0,
            },
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(4),
            requested_commodity: CommodityKind::Bread,
            requested_quantity: Quantity(2),
        }
    }

    fn sample_combat_payload() -> CombatActionPayload {
        CombatActionPayload {
            target: EntityId {
                slot: 17,
                generation: 2,
            },
            weapon: CombatWeaponRef::Unarmed,
        }
    }

    fn sample_loot_payload() -> LootActionPayload {
        LootActionPayload {
            target: EntityId {
                slot: 23,
                generation: 1,
            },
        }
    }

    fn sample_queue_payload() -> QueueForFacilityUsePayload {
        QueueForFacilityUsePayload {
            intended_action: ActionDefId(19),
        }
    }

    #[test]
    fn action_payload_satisfies_required_traits() {
        assert_traits::<ActionPayload>();
        assert_traits::<TellActionPayload>();
        assert_traits::<BribeActionPayload>();
        assert_traits::<ThreatenActionPayload>();
        assert_traits::<DeclareSupportActionPayload>();
        assert_traits::<TransportActionPayload>();
        assert_traits::<HarvestActionPayload>();
        assert_traits::<CraftActionPayload>();
        assert_traits::<TradeActionPayload>();
        assert_traits::<CombatActionPayload>();
        assert_traits::<LootActionPayload>();
        assert_traits::<QueueForFacilityUsePayload>();
    }

    #[test]
    fn action_payload_default_is_none() {
        assert_eq!(ActionPayload::default(), ActionPayload::None);
    }

    #[test]
    fn typed_accessors_cover_social_payload_variants() {
        let harvest = ActionPayload::Harvest(sample_payload());
        let tell = ActionPayload::Tell(sample_tell_payload());
        let bribe = ActionPayload::Bribe(sample_bribe_payload());
        let threaten = ActionPayload::Threaten(sample_threaten_payload());
        let declare_support = ActionPayload::DeclareSupport(sample_declare_support_payload());

        assert_eq!(tell.as_tell(), Some(&sample_tell_payload()));
        assert_eq!(tell.as_bribe(), None);
        assert_eq!(tell.as_threaten(), None);
        assert_eq!(tell.as_declare_support(), None);
        assert_eq!(tell.as_harvest(), None);
        assert_eq!(tell.as_transport(), None);
        assert_eq!(tell.as_craft(), None);
        assert_eq!(tell.as_trade(), None);
        assert_eq!(tell.as_combat(), None);
        assert_eq!(tell.as_loot(), None);
        assert_eq!(tell.as_queue_for_facility_use(), None);

        assert_eq!(bribe.as_tell(), None);
        assert_eq!(bribe.as_bribe(), Some(&sample_bribe_payload()));
        assert_eq!(bribe.as_threaten(), None);
        assert_eq!(bribe.as_declare_support(), None);
        assert_eq!(bribe.as_harvest(), None);
        assert_eq!(bribe.as_transport(), None);
        assert_eq!(bribe.as_craft(), None);
        assert_eq!(bribe.as_trade(), None);

        assert_eq!(threaten.as_tell(), None);
        assert_eq!(threaten.as_bribe(), None);
        assert_eq!(threaten.as_threaten(), Some(&sample_threaten_payload()));
        assert_eq!(threaten.as_declare_support(), None);
        assert_eq!(threaten.as_harvest(), None);
        assert_eq!(threaten.as_transport(), None);
        assert_eq!(threaten.as_craft(), None);
        assert_eq!(threaten.as_trade(), None);

        assert_eq!(declare_support.as_tell(), None);
        assert_eq!(declare_support.as_bribe(), None);
        assert_eq!(declare_support.as_threaten(), None);
        assert_eq!(
            declare_support.as_declare_support(),
            Some(&sample_declare_support_payload())
        );
        assert_eq!(declare_support.as_harvest(), None);
        assert_eq!(declare_support.as_transport(), None);
        assert_eq!(declare_support.as_craft(), None);
        assert_eq!(declare_support.as_trade(), None);

        assert_eq!(harvest.as_tell(), None);
        assert_eq!(harvest.as_bribe(), None);
        assert_eq!(harvest.as_threaten(), None);
        assert_eq!(harvest.as_declare_support(), None);
    }

    #[test]
    fn typed_accessors_cover_trade_and_production_payload_variants() {
        let transport = ActionPayload::Transport(TransportActionPayload {
            quantity: Quantity(3),
        });
        let harvest = ActionPayload::Harvest(sample_payload());
        let craft = ActionPayload::Craft(sample_craft_payload());
        let trade = ActionPayload::Trade(sample_trade_payload());

        assert_eq!(
            transport.as_transport(),
            Some(&TransportActionPayload {
                quantity: Quantity(3)
            })
        );
        assert_eq!(transport.as_bribe(), None);
        assert_eq!(transport.as_threaten(), None);
        assert_eq!(transport.as_declare_support(), None);
        assert_eq!(transport.as_harvest(), None);
        assert_eq!(transport.as_craft(), None);
        assert_eq!(transport.as_trade(), None);

        assert_eq!(harvest.as_harvest(), Some(&sample_payload()));
        assert_eq!(harvest.as_bribe(), None);
        assert_eq!(harvest.as_threaten(), None);
        assert_eq!(harvest.as_declare_support(), None);
        assert_eq!(harvest.as_transport(), None);
        assert_eq!(harvest.as_craft(), None);
        assert_eq!(harvest.as_trade(), None);

        assert_eq!(craft.as_harvest(), None);
        assert_eq!(craft.as_bribe(), None);
        assert_eq!(craft.as_threaten(), None);
        assert_eq!(craft.as_declare_support(), None);
        assert_eq!(craft.as_transport(), None);
        assert_eq!(craft.as_craft(), Some(&sample_craft_payload()));
        assert_eq!(craft.as_trade(), None);

        assert_eq!(trade.as_harvest(), None);
        assert_eq!(trade.as_bribe(), None);
        assert_eq!(trade.as_threaten(), None);
        assert_eq!(trade.as_declare_support(), None);
        assert_eq!(trade.as_transport(), None);
        assert_eq!(trade.as_craft(), None);
        assert_eq!(trade.as_trade(), Some(&sample_trade_payload()));
    }

    #[test]
    fn typed_accessors_cover_combat_and_queue_payload_variants() {
        let combat = ActionPayload::Combat(sample_combat_payload());
        let loot = ActionPayload::Loot(sample_loot_payload());
        let queue = ActionPayload::QueueForFacilityUse(sample_queue_payload());

        assert_eq!(combat.as_tell(), None);
        assert_eq!(combat.as_harvest(), None);
        assert_eq!(combat.as_bribe(), None);
        assert_eq!(combat.as_threaten(), None);
        assert_eq!(combat.as_declare_support(), None);
        assert_eq!(combat.as_transport(), None);
        assert_eq!(combat.as_craft(), None);
        assert_eq!(combat.as_trade(), None);
        assert_eq!(combat.as_combat(), Some(&sample_combat_payload()));

        assert_eq!(loot.as_tell(), None);
        assert_eq!(loot.as_harvest(), None);
        assert_eq!(loot.as_bribe(), None);
        assert_eq!(loot.as_threaten(), None);
        assert_eq!(loot.as_declare_support(), None);
        assert_eq!(loot.as_transport(), None);
        assert_eq!(loot.as_craft(), None);
        assert_eq!(loot.as_trade(), None);
        assert_eq!(loot.as_combat(), None);
        assert_eq!(loot.as_loot(), Some(&sample_loot_payload()));

        assert_eq!(queue.as_tell(), None);
        assert_eq!(queue.as_harvest(), None);
        assert_eq!(queue.as_bribe(), None);
        assert_eq!(queue.as_threaten(), None);
        assert_eq!(queue.as_declare_support(), None);
        assert_eq!(queue.as_transport(), None);
        assert_eq!(queue.as_craft(), None);
        assert_eq!(queue.as_trade(), None);
        assert_eq!(queue.as_combat(), None);
        assert_eq!(queue.as_loot(), None);
        assert_eq!(
            queue.as_queue_for_facility_use(),
            Some(&sample_queue_payload())
        );
    }

    #[test]
    fn typed_accessors_return_none_for_action_payload_none() {
        let none = ActionPayload::None;

        assert_eq!(none.as_harvest(), None);
        assert_eq!(none.as_tell(), None);
        assert_eq!(none.as_bribe(), None);
        assert_eq!(none.as_threaten(), None);
        assert_eq!(none.as_declare_support(), None);
        assert_eq!(none.as_transport(), None);
        assert_eq!(none.as_craft(), None);
        assert_eq!(none.as_trade(), None);
        assert_eq!(none.as_combat(), None);
        assert_eq!(none.as_loot(), None);
        assert_eq!(none.as_queue_for_facility_use(), None);
    }

    #[test]
    fn action_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Harvest(sample_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn tell_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Tell(sample_tell_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn bribe_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Bribe(sample_bribe_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn threaten_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Threaten(sample_threaten_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn declare_support_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::DeclareSupport(sample_declare_support_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn craft_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Craft(sample_craft_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn trade_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Trade(sample_trade_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn combat_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Combat(sample_combat_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn loot_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Loot(sample_loot_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn queue_for_facility_use_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::QueueForFacilityUse(sample_queue_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }
}
