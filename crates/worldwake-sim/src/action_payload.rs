use serde::{Deserialize, Serialize};
use worldwake_core::{CommodityKind, Quantity, RecipeId, UniqueItemKind, WorkstationTag};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ActionPayload {
    #[default]
    None,
    Harvest(HarvestActionPayload),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HarvestActionPayload {
    pub recipe_id: RecipeId,
    pub required_workstation_tag: WorkstationTag,
    pub output_commodity: CommodityKind,
    pub output_quantity: Quantity,
    pub required_tool_kinds: Vec<UniqueItemKind>,
}

#[cfg(test)]
mod tests {
    use super::{ActionPayload, HarvestActionPayload};
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{CommodityKind, Quantity, RecipeId, UniqueItemKind, WorkstationTag};

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

    #[test]
    fn action_payload_satisfies_required_traits() {
        assert_traits::<ActionPayload>();
        assert_traits::<HarvestActionPayload>();
    }

    #[test]
    fn action_payload_default_is_none() {
        assert_eq!(ActionPayload::default(), ActionPayload::None);
    }

    #[test]
    fn action_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Harvest(sample_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }
}
