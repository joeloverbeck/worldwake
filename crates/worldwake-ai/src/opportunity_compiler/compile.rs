use std::collections::BTreeMap;

use crate::PlannerOpKind;
use crate::belief_status::belief_status_tag_for_claim;
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
use worldwake_sim::{BeliefRead, RuntimeBeliefView};

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
    let required_actions_for_transfer: Vec<PlannerOpKind> = action_index
        .planner_ops_producing(EffectFactKey::CommodityTransfer)
        .iter()
        .copied()
        .collect();

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
        if entity == agent {
            continue;
        }
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
            let legal_status =
                if let BeliefRead::Known(owner_read) | BeliefRead::Stale(owner_read) = owned {
                    let believed_owner = owner_read.value;
                    if believed_owner == agent {
                        BelievedLegalStatus::BelievedOwned {
                            owner: believed_owner,
                        }
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
                        BelievedLegalStatus::BelievedOwned {
                            owner: believed_owner,
                        }
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

            opportunities.push(Opportunity {
                key,
                perceived_at: current_tick,
                source_belief: source_belief(
                    belief_view,
                    agent,
                    entity,
                    commodity,
                    &state,
                    current_tick,
                ),
                possible_effects: vec![EffectFactKey::CommodityTransfer],
                possible_information: vec![
                    ClaimTopic::EntityLocation { subject: entity },
                    ClaimTopic::CommodityAvailability { commodity, place },
                ],
                required_actions: required_actions_for_transfer.clone(),
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
    for opportunity in &opportunities {
        *load
            .compiled_by_status
            .entry(opportunity.source_belief.status)
            .or_insert(0) += 1;
    }
    load.compiled_count = u32::try_from(opportunities.len()).unwrap_or(u32::MAX);

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
    belief_view: &dyn RuntimeBeliefView,
    agent: EntityId,
    entity: EntityId,
    commodity: CommodityKind,
    state: &worldwake_core::BelievedEntityState,
    tick: Tick,
) -> BeliefRef {
    let claim_key = BeliefClaimKey {
        subject: entity,
        aspect: EntityBeliefAspect::Inventory(commodity),
    };
    BeliefRef {
        claim_key,
        claim_held_at_tick: state.last_observed_tick().unwrap_or(Tick(0)),
        status: source_belief_status(belief_view, agent, &claim_key, tick),
    }
}

fn source_belief_status(
    belief_view: &dyn RuntimeBeliefView,
    agent: EntityId,
    claim_key: &BeliefClaimKey,
    tick: Tick,
) -> BeliefStatusTag {
    let Some(claims) = belief_view
        .agent_belief_store(agent)
        .and_then(|store| store.get_entity_claims(&claim_key.subject))
    else {
        return BeliefStatusTag::Stale;
    };

    let mut first_matching = None;
    let mut first_active = None;
    let mut active_count = 0u8;
    for claim in claims
        .iter()
        .filter(|claim| claim.aspect == claim_key.aspect)
    {
        first_matching.get_or_insert(claim);
        if claim.refuted_at_tick.is_none() {
            first_active.get_or_insert(claim);
            active_count = active_count.saturating_add(1);
            if active_count > 1 {
                return BeliefStatusTag::Disputed;
            }
        }
    }

    first_active
        .or(first_matching)
        .map_or(BeliefStatusTag::Stale, |claim| {
            belief_status_tag_for_claim(belief_view, agent, claim, tick)
        })
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
    use std::collections::BTreeSet;
    use worldwake_core::{
        AgentBeliefStore, BelievedEntityState, CauseRef, ClaimId, ClaimValue, CognitiveProfile,
        EntityBeliefClaim, LawAbidingProfile, LearnedOpportunityMemory, OpportunityEntry,
        PerceptionProfile, PerceptionSource, RiskWeightProfile, SurveyMemory, SurveyRecord,
        VisibilitySpec, WitnessData,
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

    fn inventory_claim(
        claim_id: u64,
        subject: EntityId,
        commodity: CommodityKind,
        quantity: u16,
        confidence: u16,
        acquired_tick: Tick,
    ) -> EntityBeliefClaim {
        EntityBeliefClaim {
            claim_id: ClaimId(claim_id),
            subject,
            aspect: EntityBeliefAspect::Inventory(commodity),
            value: ClaimValue::Quantity(Quantity(u32::from(quantity))),
            source: PerceptionSource::DirectObservation,
            acquired_tick,
            claimed_event_tick: Some(acquired_tick),
            confidence: Permille::new(confidence).unwrap(),
            refuted_at_tick: None,
        }
    }

    fn store_with_inventory_claim(
        subject: EntityId,
        commodity: CommodityKind,
        quantity: u16,
        confidence: u16,
        acquired_tick: Tick,
    ) -> AgentBeliefStore {
        let mut store = AgentBeliefStore::new();
        store
            .known_entities
            .insert(subject, belief(subject, commodity, quantity));
        store.record_entity_claim(inventory_claim(
            1,
            subject,
            commodity,
            quantity,
            confidence,
            acquired_tick,
        ));
        store
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
            by_effect_op: BTreeMap::from([(
                EffectFactKey::CommodityTransfer,
                BTreeSet::from([PlannerOpKind::MoveCargo]),
            )]),
        }
    }

    fn multi_op_index() -> EffectSchemaIndex {
        EffectSchemaIndex {
            by_effect: BTreeMap::from([(
                EffectFactKey::CommodityTransfer,
                vec![
                    worldwake_core::ActionDefId(0),
                    worldwake_core::ActionDefId(1),
                ],
            )]),
            by_effect_op: BTreeMap::from([(
                EffectFactKey::CommodityTransfer,
                BTreeSet::from([PlannerOpKind::Harvest, PlannerOpKind::Trade]),
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
    fn compile_opportunities_does_not_anchor_acquisition_on_self_inventory() {
        let mut store = AgentBeliefStore::new();
        store
            .known_entities
            .insert(entity(2), belief(entity(2), CommodityKind::Water, 3));
        let (world, agent) = view_with_store(
            store,
            CognitiveProfile::default(),
            PerceptionProfile::default(),
            RiskWeightProfile::default(),
            LawAbidingProfile::default(),
            SurveyMemory::default(),
            LearnedOpportunityMemory::default(),
        );
        let mut store = world
            .get_component_agent_belief_store(agent)
            .unwrap()
            .clone();
        store
            .known_entities
            .insert(agent, belief(agent, CommodityKind::Bread, 2));
        let mut world = world;
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
            txn.set_component_agent_belief_store(agent, store).unwrap();
            let mut event_log = worldwake_core::EventLog::new();
            txn.commit(&mut event_log);
        }
        let view = PerAgentBeliefView::from_world(agent, &world);

        let (opportunities, load) = compile_opportunities(agent, &view, &index());

        assert_eq!(load.compiled_count, 1);
        assert_eq!(opportunities.len(), 1);
        assert_eq!(
            opportunities[0].key.anchor,
            OpportunityAnchor::Entity(entity(2))
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
        assert_eq!(load.compiled_count, 2);
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

    fn compile_single_status(store: AgentBeliefStore, tick: Tick) -> BeliefStatusTag {
        let (world, agent) = view_with_store(
            store,
            CognitiveProfile::default(),
            PerceptionProfile::default(),
            RiskWeightProfile::default(),
            LawAbidingProfile::default(),
            SurveyMemory::default(),
            LearnedOpportunityMemory::default(),
        );
        let view = PerAgentBeliefView::from_world_at_tick(agent, tick, &world);
        let (opportunities, load) = compile_opportunities(agent, &view, &index());

        assert_eq!(load.compiled_count, 1);
        assert_eq!(opportunities.len(), 1);
        opportunities[0].source_belief.status
    }

    #[test]
    fn compile_opportunities_emits_certain_status_for_high_confidence_observation() {
        let status = compile_single_status(
            store_with_inventory_claim(entity(2), CommodityKind::Bread, 3, 950, Tick(10)),
            Tick(10),
        );

        assert_eq!(status, BeliefStatusTag::Certain);
    }

    #[test]
    fn compile_opportunities_emits_probable_status_for_threshold_observation() {
        let status = compile_single_status(
            store_with_inventory_claim(entity(2), CommodityKind::Bread, 3, 75, Tick(10)),
            Tick(10),
        );

        assert_eq!(status, BeliefStatusTag::Probable);
    }

    #[test]
    fn compile_opportunities_emits_stale_status_for_decayed_observation() {
        let status = compile_single_status(
            store_with_inventory_claim(entity(2), CommodityKind::Bread, 3, 60, Tick(10)),
            Tick(12),
        );

        assert_eq!(status, BeliefStatusTag::Stale);
    }

    #[test]
    fn compile_opportunities_emits_disputed_status_for_competing_inventory_claims() {
        let subject = entity(2);
        let mut store = store_with_inventory_claim(subject, CommodityKind::Bread, 3, 780, Tick(10));
        let mut competing = inventory_claim(2, subject, CommodityKind::Bread, 4, 780, Tick(10));
        competing.source = PerceptionSource::Report {
            from: entity(90),
            chain_len: 1,
        };
        store.record_entity_claim(competing);

        let status = compile_single_status(store, Tick(10));

        assert_eq!(status, BeliefStatusTag::Disputed);
    }

    #[test]
    fn compile_opportunities_emits_contradicted_status_for_refuted_claim() {
        let subject = entity(2);
        let mut store = AgentBeliefStore::new();
        store
            .known_entities
            .insert(subject, belief(subject, CommodityKind::Bread, 3));
        let mut claim = inventory_claim(1, subject, CommodityKind::Bread, 3, 950, Tick(10));
        claim.refuted_at_tick = Some(Tick(11));
        store.record_entity_claim(claim);

        let status = compile_single_status(store, Tick(11));

        assert_eq!(status, BeliefStatusTag::Contradicted);
    }

    #[test]
    fn compile_opportunities_records_per_status_distribution() {
        let mut store = AgentBeliefStore::new();
        for (slot, confidence) in [(2, 950), (3, 60)] {
            let subject = entity(slot);
            store
                .known_entities
                .insert(subject, belief(subject, CommodityKind::Bread, 3));
            store.record_entity_claim(inventory_claim(
                u64::from(slot),
                subject,
                CommodityKind::Bread,
                3,
                confidence,
                Tick(10),
            ));
        }
        let (world, agent) = view_with_store(
            store,
            CognitiveProfile::default(),
            PerceptionProfile::default(),
            RiskWeightProfile::default(),
            LawAbidingProfile::default(),
            SurveyMemory::default(),
            LearnedOpportunityMemory::default(),
        );
        let view = PerAgentBeliefView::from_world_at_tick(agent, Tick(12), &world);

        let (_opportunities, load) = compile_opportunities(agent, &view, &index());

        assert_eq!(load.compiled_count, 2);
        assert_eq!(load.compiled_by_status.len(), 2);
        assert_eq!(
            load.compiled_by_status.get(&BeliefStatusTag::Certain),
            Some(&1)
        );
        assert_eq!(
            load.compiled_by_status.get(&BeliefStatusTag::Stale),
            Some(&1)
        );
        assert_eq!(
            load.compiled_by_status.values().sum::<u32>(),
            load.compiled_count
        );
    }

    #[test]
    fn compile_opportunities_emits_same_keys_across_status_variations() {
        let mut mixed = AgentBeliefStore::new();
        let mut certain = AgentBeliefStore::new();
        for slot in 2..5 {
            let subject = entity(slot);
            mixed
                .known_entities
                .insert(subject, belief(subject, CommodityKind::Bread, 3));
            certain
                .known_entities
                .insert(subject, belief(subject, CommodityKind::Bread, 3));
            mixed.record_entity_claim(inventory_claim(
                u64::from(slot),
                subject,
                CommodityKind::Bread,
                3,
                if slot == 2 { 60 } else { 950 },
                Tick(10),
            ));
            certain.record_entity_claim(inventory_claim(
                u64::from(slot),
                subject,
                CommodityKind::Bread,
                3,
                950,
                Tick(10),
            ));
        }

        let (mixed_world, mixed_agent) = view_with_store(
            mixed,
            CognitiveProfile::default(),
            PerceptionProfile::default(),
            RiskWeightProfile::default(),
            LawAbidingProfile::default(),
            SurveyMemory::default(),
            LearnedOpportunityMemory::default(),
        );
        let mixed_view =
            PerAgentBeliefView::from_world_at_tick(mixed_agent, Tick(12), &mixed_world);
        let (mixed_opportunities, _) = compile_opportunities(mixed_agent, &mixed_view, &index());

        let (certain_world, certain_agent) = view_with_store(
            certain,
            CognitiveProfile::default(),
            PerceptionProfile::default(),
            RiskWeightProfile::default(),
            LawAbidingProfile::default(),
            SurveyMemory::default(),
            LearnedOpportunityMemory::default(),
        );
        let certain_view =
            PerAgentBeliefView::from_world_at_tick(certain_agent, Tick(12), &certain_world);
        let (certain_opportunities, _) =
            compile_opportunities(certain_agent, &certain_view, &index());

        let mixed_keys = mixed_opportunities
            .iter()
            .map(|opportunity| opportunity.key)
            .collect::<BTreeSet<_>>();
        let certain_keys = certain_opportunities
            .iter()
            .map(|opportunity| opportunity.key)
            .collect::<BTreeSet<_>>();

        assert_eq!(mixed_keys, certain_keys);
    }

    #[test]
    fn compile_opportunities_cap_truncation_is_deterministic_under_status_tie_break() {
        let mut store = AgentBeliefStore::new();
        for slot in 2..6 {
            let subject = entity(slot);
            store
                .known_entities
                .insert(subject, belief(subject, CommodityKind::Bread, 3));
            store.record_entity_claim(inventory_claim(
                u64::from(slot),
                subject,
                CommodityKind::Bread,
                3,
                if slot % 2 == 0 { 60 } else { 950 },
                Tick(10),
            ));
        }
        let cognitive = CognitiveProfile {
            compile_opportunity_cap: 2,
            ..CognitiveProfile::default()
        };
        let (world, agent) = view_with_store(
            store,
            cognitive,
            PerceptionProfile::default(),
            RiskWeightProfile::default(),
            LawAbidingProfile::default(),
            SurveyMemory::default(),
            LearnedOpportunityMemory::default(),
        );
        let view = PerAgentBeliefView::from_world_at_tick(agent, Tick(12), &world);

        let (opportunities, load) = compile_opportunities(agent, &view, &index());

        assert_eq!(load.compiled_count, 2);
        assert_eq!(load.cap_truncated, 2);
        assert_eq!(
            load.compiled_by_status.values().sum::<u32>(),
            load.compiled_count
        );
        assert_eq!(
            opportunities
                .iter()
                .map(|opportunity| opportunity.key.anchor)
                .collect::<Vec<_>>(),
            vec![
                OpportunityAnchor::Entity(entity(2)),
                OpportunityAnchor::Entity(entity(3))
            ]
        );
        assert_eq!(
            opportunities
                .iter()
                .map(|opportunity| opportunity.source_belief.status)
                .collect::<Vec<_>>(),
            vec![BeliefStatusTag::Stale, BeliefStatusTag::Certain]
        );
    }

    #[test]
    fn compile_opportunities_emits_derived_required_actions() {
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

        let (opportunities, load) = compile_opportunities(agent, &view, &multi_op_index());

        assert_eq!(load.compiled_count, 1);
        assert_eq!(
            opportunities[0].required_actions,
            vec![PlannerOpKind::Trade, PlannerOpKind::Harvest]
        );
    }
}
