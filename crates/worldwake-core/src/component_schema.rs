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
        }
    };
}

pub(crate) use with_authoritative_components;
