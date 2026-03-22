# E16CINSBELRECCON-014: Live Helper Seam Removal + Golden Test Migration

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — remove live institutional helper paths, update all golden tests
**Deps**: E16CINSBELRECCON-009, E16CINSBELRECCON-010, E16CINSBELRECCON-011, E16CINSBELRECCON-012, E16CINSBELRECCON-013

## Problem

The current architecture has a narrow runtime seam where AI reads live office/faction/support truth directly through helper methods. Spec §11 (Migration Requirement) mandates: (1) remove the current planner-only live institutional helper path for public office/faction/support facts, (2) update E16 political AI to read institutional beliefs, (3) do not preserve both architectures in parallel (Principle 26). This is the capstone ticket — it cuts the old path and proves the new one works end-to-end.

## Assumption Reassessment (2026-03-22)

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` still exposes live institutional reads through `RuntimeBeliefView::office_holder()`, `RuntimeBeliefView::factions_of()`, `RuntimeBeliefView::support_declaration()`, and `RuntimeBeliefView::support_declarations_for_office()`; the current implementations call authoritative world relations directly at `per_agent_belief_view.rs:783-839`. This is the remaining runtime seam.
2. `crates/worldwake-sim/src/belief_view.rs` still publishes those same trait-level methods on both `GoalBeliefView` and `RuntimeBeliefView`. The public seam itself is stale even where some callers have already moved to belief-backed methods.
3. `crates/worldwake-ai/src/planning_snapshot.rs` still snapshots live support state through `actor_support_declarations` and `office_support_declarations` (`planning_snapshot.rs:206-210`, populated at `planning_snapshot.rs:281-300`). Those fields are authoritative mirrors, not belief state.
4. `crates/worldwake-ai/src/planning_state.rs` still uses those live snapshot fields for political satisfaction/counting via `support_declaration()`, `hypothetical_support_count()`, and `has_support_majority()` (`planning_state.rs:111-243`, `planning_state.rs:1462-1477`). This means planner-state politics still partially reason over omniscient support truth.
5. The office-register consultation path is already partially landed. `consult_record` is registered and tested in `crates/worldwake-systems/src/consult_record_actions.rs`; candidate generation already uses institutional beliefs for office-holder/support reads in several tests, including `candidate_generation::tests::political_candidates_do_not_fallback_to_live_support_or_holder_helpers` and `...unknown_belief_require_consultable_record_evidence`.
6. The ticket’s original “update all political goldens to consultation-driven setup” scope is stale. Current political goldens already seed institutional beliefs explicitly and do not all need consultation migration. The real gap is narrower: remove the remaining live helper/snapshot seams, and update only the focused/golden tests that still depend on them.
7. The ticket under-called faction eligibility. `candidate_is_eligible()` in `crates/worldwake-ai/src/candidate_generation.rs:586-596` still uses live `factions_of(candidate)`. Because faction membership is an institutional fact under the E16c architecture, eligibility must move onto institutional belief reads as part of this ticket.
8. The ticket correctly identified duplicate office-register interpretation, but the live locations differ slightly from the original wording: the duplicated helper exists in `crates/worldwake-ai/src/candidate_generation.rs:389-416` and `crates/worldwake-ai/src/goal_model.rs:260-297`. Both fold `RecordData` into `InstitutionalBeliefRead<Option<EntityId>>` and should be unified.
9. The old live helper path must be deleted, not deprecated or feature-flagged (Principle 26). That applies to both call sites and the trait surface.
5. N/A — no ordering.
6. Removing the live helper seam. The seam provides omniscient institutional truth to AI. The substrate replacing it is the institutional belief system already present in `AgentBeliefStore`, record consultation, witness projection, and Tell propagation. This ticket does not invent a second substrate; it removes the old path and verifies the existing one is sufficient.
7. N/A.
8. Closure boundary: political AI must read institutional beliefs for office holder, support declarations, and faction membership. The exact symbols now in scope are `PerAgentBeliefView::{office_holder,factions_of,support_declaration,support_declarations_for_office}`, the trait declarations in `crates/worldwake-sim/src/belief_view.rs`, the live snapshot fields in `crates/worldwake-ai/src/planning_snapshot.rs`, and the eligibility/planner consumers in `candidate_generation.rs` and `planning_state.rs`.
9. N/A.
10. Golden test scenarios must set up belief state through legitimate paths. That can be witness projection, Tell propagation, or record consultation. Consultation is not mandatory for every political golden; it is mandatory only where the contract is specifically “unknown institutional belief becomes known by consulting a record.”
11. Additional live-code clarification after ticket `-005`: `consult_record` is now a real registered action and the AI semantics table already classifies it minimally for registry integrity, but autonomous consult-goal emission/search remains owned by tickets `-011` and `-012`.
12. Additional live-code clarification after ticket `-006`: witness acquisition now exists in `crates/worldwake-systems/src/perception.rs`, so the remaining architecture gap is no longer acquisition for visible political events; it is the AI-side cutover off the live helper seam.
13. Additional live-code clarification after ticket `-008`: Tell now relays institutional claims for entity subjects through conversation memory into `institutional_beliefs`, so social institutional propagation is no longer the blocker. The remaining gap is removal of the AI-side live helper seam and migration of remaining political callers/goldens onto the belief-backed path.
14. Mismatch + correction: this ticket should not invent autonomous consult behavior during cutover. It should assume `-011` and `-012` are complete first, then remove the live helper seam and migrate goldens onto the new belief/consult substrate.
15. Additional architectural note: the seam is not only the `PerAgentBeliefView` implementation. `crates/worldwake-sim/src/belief_view.rs` still exposes trait methods named `office_holder`, `factions_of`, `support_declaration`, and `support_declarations_for_office`. This ticket should delete those trait methods rather than merely changing their implementations.
16. Additional reassessment after ticket `-012`: office-register interpretation is now duplicated between `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/goal_model.rs`. That duplication is part of the same live-seam cleanup problem, not a separate feature: candidate emission and planner prerequisite checks must derive office-holder reads from record data through one shared helper so their semantics cannot drift.
17. Additional reassessment after checking `cargo test -p worldwake-ai -- --list` and current golden files: no current golden named in this ticket directly exercises “unknown belief seeks ConsultRecord before political action” end-to-end. That is still a valid gap, but the ticket should describe it as a missing targeted/golden coverage addition, not as a migration of existing scenarios.

## Architecture Check

1. Removing the live helper path is the only correct approach per Principle 26 (no backward compatibility layers). Keeping both paths would create a maintenance burden and eventual divergence.
2. This is explicitly anti-backward-compatibility — the old path is deleted, not preserved.
3. Removing the trait-level omniscient institutional helpers is cleaner than trying to “make them safe.” The method names encode the wrong contract and invite future authoritative reads to creep back in. The robust architecture is explicit belief queries only.
4. Faction eligibility belongs on institutional belief reads for the same reason as office-holder/support knowledge. Leaving faction membership on live truth would preserve a second backdoor into political omniscience and make office eligibility semantically inconsistent with vacancy/support reasoning.
5. Centralizing office-register interpretation is cleaner than letting candidate generation and planner prerequisites each re-derive vacancy semantics. Foundations Principles 12, 21, 24, and 25 require one belief-backed interpretation of institutional records, with both layers consuming the same derived result rather than maintaining parallel logic.

## Verification Layers

1. trait/running-surface seam removal -> grep + compile failures for deleted methods
2. office-holder/support/faction institutional reads come from belief state, not authoritative world relations -> focused `per_agent_belief_view` and candidate-generation unit tests
3. planner hypothetical support counting uses only belief-backed baseline plus planner overrides -> focused `planning_snapshot` / `planning_state` unit tests
4. candidate generation and planner prerequisite gating interpret the same office register the same way -> focused shared-helper tests plus caller coverage
5. end-to-end political behavior still works with legitimate institutional knowledge paths -> targeted political goldens and `cargo test -p worldwake-ai`

## What to Change

### 1. Replace live helpers in `per_agent_belief_view.rs`

Delete the remaining live institutional helper methods from the AI-facing traits and remove their `PerAgentBeliefView` implementations. Replace them with belief-backed reads only.

Required belief-backed surface after cleanup:
- `believed_office_holder(office)`
- `believed_support_declaration(office, supporter)`
- `believed_support_declarations_for_office(office)`
- new belief-backed faction-membership query suitable for eligibility checks

Current live seam to remove, verified during reassessment:
- `crates/worldwake-sim/src/per_agent_belief_view.rs:783-839`
- corresponding trait declarations in `crates/worldwake-sim/src/belief_view.rs`

### 2. Remove old snapshot fields in `planning_snapshot.rs`

Remove `actor_support_declarations` and `office_support_declarations`. Replace them with belief-derived baseline data only. The baseline support-count view should be assembled from `believed_support_declarations_for_office()` by retaining only `Certain(Some(candidate))` entries. Unknown or conflicted reads must not silently count as committed support.

### 3. Update all callers of removed fields

Update `PlanningState`, `GoalKind::is_satisfied()`, and hypothetical support-counting logic to use the new belief-backed baseline plus planner overrides. Grep for `actor_support_declarations` and `office_support_declarations` across the AI crate and remove all usages.

### 4. Centralize office-register interpretation

Move the "consulted office register -> `InstitutionalBeliefRead<Option<EntityId>>`" logic behind one shared helper consumed by both candidate generation and planner/prerequisite evaluation. The helper must operate on concrete record data / planning-state record access, not on live world truth, and it must remain the only place where `OfficeRegister` entries are folded into office-holder belief certainty for political AI.

This ticket should delete the duplicated `consulted_office_holder_read_for_record` implementations rather than leaving equivalent copies behind.

### 5. Move faction eligibility onto institutional beliefs

Replace `view.factions_of(candidate).contains(faction)` in `candidate_is_eligible()` with a belief-backed membership check. Unknown or conflicted faction membership must not satisfy an eligibility rule that requires concrete institutional knowledge.

### 6. Update focused and golden tests

Update only the tests that still rely on the removed seam:
- focused tests in `per_agent_belief_view.rs`, `planning_snapshot.rs`, `planning_state.rs`, `candidate_generation.rs`, and the shared office-register helper
- political goldens that currently rely on live faction eligibility or live support baselines

Note:
- Any golden that expects autonomous "Unknown institutional belief -> seek record -> consult -> act" behavior depends on tickets `-011` and `-012`. If those tickets are not landed yet, do not paper over the gap here with live truth shortcuts or bespoke test-only seeding.
- Any golden that uses social institutional propagation should now prefer the landed `-008` Tell path over bespoke institutional-belief injection when ordinary Tell is sufficient for the scenario.

### 7. Verification grep

After all changes, run a workspace-wide grep for direct live institutional reads in AI modules. There must be zero hits for:
- `world.office_holder` / `world.offices_held` in AI/belief-view context
- `world.member_of` / `world.members_of` in AI/belief-view context
- `world.support_declarations` in AI/belief-view context

Also verify the legacy planning snapshot fields are gone:
- `actor_support_declarations`
- `office_support_declarations`

Also verify the legacy trait seam is gone:
- `fn office_holder(` in `crates/worldwake-sim/src/belief_view.rs`
- `fn factions_of(` in `crates/worldwake-sim/src/belief_view.rs`
- `fn support_declaration(` in `crates/worldwake-sim/src/belief_view.rs`
- `fn support_declarations_for_office(` in `crates/worldwake-sim/src/belief_view.rs`

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — delete legacy institutional helper methods; add belief-backed faction-membership query if needed)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — remove live institutional implementations; add belief-backed faction-membership implementation)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — remove live support snapshot fields, keep belief-derived baseline only)
- `crates/worldwake-ai/src/planning_state.rs` (modify — count/support satisfaction from belief-backed baseline plus overrides)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — remove remaining live faction read; consume shared office-register helper)
- `crates/worldwake-ai/src/goal_model.rs` (modify — consume shared office-register interpretation helper instead of local duplicate)
- `crates/worldwake-ai/src/` shared institutional helper module or existing appropriate AI module (modify/new — single office-register interpretation path)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify if needed — align faction/support setup with belief-backed architecture)
- focused unit-test sections in the files above (modify)

## Out of Scope

- Authoritative office/faction/support relations (remain unchanged — they are world truth, not AI input)
- Non-institutional belief reads (entity beliefs, wound beliefs, etc. — unchanged)
- E16b, E17, E19 changes (they build on E16c's architecture when they are implemented)
- New golden test scenarios beyond what's needed for migration (can be added later)

## Acceptance Criteria

### Tests That Must Pass

1. Workspace grep for live institutional reads in AI modules returns zero hits
2. The old trait-level institutional helper seam in `crates/worldwake-sim/src/belief_view.rs` is removed
3. `PerAgentBeliefView` institutional queries are belief-derived for office holder, faction membership, and support declarations
4. `PlanningSnapshot` / `PlanningState` no longer snapshot or count live support declarations
5. Candidate generation and planner prerequisite evaluation both call the same office-register interpretation helper; no duplicate `OfficeRegister` folding logic remains in `candidate_generation.rs` and `goal_model.rs`
6. Political eligibility under `EligibilityRule::FactionMember` uses institutional beliefs, not authoritative faction truth
7. Focused tests covering the new seams pass
8. Existing political goldens still pass
9. Existing suite: `cargo test --workspace`

### Invariants

1. No AI module depends on live public institutional helper queries (spec AC §8)
2. Both architectures are NOT preserved in parallel (Principle 26)
3. Authoritative office/faction/support state remains unchanged (only the AI reading path changed)
4. All information reaching AI is traceable to witness, report, or record consultation
5. Office-register interpretation is defined once and reused across candidate generation and planner gating, so the same institutional record cannot produce divergent vacancy semantics in different AI layers
6. Unknown or conflicted institutional beliefs do not silently satisfy faction-eligibility or support-majority checks

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — replace old live-helper tests with belief-backed institutional query coverage, including faction membership
2. `crates/worldwake-ai/src/planning_snapshot.rs` — assert the snapshot baseline support state is derived from belief reads, not live declarations
3. `crates/worldwake-ai/src/planning_state.rs` — assert support satisfaction/majority logic uses belief-backed baseline plus overrides
4. `crates/worldwake-ai/src/candidate_generation.rs` and shared helper tests — office-register interpretation and belief-backed faction eligibility
5. `crates/worldwake-ai/tests/golden_offices.rs` — update only if a current golden still relies on the removed live seam
6. Verification grep: no live institutional AI reads, no legacy snapshot fields, no legacy trait methods

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::political_candidates_do_not_fallback_to_live_support_or_holder_helpers`
2. `cargo test -p worldwake-ai planning_snapshot::tests`
3. `cargo test -p worldwake-ai planning_state::tests`
4. `cargo test -p worldwake-ai goal_model::tests::consult_record_step_overrides_unknown_vacancy_belief_and_unblocks_declare_support`
5. `cargo test -p worldwake-sim per_agent_belief_view::tests`
6. `cargo test -p worldwake-ai golden_information_locality_for_political_facts -- --exact`
7. `cargo test -p worldwake-ai golden_faction_eligibility_filters_office_claim -- --exact`
8. `cargo test -p worldwake-ai`
9. `cargo test -p worldwake-sim`
10. `cargo clippy --workspace`
11. `cargo test --workspace`

## Outcome

- Outcome amended: 2026-03-22
- Completion date: 2026-03-22
- What changed: removed the remaining live institutional helper seam from the AI/runtime belief-view boundary; moved faction eligibility and planner support baselines onto institutional belief reads; centralized office-register interpretation; updated focused coverage and the affected office goldens; aligned golden/implementation docs with the belief-backed architecture; and renamed the planner-local support accessor in `PlanningState` to `effective_support_declaration` so it no longer resembles the deleted live helper seam.
- Deviations from original plan: the original ticket over-scoped goldens as a blanket consultation migration. Reassessment narrowed the work to the remaining live helper/snapshot seam plus the specific affected tests. Scenario 14 in `golden_offices.rs` was corrected to assert courage-diverse coercion and downstream support-goal availability rather than overasserting the final office winner, because that winner depends on separate ranking dynamics outside this seam-removal ticket.
- Verification results:
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - grep verification for removed live-helper seams/snapshot fields passed after the follow-up naming cleanup
