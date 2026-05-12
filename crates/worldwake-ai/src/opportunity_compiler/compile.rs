use std::collections::BTreeMap;

use crate::PlannerOpKind;
use crate::decision_trace::OpportunityCompilerLoad;
use crate::effect_schema_index::EffectSchemaIndex;
use crate::opportunity_compiler::{
    BelievedLegalStatus, ClaimTopic, EffectFactKey, Opportunity, OpportunityHandle,
    PerceivedOpportunityIndex, RiskFact, SocialExposureBand,
};
use worldwake_core::{
    AcquisitionQuantity, BeliefClaimKey, BeliefRef, BeliefStatusTag, CommodityKind,
    CommodityPurpose, EntityBeliefAspect, EntityId, GoalKind, HypothesisKind, OpportunityAnchor,
    OpportunityKey, Permille, Quantity, Tick,
};
use worldwake_sim::RuntimeBeliefView;

pub fn compile_opportunities(
    agent: EntityId,
    belief_view: &impl RuntimeBeliefView,
    action_index: &EffectSchemaIndex,
) -> (Vec<Opportunity>, OpportunityCompilerLoad) {
    let mut load = OpportunityCompilerLoad::default();
    if action_index
        .actions_producing(EffectFactKey::CommodityTransfer)
        .is_empty()
    {
        return (Vec::new(), load);
    }

    let current_tick = belief_view.current_tick();
    let floor = belief_view
        .perception_profile(agent)
        .map_or(Permille::ZERO, |profile| profile.opportunity_floor_permille);
    let cap = usize::from(
        belief_view
            .cognitive_profile(agent)
            .unwrap_or_default()
            .compile_opportunity_cap,
    );
    let learned_memory = belief_view.learned_opportunity_memory(agent);
    let risk = belief_view.risk_weight_profile(agent).unwrap_or_default();
    let law = belief_view.law_abiding_profile(agent).unwrap_or_default();

    let mut opportunities = Vec::new();
    for (entity, state) in belief_view.known_entity_beliefs(agent) {
        let Some(place) = state
            .last_known_place
            .or_else(|| belief_view.effective_place(entity))
        else {
            continue;
        };
        if confirmed_empty_for_any_inventory(belief_view, agent, place, &state.last_known_inventory)
        {
            continue;
        }
        for (&commodity, &quantity) in &state.last_known_inventory {
            if quantity <= Quantity(0) {
                continue;
            }
            if belief_view.direct_possessor(entity) == Some(agent) {
                continue;
            }
            let Some(goal) = acquisition_goal_for_commodity(commodity) else {
                continue;
            };
            let key = OpportunityKey {
                goal_key: goal.into(),
                anchor: OpportunityAnchor::Entity(entity),
            };
            let owned = belief_view.believed_owner_of(entity);
            let mut salience = salience_for_quantity(quantity);
            let mut risks = Vec::new();
            let legal_status = if let Some(owner) = owned {
                if owner == agent {
                    BelievedLegalStatus::BelievedOwned { owner }
                } else {
                    risks.push(RiskFact::CriminalLiability {
                        violation_kind: worldwake_core::ViolationKind::SuspectedTheft {
                            theft: worldwake_core::TheftFacts {
                                missing_entity: entity,
                                expected_place: place,
                                commodity,
                                quantity,
                            },
                            suspect: Some(agent),
                        },
                    });
                    salience = penalize(salience, risk.theft_aversion);
                    salience = penalize(salience, law.criminal_threshold);
                    BelievedLegalStatus::BelievedOwned { owner }
                }
            } else {
                BelievedLegalStatus::BelievedUnclaimed
            };
            if !matches!(legal_status, BelievedLegalStatus::BelievedUnclaimed) {
                salience = penalize(salience, risk.exposure_aversion);
                salience = penalize(salience, law.social_norm_weight);
            }

            if learned_memory.is_some_and(|memory| memory.opportunities.contains_key(&key)) {
                load.learned_memory_damped = load.learned_memory_damped.saturating_add(1);
                salience = Permille::new_unchecked(salience.value() / 2);
            }
            if salience < floor {
                load.salience_floored = load.salience_floored.saturating_add(1);
                continue;
            }

            load.compiled_count = load.compiled_count.saturating_add(1);
            opportunities.push(Opportunity {
                key,
                perceived_at: current_tick,
                source_belief: source_belief(entity, commodity, &state),
                possible_effects: vec![EffectFactKey::CommodityTransfer],
                possible_information: vec![
                    ClaimTopic::EntityLocation { subject: entity },
                    ClaimTopic::CommodityAvailability { commodity, place },
                ],
                required_actions: vec![PlannerOpKind::MoveCargo],
                legal_status,
                social_exposure: if risks.is_empty() {
                    SocialExposureBand::Public
                } else {
                    SocialExposureBand::PublicWithCriminalRisk
                },
                risks,
                salience,
            });
        }
    }

    opportunities.sort_by_key(|opportunity| {
        (
            std::cmp::Reverse(opportunity.salience),
            opportunity.key,
            opportunity.source_belief,
        )
    });
    if opportunities.len() > cap {
        load.cap_truncated = u32::try_from(opportunities.len() - cap).unwrap_or(u32::MAX);
        opportunities.truncate(cap);
    }

    (opportunities, load)
}

#[must_use]
pub fn build_perceived_opportunity_index(
    opportunities: Vec<Opportunity>,
) -> PerceivedOpportunityIndex {
    let mut by_place: BTreeMap<EntityId, Vec<OpportunityHandle>> = BTreeMap::new();
    let mut by_anchor = BTreeMap::new();
    for (index, opportunity) in opportunities.iter().enumerate() {
        let handle = OpportunityHandle(u32::try_from(index).unwrap_or(u32::MAX));
        match opportunity.key.anchor {
            OpportunityAnchor::Entity(entity) => {
                by_anchor.entry(entity).or_insert(handle);
                if let Some(ClaimTopic::CommodityAvailability { place, .. }) = opportunity
                    .possible_information
                    .iter()
                    .find(|topic| matches!(topic, ClaimTopic::CommodityAvailability { .. }))
                {
                    by_place.entry(*place).or_default().push(handle);
                }
            }
            OpportunityAnchor::Place(place) => {
                by_anchor.entry(place).or_insert(handle);
                by_place.entry(place).or_default().push(handle);
            }
            OpportunityAnchor::None => {}
        }
    }
    PerceivedOpportunityIndex {
        by_place,
        by_anchor,
        all: opportunities,
    }
}

fn acquisition_goal_for_commodity(commodity: CommodityKind) -> Option<GoalKind> {
    commodity
        .spec()
        .consumable_profile
        .map(|_| GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        })
}

fn salience_for_quantity(quantity: Quantity) -> Permille {
    let value = quantity.0.saturating_mul(200).clamp(100, 1000);
    Permille::new_unchecked(u16::try_from(value).unwrap_or(1000))
}

fn penalize(salience: Permille, penalty: Permille) -> Permille {
    let retained = 1000u32.saturating_sub(u32::from(penalty.value()));
    let value = u32::from(salience.value()).saturating_mul(retained) / 1000;
    Permille::new_unchecked(u16::try_from(value).unwrap_or(0))
}

fn source_belief(
    entity: EntityId,
    commodity: CommodityKind,
    state: &worldwake_core::BelievedEntityState,
) -> BeliefRef {
    BeliefRef {
        claim_key: BeliefClaimKey {
            subject: entity,
            aspect: EntityBeliefAspect::Inventory(commodity),
        },
        claim_held_at_tick: state.last_observed_tick().unwrap_or(Tick(0)),
        status: BeliefStatusTag::Probable,
    }
}

fn confirmed_empty_for_any_inventory(
    belief_view: &impl RuntimeBeliefView,
    agent: EntityId,
    place: EntityId,
    inventory: &BTreeMap<CommodityKind, Quantity>,
) -> bool {
    let Some(memory) = belief_view.survey_memory(agent) else {
        return false;
    };
    inventory.keys().any(|commodity| {
        memory
            .find(
                place,
                HypothesisKind::MayContainCommodity {
                    commodity: *commodity,
                },
            )
            .is_some_and(|record| !record.found)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_core::{
        AgentBeliefStore, BelievedEntityState, CauseRef, CognitiveProfile, LawAbidingProfile,
        LearnedOpportunityMemory, OpportunityEntry, PerceptionProfile, PerceptionSource,
        RiskWeightProfile, SurveyMemory, SurveyRecord, VisibilitySpec, WitnessData,
    };
    use worldwake_sim::PerAgentBeliefView;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn belief(_subject: EntityId, commodity: CommodityKind, quantity: u16) -> BelievedEntityState {
        let mut state = BelievedEntityState::single_observation_defaults(
            Tick(10),
            PerceptionSource::DirectObservation,
        );
        state.last_known_place = Some(entity(1));
        state
            .last_known_inventory
            .insert(commodity, Quantity(u32::from(quantity)));
        state
    }

    fn view_with_store(
        store: AgentBeliefStore,
        cognitive: CognitiveProfile,
        perception: PerceptionProfile,
        risk: RiskWeightProfile,
        law: LawAbidingProfile,
        survey: SurveyMemory,
        learned: LearnedOpportunityMemory,
    ) -> (worldwake_core::World, EntityId) {
        let mut world =
            worldwake_core::World::new(worldwake_core::build_prototype_world()).unwrap();
        let agent;
        {
            let mut txn = worldwake_core::WorldTxn::new(
                &mut world,
                Tick(0),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            agent = txn
                .create_agent("Compiler", worldwake_core::ControlSource::Ai)
                .unwrap();
            txn.set_component_agent_belief_store(agent, store).unwrap();
            txn.set_component_cognitive_profile(agent, cognitive)
                .unwrap();
            txn.set_component_perception_profile(agent, perception)
                .unwrap();
            txn.set_component_risk_weight_profile(agent, risk).unwrap();
            txn.set_component_law_abiding_profile(agent, law).unwrap();
            txn.set_component_survey_memory(agent, survey).unwrap();
            txn.set_component_learned_opportunity_memory(agent, learned)
                .unwrap();
            let mut event_log = worldwake_core::EventLog::new();
            txn.commit(&mut event_log);
        }
        (world, agent)
    }

    fn index() -> EffectSchemaIndex {
        EffectSchemaIndex {
            by_effect: BTreeMap::from([(
                EffectFactKey::CommodityTransfer,
                vec![worldwake_core::ActionDefId(0)],
            )]),
        }
    }

    #[test]
    fn compile_opportunities_emits_inventory_backed_opportunities() {
        let mut store = AgentBeliefStore::new();
        store
            .known_entities
            .insert(entity(2), belief(entity(2), CommodityKind::Bread, 3));
        let (world, agent) = view_with_store(
            store,
            CognitiveProfile::default(),
            PerceptionProfile::default(),
            RiskWeightProfile::default(),
            LawAbidingProfile::default(),
            SurveyMemory::default(),
            LearnedOpportunityMemory::default(),
        );
        let view = PerAgentBeliefView::from_world(agent, &world);

        let (opportunities, load) = compile_opportunities(agent, &view, &index());

        assert_eq!(load.compiled_count, 1);
        assert_eq!(opportunities.len(), 1);
        assert_eq!(
            opportunities[0].key.anchor,
            OpportunityAnchor::Entity(entity(2))
        );
        assert_eq!(
            opportunities[0].possible_effects,
            vec![EffectFactKey::CommodityTransfer]
        );
    }

    #[test]
    fn compile_opportunities_applies_floor_damping_and_cap() {
        let mut store = AgentBeliefStore::new();
        for slot in 2..7 {
            store.known_entities.insert(
                entity(slot),
                belief(entity(slot), CommodityKind::Bread, slot as u16),
            );
        }
        let cognitive = CognitiveProfile {
            compile_opportunity_cap: 2,
            ..CognitiveProfile::default()
        };
        let perception = PerceptionProfile {
            opportunity_floor_permille: Permille::new_unchecked(500),
            ..PerceptionProfile::default()
        };
        let (world, agent) = view_with_store(
            store,
            cognitive,
            perception,
            RiskWeightProfile::default(),
            LawAbidingProfile::default(),
            SurveyMemory::default(),
            LearnedOpportunityMemory::default(),
        );
        let view = PerAgentBeliefView::from_world(agent, &world);

        let (opportunities, load) = compile_opportunities(agent, &view, &index());

        assert_eq!(opportunities.len(), 2);
        assert_eq!(load.salience_floored, 1);
        assert_eq!(load.cap_truncated, 2);
        assert!(opportunities[0].salience >= opportunities[1].salience);
    }

    #[test]
    fn compile_opportunities_skips_confirmed_empty_survey_places() {
        let mut store = AgentBeliefStore::new();
        store
            .known_entities
            .insert(entity(2), belief(entity(2), CommodityKind::Bread, 3));
        let mut survey = SurveyMemory::default();
        survey.record(
            SurveyRecord {
                place: entity(1),
                hypothesis: HypothesisKind::MayContainCommodity {
                    commodity: CommodityKind::Bread,
                },
                found: false,
                confidence: Permille::new_unchecked(1000),
                recorded_tick: Tick(9),
            },
            10,
        );
        let (world, agent) = view_with_store(
            store,
            CognitiveProfile::default(),
            PerceptionProfile::default(),
            RiskWeightProfile::default(),
            LawAbidingProfile::default(),
            survey,
            LearnedOpportunityMemory::default(),
        );
        let view = PerAgentBeliefView::from_world(agent, &world);

        let (opportunities, load) = compile_opportunities(agent, &view, &index());

        assert!(opportunities.is_empty());
        assert_eq!(load.compiled_count, 0);
    }

    #[test]
    fn compile_opportunities_damps_learned_memory_entries() {
        let item = entity(2);
        let mut store = AgentBeliefStore::new();
        store
            .known_entities
            .insert(item, belief(item, CommodityKind::Bread, 3));
        let key = OpportunityKey {
            goal_key: acquisition_goal_for_commodity(CommodityKind::Bread)
                .expect("bread should be consumable")
                .into(),
            anchor: OpportunityAnchor::Entity(item),
        };
        let mut learned = LearnedOpportunityMemory::default();
        learned.record(OpportunityEntry {
            opportunity: key,
            observed_tick: Tick(8),
            expires_tick: Tick(40),
            observed_at: entity(1),
        });
        let (world, agent) = view_with_store(
            store,
            CognitiveProfile::default(),
            PerceptionProfile::default(),
            RiskWeightProfile::default(),
            LawAbidingProfile::default(),
            SurveyMemory::default(),
            learned,
        );
        let view = PerAgentBeliefView::from_world(agent, &world);

        let (opportunities, load) = compile_opportunities(agent, &view, &index());

        assert_eq!(load.learned_memory_damped, 1);
        assert_eq!(opportunities[0].salience, Permille::new_unchecked(300));
    }
}
