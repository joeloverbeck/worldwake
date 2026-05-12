use worldwake_core::{
    GoalKind, HomeostaticNeedId, MotiveSource, MotiveSourceRef, OpportunityAnchor, OpportunityKey,
    Tick,
};

pub fn derive_default_motive_sources(
    goal_kind: &GoalKind,
    anchor: &OpportunityAnchor,
    introduced_tick: Tick,
) -> Vec<MotiveSourceRef> {
    let mut sources = Vec::new();
    let source = match goal_kind {
        GoalKind::AcquireCommodity {
            commodity,
            purpose: worldwake_core::CommodityPurpose::SelfConsume,
            ..
        } => {
            let need = need_for_commodity(*commodity).unwrap_or(HomeostaticNeedId::Hunger);
            sources.push(MotiveSourceRef {
                source: MotiveSource::NeedPressure { need },
                introduced_tick,
            });
            sources.push(MotiveSourceRef {
                source: MotiveSource::Greed {
                    opportunity: OpportunityKey {
                        goal_key: (*goal_kind).into(),
                        anchor: *anchor,
                    },
                },
                introduced_tick,
            });
            return sources;
        }
        GoalKind::ConsumeOwnedCommodity { commodity } => MotiveSource::NeedPressure {
            need: need_for_commodity(*commodity).unwrap_or(HomeostaticNeedId::Hunger),
        },
        GoalKind::Sleep => MotiveSource::NeedPressure {
            need: HomeostaticNeedId::Fatigue,
        },
        GoalKind::Relieve => MotiveSource::NeedPressure {
            need: HomeostaticNeedId::Bladder,
        },
        GoalKind::Wash => MotiveSource::NeedPressure {
            need: HomeostaticNeedId::Dirtiness,
        },
        GoalKind::ClaimOffice { office } => MotiveSource::OfficeDuty { office: *office },
        GoalKind::SupportCandidateForOffice { candidate, .. } => {
            MotiveSource::Loyalty { other: *candidate }
        }
        GoalKind::Accuse { violation_id, .. }
        | GoalKind::InvestigateViolation { violation_id, .. } => MotiveSource::Revenge {
            violation: *violation_id,
        },
        _ => MotiveSource::Greed {
            opportunity: OpportunityKey {
                goal_key: (*goal_kind).into(),
                anchor: *anchor,
            },
        },
    };
    sources.push(MotiveSourceRef {
        source,
        introduced_tick,
    });
    sources
}

fn need_for_commodity(commodity: worldwake_core::CommodityKind) -> Option<HomeostaticNeedId> {
    match commodity {
        worldwake_core::CommodityKind::Apple
        | worldwake_core::CommodityKind::Grain
        | worldwake_core::CommodityKind::Bread => Some(HomeostaticNeedId::Hunger),
        worldwake_core::CommodityKind::Water => Some(HomeostaticNeedId::Thirst),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_core::{
        AcquisitionQuantity, CommodityKind, CommodityPurpose, EntityId, GoalKey, Tick,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn only_source(goal_kind: GoalKind) -> MotiveSource {
        let sources = derive_default_motive_sources(&goal_kind, &OpportunityAnchor::None, Tick(7));
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].introduced_tick, Tick(7));
        sources[0].source.clone()
    }

    fn sources(goal_kind: GoalKind) -> Vec<MotiveSource> {
        derive_default_motive_sources(&goal_kind, &OpportunityAnchor::None, Tick(7))
            .into_iter()
            .map(|source| {
                assert_eq!(source.introduced_tick, Tick(7));
                source.source
            })
            .collect()
    }

    #[test]
    fn maps_core_need_goals_to_need_pressure_sources() {
        assert_eq!(
            sources(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            vec![
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Hunger
                },
                MotiveSource::Greed {
                    opportunity: OpportunityKey {
                        goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                            commodity: CommodityKind::Bread,
                            purpose: CommodityPurpose::SelfConsume,
                            quantity: AcquisitionQuantity::single(),
                        }),
                        anchor: OpportunityAnchor::None,
                    }
                },
            ]
        );
        assert_eq!(
            sources(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            vec![
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Thirst
                },
                MotiveSource::Greed {
                    opportunity: OpportunityKey {
                        goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                            commodity: CommodityKind::Water,
                            purpose: CommodityPurpose::SelfConsume,
                            quantity: AcquisitionQuantity::single(),
                        }),
                        anchor: OpportunityAnchor::None,
                    }
                },
            ]
        );
        assert_eq!(
            only_source(GoalKind::Sleep),
            MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Fatigue
            }
        );
        assert_eq!(
            only_source(GoalKind::Relieve),
            MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Bladder
            }
        );
        assert_eq!(
            only_source(GoalKind::Wash),
            MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Dirtiness
            }
        );
    }

    #[test]
    fn maps_social_and_violation_goals_to_specific_sources() {
        let office = entity(10);
        let candidate = entity(11);
        assert_eq!(
            only_source(GoalKind::ClaimOffice { office }),
            MotiveSource::OfficeDuty { office }
        );
        assert_eq!(
            only_source(GoalKind::SupportCandidateForOffice { office, candidate }),
            MotiveSource::Loyalty { other: candidate }
        );
        assert!(matches!(
            only_source(GoalKind::Accuse {
                crime_register: entity(12),
                accused: entity(13),
                violation_id: worldwake_core::ViolationId(14),
            }),
            MotiveSource::Revenge {
                violation: worldwake_core::ViolationId(14)
            }
        ));
    }

    #[test]
    fn maps_remaining_goals_to_opportunity_backed_greed() {
        let goal_kind = GoalKind::PostNotice {
            posting: worldwake_core::ArtifactPostingContext {
                posting_place: entity(20),
                issuing_authority: Some(entity(21)),
                expires_at: Some(Tick(99)),
                jurisdiction: Some(entity(22)),
            },
            topic: worldwake_core::NoticeTopic::ThreatWarning { place: entity(23) },
        };
        let anchor = OpportunityAnchor::Place(entity(20));
        let sources = derive_default_motive_sources(&goal_kind, &anchor, Tick(9));
        assert_eq!(
            sources[0].source,
            MotiveSource::Greed {
                opportunity: OpportunityKey {
                    goal_key: GoalKey::from(goal_kind),
                    anchor,
                }
            }
        );
    }
}
