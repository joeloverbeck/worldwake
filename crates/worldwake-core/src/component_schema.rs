//! Shared authoritative component declarations for typed world storage.

macro_rules! with_authoritative_components {
    ($callback:ident) => {
        $callback! {
            {
                names,
                Name,
                insert_name,
                get_name,
                get_name_mut,
                remove_name,
                has_name,
                iter_names,
                insert_component_name,
                get_component_name,
                get_component_name_mut,
                remove_component_name,
                has_component_name,
                entities_with_name,
                query_name,
                count_with_name,
                "Name",
                |_| true
            }
            {
                agents,
                AgentData,
                insert_agent_data,
                get_agent_data,
                get_agent_data_mut,
                remove_agent_data,
                has_agent_data,
                iter_agent_data,
                insert_component_agent_data,
                get_component_agent_data,
                get_component_agent_data_mut,
                remove_component_agent_data,
                has_component_agent_data,
                entities_with_agent_data,
                query_agent_data,
                count_with_agent_data,
                "AgentData",
                |kind| kind == EntityKind::Agent
            }
            {
                item_lots,
                ItemLot,
                insert_item_lot,
                get_item_lot,
                get_item_lot_mut,
                remove_item_lot,
                has_item_lot,
                iter_item_lots,
                insert_component_item_lot,
                get_component_item_lot,
                get_component_item_lot_mut,
                remove_component_item_lot,
                has_component_item_lot,
                entities_with_item_lot,
                query_item_lot,
                count_with_item_lot,
                "ItemLot",
                |kind| kind == EntityKind::ItemLot
            }
            {
                unique_items,
                UniqueItem,
                insert_unique_item,
                get_unique_item,
                get_unique_item_mut,
                remove_unique_item,
                has_unique_item,
                iter_unique_items,
                insert_component_unique_item,
                get_component_unique_item,
                get_component_unique_item_mut,
                remove_component_unique_item,
                has_component_unique_item,
                entities_with_unique_item,
                query_unique_item,
                count_with_unique_item,
                "UniqueItem",
                |kind| kind == EntityKind::UniqueItem
            }
        }
    };
}

pub(crate) use with_authoritative_components;
