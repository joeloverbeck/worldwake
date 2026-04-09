use std::collections::{BTreeMap, BTreeSet};

use worldwake_core::{
    CauseRef, EligibilityRule, EntityId, EntityKind, EventLog, EventTag, InstitutionalClaim,
    OfficeData, OfficeForceProfile, OfficeForceState, Permille, RecordEntryId, RecordKind,
    SuccessionLaw, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
};
use worldwake_sim::{
    ForceCandidateTrace, ForceInstallationDeferralReason, OfficeAvailabilityPhase,
    OfficeSuccessionOutcome, OfficeSuccessionTrace, PoliticalTraceEvent, PoliticalTraceSink,
    SupportCountTrace, SupportDeclarationTrace, SupportResolutionTrace, SystemError,
    SystemExecutionContext, VacancyTimerTrace,
};

const PUBLIC_ORDER_BASELINE: Permille = Permille::new_unchecked(750);
const VACANT_OFFICE_PENALTY: Permille = Permille::new_unchecked(200);
const HOSTILE_FACTION_PAIR_PENALTY: Permille = Permille::new_unchecked(100);
const GUARD_PRESENCE_BONUS: Permille = Permille::new_unchecked(50);
const MAX_GUARD_ORDER_BONUS: Permille = Permille::new_unchecked(200);

pub fn succession_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions: _active_actions,
        action_defs: _action_defs,
        mut politics_trace,
        perception_trace: _,
        tick,
        system_id: _system_id,
    } = ctx;

    let offices = world
        .query_office_data()
        .map(|(office, office_data)| (office, office_data.clone()))
        .collect::<Vec<_>>();

    for (office, office_data) in offices {
        evaluate_office_succession(
            world,
            event_log,
            tick,
            office,
            &office_data,
            &mut politics_trace,
        )?;
    }

    Ok(())
}

fn evaluate_office_succession(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
    office: EntityId,
    office_data: &OfficeData,
    politics_trace: &mut Option<&mut PoliticalTraceSink>,
) -> Result<(), SystemError> {
    if let Some(holder) = living_holder(world, office) {
        let cleared_stale_vacancy = office_data.vacancy_since.is_some();
        if office_data.vacancy_since.is_some() {
            let mut next = office_data.clone();
            next.vacancy_since = None;
            commit_hidden_office_update(world, event_log, tick, office, next)?;
        }
        record_political_trace(
            politics_trace,
            office_trace_event(
                tick,
                office,
                office_data,
                OfficeSuccessionOutcome::OccupiedNoAction {
                    holder,
                    cleared_stale_vacancy,
                },
                Vec::new(),
                None,
                Vec::new(),
            ),
        );
        return Ok(());
    }

    if office_data.vacancy_since.is_none() {
        let mut txn = new_political_txn(world, tick, Some(office_data.seat));
        let mut next = office_data.clone();
        next.vacancy_since = Some(tick);
        txn.set_component_office_data(office, next)
            .map_err(|error| SystemError::new(error.to_string()))?;
        txn.vacate_office(office)
            .map_err(|error| SystemError::new(error.to_string()))?;
        txn.add_target(office);
        let _ = txn.commit(event_log);
        record_political_trace(
            politics_trace,
            office_trace_event(
                tick,
                office,
                office_data,
                OfficeSuccessionOutcome::VacancyActivated,
                support_resolution_trace(office, office_data, world).declarations,
                None,
                force_candidate_traces(office_data, world),
            ),
        );
        return Ok(());
    }

    match office_data.succession_law {
        SuccessionLaw::Support => {
            resolve_support_succession(world, event_log, tick, office, office_data, politics_trace)
        }
        SuccessionLaw::Force => {
            resolve_force_succession(world, event_log, tick, office, office_data, politics_trace)
        }
    }
}

pub fn offices_with_jurisdiction(place: EntityId, world: &World) -> Vec<EntityId> {
    world
        .query_office_data()
        .filter_map(|(office, office_data)| {
            office_data.jurisdiction.contains(&place).then_some(office)
        })
        .collect()
}

pub fn office_is_vacant(office: EntityId, world: &World) -> bool {
    world.entity_kind(office) == Some(EntityKind::Office) && living_holder(world, office).is_none()
}

pub fn public_order(place: EntityId, world: &World) -> Permille {
    let mut order = PUBLIC_ORDER_BASELINE;

    for office in offices_with_jurisdiction(place, world) {
        if office_is_vacant(office, world) {
            order = order.saturating_sub(VACANT_OFFICE_PENALTY);
        }
    }

    for _ in 0..count_present_hostile_faction_pairs_at(place, world) {
        order = order.saturating_sub(HOSTILE_FACTION_PAIR_PENALTY);
    }

    order = order.saturating_add(guard_presence_factor(place, world));

    order
}

fn guard_presence_factor(place: EntityId, world: &World) -> Permille {
    let patrolling_guards = world
        .entities_effectively_at(place)
        .into_iter()
        .filter(|entity| world.entity_kind(*entity) == Some(EntityKind::Agent))
        .filter(|entity| world.get_component_patrol_route(*entity).is_some())
        .count() as u32;
    let bonus = patrolling_guards.saturating_mul(u32::from(GUARD_PRESENCE_BONUS.value()));
    let capped_bonus = bonus.min(u32::from(MAX_GUARD_ORDER_BONUS.value()));

    Permille::new(capped_bonus as u16).expect("guard presence bonus stays within Permille bounds")
}

pub fn count_present_hostile_faction_pairs_at(place: EntityId, world: &World) -> usize {
    let present_factions = present_factions_at(place, world)
        .into_iter()
        .collect::<Vec<_>>();
    let mut count = 0;

    for (index, faction_a) in present_factions.iter().enumerate() {
        for faction_b in present_factions.iter().skip(index + 1) {
            if factions_are_hostile(*faction_a, *faction_b, world) {
                count += 1;
            }
        }
    }

    count
}

pub fn eligible_agents_at(office: EntityId, place: EntityId, world: &World) -> Vec<EntityId> {
    let Some(office_data) = world.get_component_office_data(office) else {
        return Vec::new();
    };

    world
        .entities_effectively_at(place)
        .into_iter()
        .filter(|entity| world.entity_kind(*entity) == Some(EntityKind::Agent))
        .filter(|entity| world.get_component_dead_at(*entity).is_none())
        .filter(|entity| candidate_is_eligible(world, office_data, *entity))
        .collect()
}

pub(crate) fn candidate_is_eligible(
    world: &World,
    office: &OfficeData,
    candidate: EntityId,
) -> bool {
    world.entity_kind(candidate) == Some(EntityKind::Agent)
        && world.get_component_dead_at(candidate).is_none()
        && office.eligibility_rules.iter().all(|rule| match rule {
            EligibilityRule::FactionMember(faction) => {
                world.factions_of(candidate).contains(faction)
            }
        })
}

fn resolve_support_succession(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
    office: EntityId,
    office_data: &OfficeData,
    politics_trace: &mut Option<&mut PoliticalTraceSink>,
) -> Result<(), SystemError> {
    let start_tick = office_data
        .vacancy_since
        .expect("support succession requires active vacancy_since");
    let waited_ticks = tick.0.saturating_sub(start_tick.0);
    if waited_ticks < office_data.succession_period_ticks {
        let support_resolution = support_resolution_trace(office, office_data, world);
        let support_declarations = support_resolution.declarations;
        let outcome = OfficeSuccessionOutcome::WaitingForTimer;
        record_political_trace(
            politics_trace,
            office_trace_event(
                tick,
                office,
                office_data,
                outcome,
                support_declarations,
                Some(SupportResolutionTrace {
                    counted_support: support_resolution.counted_support,
                }),
                force_candidate_traces(office_data, world),
            ),
        );
        return Ok(());
    }

    let support_resolution = support_resolution_trace(office, office_data, world);
    let support_declarations = support_resolution.declarations;
    let counted_support = support_resolution.counted_support;
    let counts = counted_support_by_candidate(&counted_support);

    let Some(max_support) = counts.values().copied().max() else {
        reset_vacancy_clock(world, event_log, tick, office, office_data)?;
        record_political_trace(
            politics_trace,
            office_trace_event(
                tick,
                office,
                office_data,
                OfficeSuccessionOutcome::SupportResetNoEligibleDeclarations,
                support_declarations,
                Some(SupportResolutionTrace { counted_support }),
                Vec::new(),
            ),
        );
        return Ok(());
    };

    let winners = counts
        .into_iter()
        .filter_map(|(candidate, support)| (support == max_support).then_some(candidate))
        .collect::<Vec<_>>();

    if winners.len() != 1 {
        let outcome = OfficeSuccessionOutcome::SupportResetTie {
            tied_candidates: winners.clone(),
        };
        reset_vacancy_clock(world, event_log, tick, office, office_data)?;
        record_political_trace(
            politics_trace,
            office_trace_event(
                tick,
                office,
                office_data,
                outcome,
                support_declarations,
                Some(SupportResolutionTrace {
                    counted_support: counted_support.clone(),
                }),
                Vec::new(),
            ),
        );
        return Ok(());
    }

    let holder = winners[0];
    install_office_holder(world, event_log, tick, office, office_data, holder)?;
    let outcome = OfficeSuccessionOutcome::SupportInstalled { holder };
    record_political_trace(
        politics_trace,
        office_trace_event(
            tick,
            office,
            office_data,
            outcome,
            support_declarations,
            Some(SupportResolutionTrace { counted_support }),
            Vec::new(),
        ),
    );
    Ok(())
}

fn resolve_force_succession(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
    office: EntityId,
    office_data: &OfficeData,
    politics_trace: &mut Option<&mut PoliticalTraceSink>,
) -> Result<(), SystemError> {
    let context = build_force_succession_context(world, office, office_data)?;
    let resolution = evaluate_force_resolution(tick, &context);

    // Check whether installation is possible: desired_controller exists and
    // all gate conditions pass.
    let installation_deferral = resolution.desired_controller.and_then(|controller| {
        check_force_installation_gate(controller, &context, &resolution, tick)
            .map(|reason| (controller, reason))
    });

    if let Some(controller) = resolution
        .desired_controller
        .filter(|_| installation_deferral.is_none())
    {
        install_force_office_holder(
            world,
            event_log,
            tick,
            office_data,
            office,
            &context,
            &resolution,
        )?;
        let outcome = OfficeSuccessionOutcome::ForceInstalled { holder: controller };
        record_political_trace(
            politics_trace,
            office_trace_event(
                tick,
                office,
                office_data,
                outcome,
                Vec::new(),
                None,
                context.force_candidates,
            ),
        );
        return Ok(());
    }

    commit_force_control_update(world, event_log, tick, office, &context, &resolution)?;

    // Always emit the resolution outcome (e.g., ForceControllerMaintained).
    let resolution_candidates = context.force_candidates.clone();
    record_political_trace(
        politics_trace,
        office_trace_event(
            tick,
            office,
            office_data,
            resolution.outcome,
            Vec::new(),
            None,
            resolution_candidates,
        ),
    );

    // If the gate blocked installation, emit an additional deferral trace
    // explaining WHY the controller was not installed despite being desired.
    if let Some((controller, reason)) = installation_deferral {
        record_political_trace(
            politics_trace,
            office_trace_event(
                tick,
                office,
                office_data,
                OfficeSuccessionOutcome::ForceInstallationDeferred { controller, reason },
                Vec::new(),
                None,
                context.force_candidates,
            ),
        );
    }

    Ok(())
}

fn build_force_succession_context(
    world: &World,
    office: EntityId,
    office_data: &OfficeData,
) -> Result<ForceSuccessionContext, SystemError> {
    let profile = world
        .get_component_office_force_profile(office)
        .cloned()
        .ok_or_else(|| {
            SystemError::new(format!("force office {office} lacks OfficeForceProfile"))
        })?;
    let prior_state = world
        .get_component_office_force_state(office)
        .cloned()
        .ok_or_else(|| SystemError::new(format!("force office {office} lacks OfficeForceState")))?;
    let raw_claimants = world.force_claimants_for_office_including_dead(office);
    let dead_claimants = raw_claimants
        .iter()
        .copied()
        .filter(|claimant| world.get_component_dead_at(*claimant).is_some())
        .collect::<Vec<_>>();
    let live_claimants = raw_claimants
        .iter()
        .copied()
        .filter(|claimant| world.get_component_dead_at(*claimant).is_none())
        .collect::<Vec<_>>();
    let present_claimants = live_claimants
        .iter()
        .copied()
        .filter(|claimant| world.effective_place(*claimant) == Some(office_data.seat))
        .filter(|claimant| candidate_is_eligible(world, office_data, *claimant))
        .collect::<Vec<_>>();
    let current_controller = world
        .office_controller(office)
        .filter(|controller| world.effective_place(*controller) == Some(office_data.seat))
        .filter(|controller| candidate_is_eligible(world, office_data, *controller))
        .filter(|controller| present_claimants.contains(controller));

    Ok(ForceSuccessionContext {
        profile,
        prior_state,
        vacancy_since: office_data.vacancy_since,
        current_controller,
        raw_claimants,
        dead_claimants,
        live_claimants,
        present_claimants,
        force_candidates: force_candidate_traces(office_data, world),
    })
}

fn evaluate_force_resolution(tick: Tick, context: &ForceSuccessionContext) -> ForceResolution {
    let mut next_state = context.prior_state.clone();
    let mut desired_controller = context.current_controller;
    let outcome = if let Some(controller) = context.current_controller {
        let challenger_count = context
            .present_claimants
            .iter()
            .filter(|claimant| **claimant != controller)
            .count();
        if challenger_count == 0 {
            next_state.challenged_since = None;
            next_state.contested_since = None;
            next_state.last_uncontested_tick = Some(tick);
            if next_state.control_since.is_none() {
                next_state.control_since = Some(tick);
            }
            OfficeSuccessionOutcome::ForceControllerMaintained { controller }
        } else {
            next_state.last_uncontested_tick = None;
            next_state.contested_since = None;
            let challenged_since = next_state.challenged_since.get_or_insert(tick);
            let waited_ticks = tick.0.saturating_sub(challenged_since.0) + 1;
            let required_ticks = u64::from(context.profile.challenger_presence_grace_ticks.get());
            if waited_ticks < required_ticks {
                OfficeSuccessionOutcome::ForceChallengerGracePending {
                    controller,
                    challenger_count,
                    waited_ticks,
                    required_ticks,
                }
            } else {
                desired_controller = None;
                next_state.control_since = None;
                next_state.challenged_since = None;
                next_state.contested_since = Some(tick);
                OfficeSuccessionOutcome::ForceContested {
                    claimant_count: context.present_claimants.len(),
                }
            }
        }
    } else {
        match context.present_claimants.as_slice() {
            [] => {
                desired_controller = None;
                reset_force_state(&mut next_state);
                OfficeSuccessionOutcome::ForceNoClaimants
            }
            [claimant] => {
                next_state.challenged_since = None;
                next_state.contested_since = None;
                next_state.last_uncontested_tick = None;
                let waited_ticks = context
                    .vacancy_since
                    .map(|vacancy_since| tick.0.saturating_sub(vacancy_since.0))
                    .unwrap_or_default();
                let required_ticks = u64::from(context.profile.vacancy_claim_grace_ticks.get());
                if waited_ticks < required_ticks {
                    desired_controller = None;
                    next_state.control_since = None;
                    OfficeSuccessionOutcome::ForceVacancyClaimGracePending {
                        claimant: *claimant,
                        waited_ticks,
                        required_ticks,
                    }
                } else {
                    desired_controller = Some(*claimant);
                    next_state.control_since = Some(tick);
                    next_state.last_uncontested_tick = Some(tick);
                    OfficeSuccessionOutcome::ForceControllerEstablished {
                        controller: *claimant,
                    }
                }
            }
            claimants => {
                desired_controller = None;
                next_state.control_since = None;
                next_state.challenged_since = None;
                next_state.last_uncontested_tick = None;
                next_state.contested_since = next_state.contested_since.or(Some(tick));
                OfficeSuccessionOutcome::ForceContested {
                    claimant_count: claimants.len(),
                }
            }
        }
    };

    ForceResolution {
        desired_controller,
        next_state,
        outcome,
    }
}

fn install_office_holder(
    world: &mut World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    office: EntityId,
    office_data: &OfficeData,
    holder: EntityId,
) -> Result<(), SystemError> {
    let mut txn = new_political_txn(world, tick, Some(office_data.seat));
    stage_office_holder_install(&mut txn, office, office_data, holder)?;
    let _ = txn.commit(event_log);
    Ok(())
}

fn install_force_office_holder(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
    office_data: &OfficeData,
    office: EntityId,
    context: &ForceSuccessionContext,
    resolution: &ForceResolution,
) -> Result<(), SystemError> {
    let holder = resolution
        .desired_controller
        .expect("force installation requires a controller");
    let mut txn = new_political_txn(world, tick, Some(office_data.seat));
    stage_office_holder_install(&mut txn, office, office_data, holder)?;
    for claimant in &context.raw_claimants {
        txn.remove_force_claim(*claimant, office)
            .map_err(|error| SystemError::new(error.to_string()))?;
    }
    txn.clear_office_controller(office)
        .map_err(|error| SystemError::new(error.to_string()))?;
    let cleared_state = cleared_force_state();
    if resolution.next_state != cleared_state {
        txn.set_component_office_force_state(office, cleared_state)
            .map_err(|error| SystemError::new(error.to_string()))?;
    }
    stage_force_control_record_update(
        &mut txn,
        office,
        office_data.seat,
        force_control_claim(office, None, false, tick),
    )?;
    txn.add_target(office).add_target(holder);
    let _ = txn.commit(event_log);
    Ok(())
}

fn commit_force_control_update(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
    office: EntityId,
    context: &ForceSuccessionContext,
    resolution: &ForceResolution,
) -> Result<(), SystemError> {
    let controller_changed = resolution.desired_controller != context.current_controller
        || (resolution.desired_controller.is_none() && context.prior_state.control_since.is_some());
    let state_changed = resolution.next_state != context.prior_state;
    if !controller_changed && !state_changed && context.dead_claimants.is_empty() {
        return Ok(());
    }

    let seat = world
        .get_component_office_data(office)
        .map(|office_data| office_data.seat);
    let seat = seat.unwrap_or_else(|| {
        world
            .get_component_office_data(office)
            .expect("office should still have OfficeData")
            .seat
    });
    let mut txn = new_political_txn(world, tick, Some(seat));
    if controller_changed {
        if let Some(controller) = resolution.desired_controller {
            txn.set_office_controller(office, controller)
                .map_err(|error| SystemError::new(error.to_string()))?;
            txn.add_target(controller);
        } else {
            txn.clear_office_controller(office)
                .map_err(|error| SystemError::new(error.to_string()))?;
        }
    }
    if state_changed {
        txn.set_component_office_force_state(office, resolution.next_state.clone())
            .map_err(|error| SystemError::new(error.to_string()))?;
    }
    for claimant in &context.dead_claimants {
        txn.remove_force_claim(*claimant, office)
            .map_err(|error| SystemError::new(error.to_string()))?;
    }
    stage_force_control_record_update(
        &mut txn,
        office,
        seat,
        force_control_claim(
            office,
            resolution.desired_controller,
            resolution.next_state.contested_since.is_some(),
            tick,
        ),
    )?;
    txn.add_target(office);
    let _ = txn.commit(event_log);
    Ok(())
}

fn stage_office_holder_install(
    txn: &mut WorldTxn<'_>,
    office: EntityId,
    office_data: &OfficeData,
    holder: EntityId,
) -> Result<(), SystemError> {
    let mut next = office_data.clone();
    next.vacancy_since = None;
    txn.set_component_office_data(office, next)
        .map_err(|error| SystemError::new(error.to_string()))?;
    txn.assign_office(office, holder)
        .map_err(|error| SystemError::new(error.to_string()))?;
    txn.clear_support_declarations_for_office(office)
        .map_err(|error| SystemError::new(error.to_string()))?;
    txn.add_target(office).add_target(holder);
    Ok(())
}

fn force_control_claim(
    office: EntityId,
    controller: Option<EntityId>,
    contested: bool,
    tick: Tick,
) -> InstitutionalClaim {
    InstitutionalClaim::ForceControl {
        office,
        controller,
        contested,
        effective_tick: tick,
    }
}

fn stage_force_control_record_update(
    txn: &mut WorldTxn<'_>,
    office: EntityId,
    seat: EntityId,
    claim: InstitutionalClaim,
) -> Result<(), SystemError> {
    let Some(record) = unique_record_at_place(txn, seat, RecordKind::OfficeRegister)? else {
        return Ok(());
    };
    let current = active_force_control_entry(txn, record, office)?;
    if current
        .as_ref()
        .is_some_and(|(_, existing)| *existing == claim)
    {
        return Ok(());
    }

    match current {
        Some((entry_id, _)) => txn
            .supersede_record_entry(record, entry_id, claim)
            .map_err(|error| SystemError::new(error.to_string()))?,
        None => txn
            .append_record_entry(record, claim)
            .map_err(|error| SystemError::new(error.to_string()))?,
    };
    Ok(())
}

fn unique_record_at_place(
    txn: &WorldTxn<'_>,
    place: EntityId,
    kind: RecordKind,
) -> Result<Option<EntityId>, SystemError> {
    let matches = txn
        .query_record_data()
        .filter_map(|(entity, record)| {
            (record.home_place == place && record.record_kind == kind).then_some(entity)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [record] => Ok(Some(*record)),
        _ => Err(SystemError::new(format!(
            "multiple {kind:?} records at place {place}"
        ))),
    }
}

fn active_force_control_entry(
    txn: &WorldTxn<'_>,
    record: EntityId,
    office: EntityId,
) -> Result<Option<(RecordEntryId, InstitutionalClaim)>, SystemError> {
    let record_data = txn
        .get_component_record_data(record)
        .ok_or_else(|| SystemError::new(format!("record {record} lacks RecordData")))?;
    let matches = record_data
        .active_entries()
        .into_iter()
        .filter_map(|entry| match entry.claim {
            InstitutionalClaim::ForceControl {
                office: claim_office,
                ..
            } if claim_office == office => Some((entry.entry_id, entry.claim)),
            _ => None,
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [entry] => Ok(Some(*entry)),
        _ => Err(SystemError::new(format!(
            "multiple active force-control entries for office {office} in record {record}"
        ))),
    }
}

/// Check whether the force installation gate allows installing this controller.
/// Returns `None` if installation may proceed, or `Some(reason)` explaining why
/// installation is deferred.
fn check_force_installation_gate(
    controller: EntityId,
    context: &ForceSuccessionContext,
    resolution: &ForceResolution,
    tick: Tick,
) -> Option<ForceInstallationDeferralReason> {
    let blocking_claimants: Vec<EntityId> = context
        .live_claimants
        .iter()
        .copied()
        .filter(|claimant| *claimant != controller)
        .collect();
    if !blocking_claimants.is_empty() {
        return Some(ForceInstallationDeferralReason::OtherLiveClaimants {
            controller,
            blocking_claimants,
        });
    }
    if !force_hold_complete(&context.profile, &resolution.next_state, tick) {
        let held_ticks = resolution
            .next_state
            .control_since
            .map_or(0, |since| tick.0.saturating_sub(since.0) + 1);
        return Some(ForceInstallationDeferralReason::HoldIncomplete {
            held_ticks,
            required_ticks: u64::from(context.profile.uncontested_hold_ticks.get()),
        });
    }
    if resolution.next_state.last_uncontested_tick != Some(tick) {
        return Some(ForceInstallationDeferralReason::NotUncontestedThisTick);
    }
    None
}

fn force_hold_complete(profile: &OfficeForceProfile, state: &OfficeForceState, tick: Tick) -> bool {
    let Some(control_since) = state.control_since else {
        return false;
    };
    let held_ticks = tick.0.saturating_sub(control_since.0) + 1;
    held_ticks >= u64::from(profile.uncontested_hold_ticks.get())
}

fn reset_force_state(state: &mut OfficeForceState) {
    *state = cleared_force_state();
}

fn cleared_force_state() -> OfficeForceState {
    OfficeForceState {
        control_since: None,
        challenged_since: None,
        contested_since: None,
        last_uncontested_tick: None,
    }
}

struct SupportResolutionSnapshot {
    declarations: Vec<SupportDeclarationTrace>,
    counted_support: Vec<SupportCountTrace>,
}

struct ForceSuccessionContext {
    profile: OfficeForceProfile,
    prior_state: OfficeForceState,
    vacancy_since: Option<Tick>,
    current_controller: Option<EntityId>,
    raw_claimants: Vec<EntityId>,
    dead_claimants: Vec<EntityId>,
    live_claimants: Vec<EntityId>,
    present_claimants: Vec<EntityId>,
    force_candidates: Vec<ForceCandidateTrace>,
}

struct ForceResolution {
    desired_controller: Option<EntityId>,
    next_state: OfficeForceState,
    outcome: OfficeSuccessionOutcome,
}

fn support_resolution_trace(
    office: EntityId,
    office_data: &OfficeData,
    world: &World,
) -> SupportResolutionSnapshot {
    let mut counted = BTreeMap::<EntityId, usize>::new();
    let declarations = world
        .support_declarations_for_office(office)
        .into_iter()
        .map(|(supporter, candidate)| {
            let candidate_eligible = candidate_is_eligible(world, office_data, candidate);
            if candidate_eligible {
                *counted.entry(candidate).or_default() += 1;
            }
            SupportDeclarationTrace {
                supporter,
                candidate,
                candidate_eligible,
                counted: candidate_eligible,
            }
        })
        .collect();
    let counted_support = counted
        .into_iter()
        .map(|(candidate, support)| SupportCountTrace { candidate, support })
        .collect();
    SupportResolutionSnapshot {
        declarations,
        counted_support,
    }
}

fn counted_support_by_candidate(
    counted_support: &[SupportCountTrace],
) -> BTreeMap<EntityId, usize> {
    counted_support
        .iter()
        .map(|entry| (entry.candidate, entry.support))
        .collect()
}

fn force_candidate_traces(office_data: &OfficeData, world: &World) -> Vec<ForceCandidateTrace> {
    world
        .entities_effectively_at(office_data.seat)
        .into_iter()
        .filter(|entity| world.entity_kind(*entity) == Some(EntityKind::Agent))
        .map(|candidate| ForceCandidateTrace {
            candidate,
            eligible: candidate_is_eligible(world, office_data, candidate),
        })
        .collect()
}

fn record_political_trace(sink: &mut Option<&mut PoliticalTraceSink>, event: PoliticalTraceEvent) {
    if let Some(sink) = sink.as_deref_mut() {
        sink.record(event);
    }
}

fn availability_phase_for_trace(
    outcome: &OfficeSuccessionOutcome,
    support_declarations: &[SupportDeclarationTrace],
) -> OfficeAvailabilityPhase {
    match outcome {
        OfficeSuccessionOutcome::OccupiedNoAction { .. }
        | OfficeSuccessionOutcome::SupportInstalled { .. }
        | OfficeSuccessionOutcome::ForceInstalled { .. } => OfficeAvailabilityPhase::ClosedOccupied,
        OfficeSuccessionOutcome::VacancyActivated | OfficeSuccessionOutcome::ForceNoClaimants => {
            OfficeAvailabilityPhase::VacantClaimable
        }
        OfficeSuccessionOutcome::WaitingForTimer => {
            if support_declarations.is_empty() {
                OfficeAvailabilityPhase::VacantWaitingForTimer
            } else {
                OfficeAvailabilityPhase::VacantPendingResolution
            }
        }
        OfficeSuccessionOutcome::ForceControllerEstablished { .. }
        | OfficeSuccessionOutcome::ForceVacancyClaimGracePending { .. }
        | OfficeSuccessionOutcome::ForceControllerMaintained { .. }
        | OfficeSuccessionOutcome::ForceChallengerGracePending { .. }
        | OfficeSuccessionOutcome::ForceContested { .. }
        | OfficeSuccessionOutcome::ForceInstallationDeferred { .. }
        | OfficeSuccessionOutcome::ForceBlocked { .. } => {
            OfficeAvailabilityPhase::VacantPendingResolution
        }
        OfficeSuccessionOutcome::SupportResetNoEligibleDeclarations
        | OfficeSuccessionOutcome::SupportResetTie { .. } => {
            OfficeAvailabilityPhase::VacantReopenedAfterReset
        }
    }
}

fn office_trace_event(
    tick: Tick,
    office: EntityId,
    office_data: &OfficeData,
    outcome: OfficeSuccessionOutcome,
    support_declarations: Vec<SupportDeclarationTrace>,
    support_resolution: Option<SupportResolutionTrace>,
    force_candidates: Vec<ForceCandidateTrace>,
) -> PoliticalTraceEvent {
    let availability_phase = availability_phase_for_trace(&outcome, &support_declarations);
    let holder_before = match &outcome {
        OfficeSuccessionOutcome::OccupiedNoAction { holder, .. } => Some(*holder),
        _ => None,
    };
    let vacancy_since_before = match &outcome {
        OfficeSuccessionOutcome::VacancyActivated => None,
        _ => office_data.vacancy_since,
    };
    let vacancy_timer = office_data
        .vacancy_since
        .map(|start_tick| VacancyTimerTrace {
            start_tick,
            waited_ticks: tick.0.saturating_sub(start_tick.0),
            required_ticks: office_data.succession_period_ticks,
            remaining_ticks: office_data
                .succession_period_ticks
                .saturating_sub(tick.0.saturating_sub(start_tick.0)),
        });
    PoliticalTraceEvent {
        tick,
        office,
        trace: OfficeSuccessionTrace {
            seat: office_data.seat,
            succession_law: office_data.succession_law.clone(),
            holder_before,
            vacancy_since_before,
            availability_phase,
            vacancy_timer,
            outcome,
            support_declarations,
            support_resolution,
            force_candidates,
        },
    }
}

fn reset_vacancy_clock(
    world: &mut World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    office: EntityId,
    office_data: &OfficeData,
) -> Result<(), SystemError> {
    let mut next = office_data.clone();
    next.vacancy_since = Some(tick);
    commit_hidden_office_update(world, event_log, tick, office, next)
}

fn commit_hidden_office_update(
    world: &mut World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    office: EntityId,
    office_data: OfficeData,
) -> Result<(), SystemError> {
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::System)
        .add_tag(EventTag::WorldMutation)
        .add_tag(EventTag::Political)
        .add_target(office);
    txn.set_component_office_data(office, office_data)
        .map_err(|error| SystemError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    Ok(())
}

fn new_political_txn(
    world: &mut World,
    tick: worldwake_core::Tick,
    place_id: Option<EntityId>,
) -> WorldTxn<'_> {
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        place_id,
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::System)
        .add_tag(EventTag::WorldMutation)
        .add_tag(EventTag::Political);
    txn
}

fn living_holder(world: &World, office: EntityId) -> Option<EntityId> {
    let holder = world.office_holder(office)?;
    (world.get_component_dead_at(holder).is_none()).then_some(holder)
}

fn present_factions_at(place: EntityId, world: &World) -> BTreeSet<EntityId> {
    world
        .entities_effectively_at(place)
        .into_iter()
        .filter(|entity| world.entity_kind(*entity) == Some(EntityKind::Agent))
        .flat_map(|entity| world.factions_of(entity))
        .collect()
}

fn factions_are_hostile(faction_a: EntityId, faction_b: EntityId, world: &World) -> bool {
    world.hostile_targets_of(faction_a).contains(&faction_b)
        || world.hostile_targets_of(faction_b).contains(&faction_a)
}

#[cfg(test)]
mod tests {
    use super::{
        candidate_is_eligible, count_present_hostile_faction_pairs_at, eligible_agents_at,
        office_is_vacant, offices_with_jurisdiction, public_order, succession_system,
    };
    use crate::dispatch_table;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        CauseRef, ControlSource, EntityId, EventLog, EventTag, EventView, OfficeData,
        OfficeForceProfile, OfficeForceState, PatrolRoute, Permille, RecordData, RecordKind, Seed,
        Tick, UtilityProfile, VisibilitySpec, WitnessData, World, WorldTxn, build_prototype_world,
    };
    use worldwake_sim::{
        ActionDefRegistry, DeterministicRng, ForceCandidateTrace, OfficeAvailabilityPhase,
        OfficeSuccessionOutcome, PoliticalTraceSink, SupportCountTrace, SupportResolutionTrace,
        SystemExecutionContext, SystemId, VacancyTimerTrace,
    };

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        )
    }

    fn run_succession(world: &mut World, event_log: &mut EventLog, tick: u64) {
        let mut rng = DeterministicRng::new(Seed([tick as u8; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();
        succession_system(SystemExecutionContext {
            world,
            event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(tick),
            system_id: SystemId::Politics,
        })
        .unwrap();
    }

    fn run_succession_with_trace(
        world: &mut World,
        event_log: &mut EventLog,
        trace: &mut PoliticalTraceSink,
        tick: u64,
    ) {
        let mut rng = DeterministicRng::new(Seed([tick as u8; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();
        succession_system(SystemExecutionContext {
            world,
            event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: Some(trace),
            perception_trace: None,
            tick: Tick(tick),
            system_id: SystemId::Politics,
        })
        .unwrap();
    }

    fn create_record(
        txn: &mut WorldTxn<'_>,
        place: EntityId,
        issuer: EntityId,
        kind: RecordKind,
    ) -> EntityId {
        txn.create_record(RecordData {
            record_kind: kind,
            home_place: place,
            issuer,
            consultation_ticks: 4,
            max_entries_per_consult: 6,
            entries: Vec::new(),
            next_entry_id: 0,
        })
        .unwrap()
    }

    fn record_at_place(world: &World, place: EntityId, kind: RecordKind) -> RecordData {
        world
            .query_record_data()
            .find_map(|(_, record)| {
                (record.home_place == place && record.record_kind == kind).then_some(record.clone())
            })
            .expect("fixture should provision the requested record")
    }

    struct Fixture {
        world: World,
        place: EntityId,
        office: EntityId,
        holder: EntityId,
        candidate_a: EntityId,
        candidate_b: EntityId,
        faction: EntityId,
    }

    impl Fixture {
        #[allow(clippy::needless_pass_by_value)]
        fn new(law: worldwake_core::SuccessionLaw) -> Self {
            let mut world = World::new(build_prototype_world()).unwrap();
            let place = world.topology().place_ids().next().unwrap();
            let (office, faction, holder, candidate_a, candidate_b) = {
                let mut txn = new_txn(&mut world, 1);
                let office = txn.create_office("Ruler").unwrap();
                let faction = txn.create_faction("Ward").unwrap();
                let holder = txn.create_agent("Holder", ControlSource::Ai).unwrap();
                let candidate_a = txn.create_agent("Alice", ControlSource::Ai).unwrap();
                let candidate_b = txn.create_agent("Bob", ControlSource::Ai).unwrap();
                for entity in [holder, candidate_a, candidate_b] {
                    txn.set_ground_location(entity, place).unwrap();
                }
                txn.add_member(candidate_a, faction).unwrap();
                txn.add_member(candidate_b, faction).unwrap();
                txn.set_component_office_data(
                    office,
                    OfficeData {
                        title: "Ruler".to_string(),
                        seat: place,
                        jurisdiction: BTreeSet::from([place]),
                        succession_law: law.clone(),
                        eligibility_rules: vec![worldwake_core::EligibilityRule::FactionMember(
                            faction,
                        )],
                        succession_period_ticks: 3,
                        vacancy_since: None,
                    },
                )
                .unwrap();
                if matches!(law, worldwake_core::SuccessionLaw::Force) {
                    txn.set_component_office_force_profile(
                        office,
                        OfficeForceProfile {
                            uncontested_hold_ticks: NonZeroU32::new(3).unwrap(),
                            vacancy_claim_grace_ticks: NonZeroU32::new(1).unwrap(),
                            challenger_presence_grace_ticks: NonZeroU32::new(1).unwrap(),
                        },
                    )
                    .unwrap();
                    txn.set_component_office_force_state(
                        office,
                        OfficeForceState {
                            control_since: None,
                            challenged_since: None,
                            contested_since: None,
                            last_uncontested_tick: None,
                        },
                    )
                    .unwrap();
                }
                let _ = create_record(&mut txn, place, holder, RecordKind::OfficeRegister);
                let _ = create_record(&mut txn, place, holder, RecordKind::SupportLedger);
                txn.assign_office(office, holder).unwrap();
                txn.set_component_utility_profile(holder, UtilityProfile::default())
                    .unwrap();
                txn.set_component_utility_profile(candidate_a, UtilityProfile::default())
                    .unwrap();
                txn.set_component_utility_profile(candidate_b, UtilityProfile::default())
                    .unwrap();
                let mut log = EventLog::new();
                let _ = txn.commit(&mut log);
                (office, faction, holder, candidate_a, candidate_b)
            };

            Self {
                world,
                place,
                office,
                holder,
                candidate_a,
                candidate_b,
                faction,
            }
        }

        fn kill_holder(&mut self, tick: u64) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.set_component_dead_at(
                self.holder,
                worldwake_core::DeadAt {
                    tick: Tick(tick),
                    cause: worldwake_core::DeathCause::CombatWounds,
                },
            )
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn declare_support(&mut self, supporter: EntityId, candidate: EntityId, tick: u64) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.declare_support(supporter, self.office, candidate)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn add_force_claim(&mut self, claimant: EntityId, tick: u64) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.add_force_claim(claimant, self.office).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn remove_force_claim(&mut self, claimant: EntityId, tick: u64) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.remove_force_claim(claimant, self.office).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn set_force_profile(
            &mut self,
            uncontested_hold_ticks: u32,
            vacancy_claim_grace_ticks: u32,
            challenger_presence_grace_ticks: u32,
            tick: u64,
        ) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.set_component_office_force_profile(
                self.office,
                OfficeForceProfile {
                    uncontested_hold_ticks: NonZeroU32::new(uncontested_hold_ticks).unwrap(),
                    vacancy_claim_grace_ticks: NonZeroU32::new(vacancy_claim_grace_ticks).unwrap(),
                    challenger_presence_grace_ticks: NonZeroU32::new(
                        challenger_presence_grace_ticks,
                    )
                    .unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn move_agent(&mut self, agent: EntityId, place: EntityId, tick: u64) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.set_ground_location(agent, place).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn kill_agent(&mut self, agent: EntityId, tick: u64) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.set_component_dead_at(
                agent,
                worldwake_core::DeadAt {
                    tick: Tick(tick),
                    cause: worldwake_core::DeathCause::CombatWounds,
                },
            )
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn add_patrolling_guard(&mut self, tick: u64) -> EntityId {
            let mut txn = new_txn(&mut self.world, tick);
            let guard = txn.create_agent("Guard", ControlSource::Ai).unwrap();
            txn.set_ground_location(guard, self.place).unwrap();
            txn.set_component_patrol_route(
                guard,
                PatrolRoute {
                    assigned_places: vec![self.place],
                    current_index: 0,
                },
            )
            .unwrap();
            txn.set_component_utility_profile(guard, UtilityProfile::default())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            guard
        }
    }

    #[test]
    fn office_helpers_reflect_current_authoritative_state() {
        let fx = Fixture::new(worldwake_core::SuccessionLaw::Support);

        assert_eq!(
            offices_with_jurisdiction(fx.place, &fx.world),
            vec![fx.office]
        );
        assert!(!office_is_vacant(fx.office, &fx.world));
        assert!(candidate_is_eligible(
            &fx.world,
            fx.world.get_component_office_data(fx.office).unwrap(),
            fx.candidate_a
        ));
        assert_eq!(
            eligible_agents_at(fx.office, fx.place, &fx.world),
            vec![fx.candidate_a, fx.candidate_b]
        );
    }

    #[test]
    fn offices_with_jurisdiction_matches_any_place_in_jurisdiction_set() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let extra_place = fx
            .world
            .topology()
            .place_ids()
            .find(|place| *place != fx.place)
            .unwrap();

        let mut txn = new_txn(&mut fx.world, 2);
        let mut office = txn.get_component_office_data(fx.office).cloned().unwrap();
        office.jurisdiction.insert(extra_place);
        txn.set_component_office_data(fx.office, office).unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);

        assert_eq!(
            offices_with_jurisdiction(extra_place, &fx.world),
            vec![fx.office]
        );
    }

    #[test]
    fn vacancy_activation_sets_vacancy_since_clears_relation_and_emits_visible_event() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 3);

        let office_data = fx.world.get_component_office_data(fx.office).unwrap();
        assert_eq!(office_data.vacancy_since, Some(Tick(3)));
        assert_eq!(fx.world.office_holder(fx.office), None);
        let register = record_at_place(&fx.world, fx.place, RecordKind::OfficeRegister);
        assert_eq!(register.entries.len(), 2);
        assert_eq!(
            register.entries[1].supersedes,
            Some(register.entries[0].entry_id)
        );
        let record = event_log
            .get(event_log.events_by_tag(EventTag::Political)[0])
            .unwrap();
        assert_eq!(record.place_id(), Some(fx.place));
        assert_eq!(record.visibility(), VisibilitySpec::SamePlace);
        assert!(record.target_ids().contains(&fx.office));
    }

    #[test]
    fn living_holder_clears_stale_vacancy_since() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        {
            let mut txn = new_txn(&mut fx.world, 2);
            let mut office = txn.get_component_office_data(fx.office).cloned().unwrap();
            office.vacancy_since = Some(Tick(1));
            txn.set_component_office_data(fx.office, office).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 3);

        assert_eq!(
            fx.world
                .get_component_office_data(fx.office)
                .unwrap()
                .vacancy_since,
            None
        );
        let record = event_log
            .get(event_log.events_by_tag(EventTag::Political)[0])
            .unwrap();
        assert_eq!(record.visibility(), VisibilitySpec::Hidden);
    }

    #[test]
    fn succession_trace_records_vacancy_activation_and_timer_wait() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        let mut trace = PoliticalTraceSink::new();

        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 3);
        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 4);

        let activation = trace.event_for_office_at(fx.office, Tick(3)).unwrap();
        assert_eq!(
            activation.trace.availability_phase,
            OfficeAvailabilityPhase::VacantClaimable
        );
        assert_eq!(
            activation.trace.outcome,
            OfficeSuccessionOutcome::VacancyActivated
        );

        let waiting = trace.event_for_office_at(fx.office, Tick(4)).unwrap();
        assert_eq!(
            waiting.trace.availability_phase,
            OfficeAvailabilityPhase::VacantWaitingForTimer
        );
        assert_eq!(
            waiting.trace.outcome,
            OfficeSuccessionOutcome::WaitingForTimer
        );
        assert_eq!(
            waiting.trace.vacancy_timer,
            Some(VacancyTimerTrace {
                start_tick: Tick(3),
                waited_ticks: 1,
                required_ticks: 3,
                remaining_ticks: 2,
            })
        );
        assert_eq!(
            waiting.trace.support_resolution,
            Some(SupportResolutionTrace {
                counted_support: Vec::new(),
            })
        );
    }

    #[test]
    fn succession_trace_records_pending_declarations_before_timer_elapses() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        let mut trace = PoliticalTraceSink::new();

        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 3);
        fx.declare_support(fx.candidate_a, fx.candidate_a, 4);
        event_log = EventLog::new();

        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 4);

        let waiting = trace.event_for_office_at(fx.office, Tick(4)).unwrap();
        assert_eq!(
            waiting.trace.availability_phase,
            OfficeAvailabilityPhase::VacantPendingResolution
        );
        assert_eq!(waiting.trace.support_declarations.len(), 1);
        assert!(waiting.trace.support_declarations[0].counted);
        assert_eq!(
            waiting.trace.outcome,
            OfficeSuccessionOutcome::WaitingForTimer
        );
        assert_eq!(
            waiting.trace.support_resolution,
            Some(SupportResolutionTrace {
                counted_support: vec![SupportCountTrace {
                    candidate: fx.candidate_a,
                    support: 1,
                }],
            })
        );
    }

    #[test]
    fn support_succession_installs_unique_top_supported_candidate_and_clears_declarations() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.declare_support(fx.candidate_a, fx.candidate_a, 4);
        fx.declare_support(fx.candidate_b, fx.candidate_a, 4);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 6);

        assert_eq!(fx.world.office_holder(fx.office), Some(fx.candidate_a));
        let register = record_at_place(&fx.world, fx.place, RecordKind::OfficeRegister);
        assert_eq!(register.entries.len(), 3);
        assert_eq!(
            register.entries[2].supersedes,
            Some(register.entries[1].entry_id)
        );
        assert_eq!(
            fx.world
                .get_component_office_data(fx.office)
                .unwrap()
                .vacancy_since,
            None
        );
        assert!(
            fx.world
                .support_declarations_for_office(fx.office)
                .is_empty()
        );
        let record = event_log
            .get(event_log.events_by_tag(EventTag::Political)[0])
            .unwrap();
        assert_eq!(record.place_id(), Some(fx.place));
        assert!(record.target_ids().contains(&fx.candidate_a));
    }

    #[test]
    fn support_succession_ignores_ineligible_declarations_and_resets_timer_on_no_valid_votes() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let outsider = {
            let mut txn = new_txn(&mut fx.world, 2);
            let outsider = txn.create_agent("Outsider", ControlSource::Ai).unwrap();
            let supporter = txn.create_agent("Supporter", ControlSource::Ai).unwrap();
            txn.set_ground_location(outsider, fx.place).unwrap();
            txn.set_ground_location(supporter, fx.place).unwrap();
            txn.set_component_utility_profile(outsider, UtilityProfile::default())
                .unwrap();
            txn.set_component_utility_profile(supporter, UtilityProfile::default())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            txn = new_txn(&mut fx.world, 3);
            txn.declare_support(supporter, fx.office, outsider).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            outsider
        };
        let _ = outsider;
        fx.kill_holder(4);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 5);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 8);

        assert_eq!(fx.world.office_holder(fx.office), None);
        assert_eq!(
            fx.world
                .get_component_office_data(fx.office)
                .unwrap()
                .vacancy_since,
            Some(Tick(8))
        );
        let record = event_log
            .get(event_log.events_by_tag(EventTag::Political)[0])
            .unwrap();
        assert_eq!(record.visibility(), VisibilitySpec::Hidden);
    }

    #[test]
    fn support_tie_resets_vacancy_clock_without_installing_anyone() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.declare_support(fx.candidate_a, fx.candidate_a, 4);
        fx.declare_support(fx.candidate_b, fx.candidate_b, 4);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 6);

        assert_eq!(fx.world.office_holder(fx.office), None);
        assert_eq!(
            fx.world
                .get_component_office_data(fx.office)
                .unwrap()
                .vacancy_since,
            Some(Tick(6))
        );
    }

    #[test]
    fn support_succession_trace_records_install_with_resolution_snapshot() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.declare_support(fx.candidate_a, fx.candidate_a, 4);
        fx.declare_support(fx.candidate_b, fx.candidate_a, 4);
        let mut trace = PoliticalTraceSink::new();
        event_log = EventLog::new();

        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 6);

        let event = trace.event_for_office_at(fx.office, Tick(6)).unwrap();
        assert_eq!(
            event.trace.availability_phase,
            OfficeAvailabilityPhase::ClosedOccupied
        );
        assert_eq!(
            event.trace.outcome,
            OfficeSuccessionOutcome::SupportInstalled {
                holder: fx.candidate_a,
            }
        );
        assert_eq!(
            event.trace.vacancy_timer,
            Some(VacancyTimerTrace {
                start_tick: Tick(3),
                waited_ticks: 3,
                required_ticks: 3,
                remaining_ticks: 0,
            })
        );
        assert_eq!(
            event.trace.support_resolution,
            Some(SupportResolutionTrace {
                counted_support: vec![SupportCountTrace {
                    candidate: fx.candidate_a,
                    support: 2,
                }],
            })
        );
        assert_eq!(event.trace.support_declarations.len(), 2);
        assert!(
            event
                .trace
                .support_declarations
                .iter()
                .all(|declaration| declaration.counted)
        );
    }

    #[test]
    fn support_succession_trace_records_tie_reset_with_resolution_snapshot() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.declare_support(fx.candidate_a, fx.candidate_a, 4);
        fx.declare_support(fx.candidate_b, fx.candidate_b, 4);
        let mut trace = PoliticalTraceSink::new();
        event_log = EventLog::new();

        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 6);

        let event = trace.event_for_office_at(fx.office, Tick(6)).unwrap();
        assert_eq!(
            event.trace.availability_phase,
            OfficeAvailabilityPhase::VacantReopenedAfterReset
        );
        assert_eq!(
            event.trace.outcome,
            OfficeSuccessionOutcome::SupportResetTie {
                tied_candidates: vec![fx.candidate_a, fx.candidate_b],
            }
        );
        assert_eq!(
            event.trace.vacancy_timer,
            Some(VacancyTimerTrace {
                start_tick: Tick(3),
                waited_ticks: 3,
                required_ticks: 3,
                remaining_ticks: 0,
            })
        );
        assert_eq!(
            event.trace.support_resolution,
            Some(SupportResolutionTrace {
                counted_support: vec![
                    SupportCountTrace {
                        candidate: fx.candidate_a,
                        support: 1,
                    },
                    SupportCountTrace {
                        candidate: fx.candidate_b,
                        support: 1,
                    },
                ],
            })
        );
        assert_eq!(event.trace.support_declarations.len(), 2);
        assert!(
            event
                .trace
                .support_declarations
                .iter()
                .all(|declaration| declaration.candidate_eligible && declaration.counted)
        );
    }

    #[test]
    fn support_succession_trace_records_no_eligible_reset_with_resolution_snapshot() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let outsider = {
            let mut txn = new_txn(&mut fx.world, 2);
            let outsider = txn.create_agent("Outsider", ControlSource::Ai).unwrap();
            let supporter = txn.create_agent("Supporter", ControlSource::Ai).unwrap();
            txn.set_ground_location(outsider, fx.place).unwrap();
            txn.set_ground_location(supporter, fx.place).unwrap();
            txn.set_component_utility_profile(outsider, UtilityProfile::default())
                .unwrap();
            txn.set_component_utility_profile(supporter, UtilityProfile::default())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            txn = new_txn(&mut fx.world, 3);
            txn.declare_support(supporter, fx.office, outsider).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            outsider
        };
        let _ = outsider;
        fx.kill_holder(4);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 5);
        let mut trace = PoliticalTraceSink::new();
        event_log = EventLog::new();

        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 8);

        let event = trace.event_for_office_at(fx.office, Tick(8)).unwrap();
        assert_eq!(
            event.trace.availability_phase,
            OfficeAvailabilityPhase::VacantReopenedAfterReset
        );
        assert_eq!(
            event.trace.outcome,
            OfficeSuccessionOutcome::SupportResetNoEligibleDeclarations
        );
        assert_eq!(
            event.trace.vacancy_timer,
            Some(VacancyTimerTrace {
                start_tick: Tick(5),
                waited_ticks: 3,
                required_ticks: 3,
                remaining_ticks: 0,
            })
        );
        assert_eq!(
            event.trace.support_resolution,
            Some(SupportResolutionTrace {
                counted_support: Vec::new(),
            })
        );
        assert_eq!(event.trace.support_declarations.len(), 1);
        assert!(!event.trace.support_declarations[0].candidate_eligible);
        assert!(!event.trace.support_declarations[0].counted);
    }

    #[test]
    fn force_control_establishes_controller_before_installation() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.add_force_claim(fx.candidate_a, 4);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 4);

        assert_eq!(fx.world.office_holder(fx.office), None);
        assert_eq!(fx.world.office_controller(fx.office), Some(fx.candidate_a));
        assert_eq!(
            fx.world.get_component_office_force_state(fx.office),
            Some(&OfficeForceState {
                control_since: Some(Tick(4)),
                challenged_since: None,
                contested_since: None,
                last_uncontested_tick: Some(Tick(4)),
            })
        );
    }

    #[test]
    fn force_control_vacancy_claim_grace_delays_controller_establishment() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        fx.set_force_profile(3, 2, 1, 2);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        let mut trace = PoliticalTraceSink::new();
        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 3);
        fx.add_force_claim(fx.candidate_a, 4);
        event_log = EventLog::new();

        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 4);

        assert_eq!(fx.world.office_holder(fx.office), None);
        assert_eq!(fx.world.office_controller(fx.office), None);
        assert_eq!(
            fx.world.get_component_office_force_state(fx.office),
            Some(&OfficeForceState {
                control_since: None,
                challenged_since: None,
                contested_since: None,
                last_uncontested_tick: None,
            })
        );
        let pending = trace.event_for_office_at(fx.office, Tick(4)).unwrap();
        assert_eq!(
            pending.trace.outcome,
            OfficeSuccessionOutcome::ForceVacancyClaimGracePending {
                claimant: fx.candidate_a,
                waited_ticks: 1,
                required_ticks: 2,
            }
        );
    }

    #[test]
    fn force_control_contest_clears_controller_and_sets_contested_state() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.add_force_claim(fx.candidate_a, 4);
        event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 4);
        fx.add_force_claim(fx.candidate_b, 5);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 5);

        assert_eq!(fx.world.office_holder(fx.office), None);
        assert_eq!(fx.world.office_controller(fx.office), None);
        assert_eq!(
            fx.world.get_component_office_force_state(fx.office),
            Some(&OfficeForceState {
                control_since: None,
                challenged_since: None,
                contested_since: Some(Tick(5)),
                last_uncontested_tick: None,
            })
        );
        let register = record_at_place(&fx.world, fx.place, RecordKind::OfficeRegister);
        assert_eq!(
            register.entries.last().unwrap().claim,
            worldwake_core::InstitutionalClaim::ForceControl {
                office: fx.office,
                controller: None,
                contested: true,
                effective_tick: Tick(5),
            }
        );
    }

    #[test]
    fn force_control_establishment_writes_force_control_record_entry() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.add_force_claim(fx.candidate_a, 4);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 4);

        let register = record_at_place(&fx.world, fx.place, RecordKind::OfficeRegister);
        assert_eq!(
            register.entries.last().unwrap().claim,
            worldwake_core::InstitutionalClaim::ForceControl {
                office: fx.office,
                controller: Some(fx.candidate_a),
                contested: false,
                effective_tick: Tick(4),
            }
        );
    }

    #[test]
    fn living_holder_trace_records_closed_occupied_phase() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        {
            let mut txn = new_txn(&mut fx.world, 2);
            let mut office = txn.get_component_office_data(fx.office).cloned().unwrap();
            office.vacancy_since = Some(Tick(1));
            txn.set_component_office_data(fx.office, office).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut event_log = EventLog::new();
        let mut trace = PoliticalTraceSink::new();

        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 3);

        let event = trace.event_for_office_at(fx.office, Tick(3)).unwrap();
        assert_eq!(
            event.trace.availability_phase,
            OfficeAvailabilityPhase::ClosedOccupied
        );
        assert_eq!(
            event.trace.outcome,
            OfficeSuccessionOutcome::OccupiedNoAction {
                holder: fx.holder,
                cleared_stale_vacancy: true,
            }
        );
    }

    #[test]
    fn force_control_departure_or_death_clears_controller_and_prunes_dead_claims() {
        let mut departure_fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        departure_fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut departure_fx.world, &mut event_log, 3);
        departure_fx.add_force_claim(departure_fx.candidate_a, 4);
        event_log = EventLog::new();
        run_succession(&mut departure_fx.world, &mut event_log, 4);
        let other_place = departure_fx
            .world
            .topology()
            .place_ids()
            .find(|place| *place != departure_fx.place)
            .unwrap();
        departure_fx.move_agent(departure_fx.candidate_a, other_place, 5);
        event_log = EventLog::new();

        run_succession(&mut departure_fx.world, &mut event_log, 5);

        assert_eq!(
            departure_fx.world.office_controller(departure_fx.office),
            None
        );
        assert_eq!(
            departure_fx
                .world
                .get_component_office_force_state(departure_fx.office),
            Some(&OfficeForceState {
                control_since: None,
                challenged_since: None,
                contested_since: None,
                last_uncontested_tick: None,
            })
        );

        let mut death_fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        death_fx.kill_holder(2);
        event_log = EventLog::new();
        run_succession(&mut death_fx.world, &mut event_log, 3);
        death_fx.add_force_claim(death_fx.candidate_b, 4);
        death_fx.kill_agent(death_fx.candidate_b, 5);
        event_log = EventLog::new();

        run_succession(&mut death_fx.world, &mut event_log, 5);

        assert!(
            death_fx
                .world
                .force_claimants_for_office_including_dead(death_fx.office)
                .is_empty()
        );
    }

    #[test]
    fn force_control_installation_requires_uncontested_hold_and_clears_claims() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.add_force_claim(fx.candidate_a, 4);
        for tick in [4_u64, 5, 6] {
            event_log = EventLog::new();
            run_succession(&mut fx.world, &mut event_log, tick);
        }

        assert_eq!(fx.world.office_holder(fx.office), Some(fx.candidate_a));
        assert_eq!(fx.world.office_controller(fx.office), None);
        assert_eq!(
            fx.world.get_component_office_force_state(fx.office),
            Some(&OfficeForceState {
                control_since: None,
                challenged_since: None,
                contested_since: None,
                last_uncontested_tick: None,
            })
        );
        assert!(
            fx.world
                .force_claimants_for_office_including_dead(fx.office)
                .is_empty()
        );
        let register = record_at_place(&fx.world, fx.place, RecordKind::OfficeRegister);
        assert_eq!(
            register.active_entries().len(),
            2,
            "office register should retain one active holder entry and one active force-control entry"
        );
        assert!(register.active_entries().iter().any(|entry| {
            entry.claim
                == worldwake_core::InstitutionalClaim::OfficeHolder {
                    office: fx.office,
                    holder: Some(fx.candidate_a),
                    effective_tick: Tick(6),
                }
        }));
        assert!(register.active_entries().iter().any(|entry| {
            entry.claim
                == worldwake_core::InstitutionalClaim::ForceControl {
                    office: fx.office,
                    controller: None,
                    contested: false,
                    effective_tick: Tick(6),
                }
        }));
        assert!(
            register.entries.len() >= 4,
            "force-control history should preserve controller transitions before installation"
        );
        let record = event_log
            .get(event_log.events_by_tag(EventTag::Political)[0])
            .unwrap();
        assert_eq!(record.visibility(), VisibilitySpec::SamePlace);
        assert!(record.target_ids().contains(&fx.office));
        assert!(record.target_ids().contains(&fx.candidate_a));
    }

    #[test]
    fn force_control_trace_reflects_controller_state_machine() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        let mut trace = PoliticalTraceSink::new();

        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 3);
        fx.add_force_claim(fx.candidate_a, 4);
        event_log = EventLog::new();
        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 4);
        event_log = EventLog::new();
        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 5);
        fx.add_force_claim(fx.candidate_b, 6);
        event_log = EventLog::new();
        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 6);

        let established = trace.event_for_office_at(fx.office, Tick(4)).unwrap();
        assert_eq!(
            established.trace.availability_phase,
            OfficeAvailabilityPhase::VacantPendingResolution
        );
        assert_eq!(
            established.trace.outcome,
            OfficeSuccessionOutcome::ForceControllerEstablished {
                controller: fx.candidate_a,
            }
        );

        let maintained = trace.event_for_office_at(fx.office, Tick(5)).unwrap();
        assert_eq!(
            maintained.trace.outcome,
            OfficeSuccessionOutcome::ForceControllerMaintained {
                controller: fx.candidate_a,
            }
        );

        let contested = trace.event_for_office_at(fx.office, Tick(6)).unwrap();
        assert_eq!(
            contested.trace.availability_phase,
            OfficeAvailabilityPhase::VacantPendingResolution
        );
        assert_eq!(
            contested.trace.outcome,
            OfficeSuccessionOutcome::ForceContested { claimant_count: 2 }
        );
        assert_eq!(
            contested.trace.force_candidates,
            vec![
                ForceCandidateTrace {
                    candidate: fx.holder,
                    eligible: false,
                },
                ForceCandidateTrace {
                    candidate: fx.candidate_a,
                    eligible: true,
                },
                ForceCandidateTrace {
                    candidate: fx.candidate_b,
                    eligible: true,
                },
            ]
        );
    }

    #[test]
    fn force_control_challenger_presence_grace_delays_contest() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        fx.set_force_profile(3, 1, 2, 2);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        let mut trace = PoliticalTraceSink::new();
        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 3);
        fx.add_force_claim(fx.candidate_a, 4);
        event_log = EventLog::new();
        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 4);
        fx.add_force_claim(fx.candidate_b, 5);
        event_log = EventLog::new();

        run_succession_with_trace(&mut fx.world, &mut event_log, &mut trace, 5);

        assert_eq!(fx.world.office_holder(fx.office), None);
        assert_eq!(fx.world.office_controller(fx.office), Some(fx.candidate_a));
        assert_eq!(
            fx.world.get_component_office_force_state(fx.office),
            Some(&OfficeForceState {
                control_since: Some(Tick(4)),
                challenged_since: Some(Tick(5)),
                contested_since: None,
                last_uncontested_tick: None,
            })
        );
        let pending = trace.event_for_office_at(fx.office, Tick(5)).unwrap();
        assert_eq!(
            pending.trace.outcome,
            OfficeSuccessionOutcome::ForceChallengerGracePending {
                controller: fx.candidate_a,
                challenger_count: 1,
                waited_ticks: 1,
                required_ticks: 2,
            }
        );
    }

    #[test]
    fn force_control_challenger_departure_before_grace_expiry_preserves_controller_continuity() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Force);
        fx.set_force_profile(4, 1, 2, 2);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        fx.add_force_claim(fx.candidate_a, 4);
        event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 4);
        fx.add_force_claim(fx.candidate_b, 5);
        event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 5);
        fx.remove_force_claim(fx.candidate_b, 6);
        event_log = EventLog::new();

        run_succession(&mut fx.world, &mut event_log, 6);

        assert_eq!(fx.world.office_holder(fx.office), None);
        assert_eq!(fx.world.office_controller(fx.office), Some(fx.candidate_a));
        assert_eq!(
            fx.world.get_component_office_force_state(fx.office),
            Some(&OfficeForceState {
                control_since: Some(Tick(4)),
                challenged_since: None,
                contested_since: None,
                last_uncontested_tick: Some(Tick(6)),
            })
        );
    }

    #[test]
    fn public_order_baseline_is_stable_when_place_has_no_vacancy_or_hostility() {
        let fx = Fixture::new(worldwake_core::SuccessionLaw::Support);

        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(750)
        );
    }

    #[test]
    fn public_order_subtracts_vacant_office_penalties() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let office_two = {
            let mut txn = new_txn(&mut fx.world, 2);
            let office = txn.create_office("Captain").unwrap();
            txn.set_component_office_data(
                office,
                OfficeData {
                    title: "Captain".to_string(),
                    seat: fx.place,
                    jurisdiction: BTreeSet::from([fx.place]),
                    succession_law: worldwake_core::SuccessionLaw::Support,
                    eligibility_rules: vec![worldwake_core::EligibilityRule::FactionMember(
                        fx.faction,
                    )],
                    succession_period_ticks: 3,
                    vacancy_since: Some(Tick(2)),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            office
        };
        let _ = office_two;
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);

        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(350)
        );
    }

    #[test]
    fn public_order_adds_guard_presence_bonus_for_patrolling_guard() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let _guard = fx.add_patrolling_guard(2);

        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(800)
        );
    }

    #[test]
    fn public_order_ignores_non_guard_agents_for_guard_bonus() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        {
            let mut txn = new_txn(&mut fx.world, 2);
            let bystander = txn.create_agent("Bystander", ControlSource::Ai).unwrap();
            txn.set_ground_location(bystander, fx.place).unwrap();
            txn.set_component_utility_profile(bystander, UtilityProfile::default())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(750)
        );
    }

    #[test]
    fn public_order_caps_guard_presence_bonus() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        for tick in 2..=6 {
            let _guard = fx.add_patrolling_guard(tick);
        }

        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(950)
        );
    }

    #[test]
    fn hostile_faction_pairs_count_one_way_hostility_once() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let faction_b = {
            let mut txn = new_txn(&mut fx.world, 2);
            let faction_b = txn.create_faction("Rivals").unwrap();
            let rival = txn.create_agent("Rival", ControlSource::Ai).unwrap();
            txn.set_ground_location(rival, fx.place).unwrap();
            txn.add_member(rival, faction_b).unwrap();
            txn.add_hostility(fx.faction, faction_b).unwrap();
            txn.set_component_utility_profile(rival, UtilityProfile::default())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            faction_b
        };

        assert_eq!(
            count_present_hostile_faction_pairs_at(fx.place, &fx.world),
            1
        );
        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(650)
        );

        {
            let mut txn = new_txn(&mut fx.world, 3);
            txn.add_hostility(faction_b, fx.faction).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        assert_eq!(
            count_present_hostile_faction_pairs_at(fx.place, &fx.world),
            1
        );
        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(650)
        );
    }

    #[test]
    fn hostile_pair_count_ignores_duplicate_members_from_same_faction() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        {
            let mut txn = new_txn(&mut fx.world, 2);
            let faction_b = txn.create_faction("Rivals").unwrap();
            let rival_a = txn.create_agent("Rival A", ControlSource::Ai).unwrap();
            let rival_b = txn.create_agent("Rival B", ControlSource::Ai).unwrap();
            for rival in [rival_a, rival_b] {
                txn.set_ground_location(rival, fx.place).unwrap();
                txn.add_member(rival, faction_b).unwrap();
                txn.set_component_utility_profile(rival, UtilityProfile::default())
                    .unwrap();
            }
            txn.add_hostility(fx.faction, faction_b).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        assert_eq!(
            count_present_hostile_faction_pairs_at(fx.place, &fx.world),
            1
        );
    }

    #[test]
    fn public_order_combines_vacancy_and_hostility_and_saturates_at_zero() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let extra_places = fx
            .world
            .topology()
            .place_ids()
            .filter(|place| *place != fx.place)
            .take(3)
            .collect::<Vec<_>>();
        let extra_places_len = extra_places.len();
        assert_eq!(extra_places_len, 3);
        {
            let mut txn = new_txn(&mut fx.world, 2);
            for index in 0..4 {
                let office = txn.create_office(&format!("Vacant {index}")).unwrap();
                txn.set_component_office_data(
                    office,
                    OfficeData {
                        title: format!("Vacant {index}"),
                        seat: fx.place,
                        jurisdiction: BTreeSet::from([fx.place]),
                        succession_law: worldwake_core::SuccessionLaw::Support,
                        eligibility_rules: vec![worldwake_core::EligibilityRule::FactionMember(
                            fx.faction,
                        )],
                        succession_period_ticks: 3,
                        vacancy_since: Some(Tick(2)),
                    },
                )
                .unwrap();
            }

            let faction_b = txn.create_faction("F2").unwrap();
            let faction_c = txn.create_faction("F3").unwrap();
            let faction_d = txn.create_faction("F4").unwrap();
            for (name, faction) in [("B", faction_b), ("C", faction_c), ("D", faction_d)] {
                let agent = txn.create_agent(name, ControlSource::Ai).unwrap();
                txn.set_ground_location(agent, fx.place).unwrap();
                txn.add_member(agent, faction).unwrap();
                txn.set_component_utility_profile(agent, UtilityProfile::default())
                    .unwrap();
            }
            txn.add_hostility(fx.faction, faction_b).unwrap();
            txn.add_hostility(fx.faction, faction_c).unwrap();
            txn.add_hostility(fx.faction, faction_d).unwrap();
            txn.add_hostility(faction_b, faction_c).unwrap();
            txn.add_hostility(faction_b, faction_d).unwrap();
            txn.add_hostility(faction_c, faction_d).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);

        assert_eq!(
            count_present_hostile_faction_pairs_at(fx.place, &fx.world),
            6
        );
        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(0)
        );
    }

    #[test]
    fn public_order_guard_bonus_composes_with_existing_penalties() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        let _guard = fx.add_patrolling_guard(2);
        fx.kill_holder(2);
        let mut event_log = EventLog::new();
        run_succession(&mut fx.world, &mut event_log, 3);
        {
            let mut txn = new_txn(&mut fx.world, 3);
            let faction_b = txn.create_faction("Rivals").unwrap();
            let rival = txn.create_agent("Rival", ControlSource::Ai).unwrap();
            txn.set_ground_location(rival, fx.place).unwrap();
            txn.add_member(rival, faction_b).unwrap();
            txn.add_hostility(fx.faction, faction_b).unwrap();
            txn.set_component_utility_profile(rival, UtilityProfile::default())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        assert_eq!(
            public_order(fx.place, &fx.world),
            Permille::new_unchecked(500)
        );
    }

    #[test]
    fn dispatch_table_runs_real_politics_system() {
        let mut fx = Fixture::new(worldwake_core::SuccessionLaw::Support);
        fx.kill_holder(2);
        let systems = dispatch_table();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([3; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        systems.get(SystemId::Politics)(SystemExecutionContext {
            world: &mut fx.world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Politics,
        })
        .unwrap();

        assert_eq!(
            fx.world
                .get_component_office_data(fx.office)
                .unwrap()
                .vacancy_since,
            Some(Tick(3))
        );
    }
}
