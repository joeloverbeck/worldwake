use serde::{Deserialize, Serialize};
use worldwake_core::{CommodityKind, EntityId, Quantity, RecipeId, UniqueItemKind, WorkstationTag};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ActionPayload {
    #[default]
    None,
    Harvest(HarvestActionPayload),
    Craft(CraftActionPayload),
    Trade(TradeActionPayload),
}

impl ActionPayload {
    #[must_use]
    pub const fn as_harvest(&self) -> Option<&HarvestActionPayload> {
        match self {
            Self::Harvest(payload) => Some(payload),
            Self::None | Self::Craft(_) | Self::Trade(_) => None,
        }
    }

    #[must_use]
    pub const fn as_craft(&self) -> Option<&CraftActionPayload> {
        match self {
            Self::Craft(payload) => Some(payload),
            Self::None | Self::Harvest(_) | Self::Trade(_) => None,
        }
    }

    #[must_use]
    pub const fn as_trade(&self) -> Option<&TradeActionPayload> {
        match self {
            Self::Trade(payload) => Some(payload),
            Self::None | Self::Harvest(_) | Self::Craft(_) => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HarvestActionPayload {
    pub recipe_id: RecipeId,
    pub required_workstation_tag: WorkstationTag,
    pub output_commodity: CommodityKind,
    pub output_quantity: Quantity,
    pub required_tool_kinds: Vec<UniqueItemKind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CraftActionPayload {
    pub recipe_id: RecipeId,
    pub required_workstation_tag: WorkstationTag,
    pub inputs: Vec<(CommodityKind, Quantity)>,
    pub outputs: Vec<(CommodityKind, Quantity)>,
    pub required_tool_kinds: Vec<UniqueItemKind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradeActionPayload {
    pub counterparty: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
    pub requested_commodity: CommodityKind,
    pub requested_quantity: Quantity,
}

#[cfg(test)]
mod tests {
    use super::{ActionPayload, CraftActionPayload, HarvestActionPayload, TradeActionPayload};
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{
        CommodityKind, EntityId, Quantity, RecipeId, UniqueItemKind, WorkstationTag,
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

    #[test]
    fn action_payload_satisfies_required_traits() {
        assert_traits::<ActionPayload>();
        assert_traits::<HarvestActionPayload>();
        assert_traits::<CraftActionPayload>();
        assert_traits::<TradeActionPayload>();
    }

    #[test]
    fn action_payload_default_is_none() {
        assert_eq!(ActionPayload::default(), ActionPayload::None);
    }

    #[test]
    fn typed_accessors_return_only_matching_payload_variant() {
        let harvest = ActionPayload::Harvest(sample_payload());
        let craft = ActionPayload::Craft(sample_craft_payload());
        let trade = ActionPayload::Trade(sample_trade_payload());
        let none = ActionPayload::None;

        assert_eq!(harvest.as_harvest(), Some(&sample_payload()));
        assert_eq!(harvest.as_craft(), None);
        assert_eq!(harvest.as_trade(), None);

        assert_eq!(craft.as_harvest(), None);
        assert_eq!(craft.as_craft(), Some(&sample_craft_payload()));
        assert_eq!(craft.as_trade(), None);

        assert_eq!(trade.as_harvest(), None);
        assert_eq!(trade.as_craft(), None);
        assert_eq!(trade.as_trade(), Some(&sample_trade_payload()));

        assert_eq!(none.as_harvest(), None);
        assert_eq!(none.as_craft(), None);
        assert_eq!(none.as_trade(), None);
    }

    #[test]
    fn action_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Harvest(sample_payload());

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
}
