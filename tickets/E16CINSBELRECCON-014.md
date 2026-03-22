# E16CINSBELRECCON-014: Live Helper Seam Removal + Golden Test Migration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — remove live institutional helper paths, update all golden tests
**Deps**: E16CINSBELRECCON-009, E16CINSBELRECCON-010, E16CINSBELRECCON-011, E16CINSBELRECCON-012, E16CINSBELRECCON-013

## Problem

The current architecture has a narrow runtime seam where AI reads live office/faction/support truth directly through helper methods. Spec §11 (Migration Requirement) mandates: (1) remove the current planner-only live institutional helper path for public office/faction/support facts, (2) update E16 political AI to read institutional beliefs, (3) do not preserve both architectures in parallel (Principle 26). This is the capstone ticket — it cuts the old path and proves the new one works end-to-end.

## Assumption Reassessment (2026-03-22)

1. `per_agent_belief_view.rs` in worldwake-sim contains the `PerAgentBeliefView` that backs AI queries. It currently reads live world state for some institutional queries (office holder, faction membership, support declarations). These reads must be replaced with institutional belief reads.
2. `PlanningSnapshot` currently captures `actor_support_declarations` and `office_support_declarations` from live world state (planning_snapshot.rs:201-204). These fields must be replaced with belief-derived fields from ticket -010.
3. Golden tests in `worldwake-ai/tests/` exercise political behavior. They must be updated to set up institutional beliefs (via record consultation or witness projection) rather than relying on omniscient truth.
4. The old live helper path must be deleted, not deprecated or feature-flagged (Principle 26).
5. N/A — no ordering.
6. Removing the live helper seam. The seam provides omniscient institutional truth to AI. The substrate replacing it is the institutional belief system (tickets -001 through -013). This ticket does not remove the substrate — it removes the old path and verifies the new one is sufficient.
7. N/A.
8. Closure boundary: political AI must read institutional beliefs for office holder, support declarations, faction membership. The exact symbols: `PerAgentBeliefView::office_holder()`, `PerAgentBeliefView::support_declarations_for_office()`, and any direct world queries in the AI planning path.
9. N/A.
10. Golden test scenarios must set up belief state (records + consultation or witness events) so agents have institutional knowledge through legitimate paths. Scenarios that previously relied on omniscient truth must be updated.
11. Additional live-code clarification after ticket `-005`: `consult_record` is now a real registered action and the AI semantics table already classifies it minimally for registry integrity, but autonomous consult-goal emission/search remains owned by tickets `-011` and `-012`.
12. Additional live-code clarification after ticket `-006`: witness acquisition now exists in `crates/worldwake-systems/src/perception.rs`, so the remaining architecture gap is no longer acquisition for visible political events; it is the AI-side cutover off the live helper seam.
13. Additional live-code clarification after ticket `-008`: Tell now relays institutional claims for entity subjects through conversation memory into `institutional_beliefs`, so social institutional propagation is no longer the blocker. The remaining gap is removal of the AI-side live helper seam and migration of remaining political callers/goldens onto the belief-backed path.
14. Mismatch + correction: this ticket should not invent autonomous consult behavior during cutover. It should assume `-011` and `-012` are complete first, then remove the live helper seam and migrate goldens onto the new belief/consult substrate.
15. Additional architectural note: the seam is not only the `PerAgentBeliefView` implementation. `crates/worldwake-sim/src/belief_view.rs` still exposes trait methods named `office_holder`, `factions_of`, `support_declaration`, and `support_declarations_for_office`. After all AI callers migrate, this ticket should either delete those public-institutional trait methods or narrow them so the old omniscient query shape cannot be reintroduced through another runtime belief-view implementation.
16. Additional reassessment after ticket `-012`: office-register interpretation is now duplicated between `crates/worldwake-ai/src/candidate_generation.rs` (`consulted_office_holder_read_for_record`) and `crates/worldwake-ai/src/goal_model.rs` (`consulted_office_holder_read_for_record`). That duplication is part of the same live-seam cleanup problem, not a separate feature: candidate emission and planner prerequisite checks must derive office-holder reads from record data through one shared helper so their semantics cannot drift.

## Architecture Check

1. Removing the live helper path is the only correct approach per Principle 26 (no backward compatibility layers). Keeping both paths would create a maintenance burden and eventual divergence.
2. This is explicitly anti-backward-compatibility — the old path is deleted, not preserved.
3. Centralizing office-register interpretation is cleaner than letting candidate generation and planner prerequisites each re-derive vacancy semantics. Foundations Principles 12, 21, 24, and 25 require one belief-backed interpretation of institutional records, with both layers consuming the same derived result rather than maintaining parallel logic.

## Verification Layers

1. No AI module reads live office_holder / member_of / support_declarations → grep verification
2. `PerAgentBeliefView` institutional methods backed by belief store → unit tests
3. Golden tests pass with institutional belief setup → `cargo test -p worldwake-ai`
4. Political AI behaves correctly with belief-based institutional knowledge → golden test scenarios
5. Candidate generation and planner prerequisite gating interpret the same office register the same way → focused unit tests on the shared helper plus caller coverage

## What to Change

### 1. Replace live helpers in `per_agent_belief_view.rs`

Replace implementations of institutional query methods to read from `AgentBeliefStore.institutional_beliefs` via derivation helpers (ticket -009) instead of reading from `self.world.office_holder()` etc.

Methods to migrate:
- `office_holder(office)` → `believed_office_holder(office)` from belief store
- `factions_of(member)` → belief-derived membership read / aggregation from belief store
- `support_declarations_for_office(office)` → `believed_support_declarations_for_office(office)` from belief store
- `support_declaration(supporter, office)` → belief-derived support read from belief store
- Any other institutional queries that currently read live truth

Current live seam to remove, verified during reassessment:
- `crates/worldwake-sim/src/per_agent_belief_view.rs:782-789`
- `crates/worldwake-sim/src/per_agent_belief_view.rs:792-797`
- `crates/worldwake-sim/src/per_agent_belief_view.rs:811-822`

### 2. Remove old snapshot fields in `planning_snapshot.rs`

Replace `actor_support_declarations` and `office_support_declarations` with belief-derived institutional belief data (from ticket -010's `actor_institutional_beliefs` field). Remove the old fields entirely.

### 3. Update all callers of removed fields

Grep for `actor_support_declarations` and `office_support_declarations` across the AI crate and update to use the new institutional belief query methods on `PlanningState`.

### 4. Centralize office-register interpretation

Move the "consulted office register -> `InstitutionalBeliefRead<Option<EntityId>>`" logic behind one shared helper consumed by both candidate generation and planner/prerequisite evaluation. The helper must operate on concrete record data / planning-state record access, not on live world truth, and it must remain the only place where `OfficeRegister` entries are folded into office-holder belief certainty for political AI.

This ticket should delete the duplicated `consulted_office_holder_read_for_record` implementations rather than leaving equivalent copies behind.

### 5. Update golden tests

For each golden test that exercises political behavior:
- Add record entities to the test world setup
- Ensure agents gain institutional beliefs through legitimate paths (consultation, witness, tell)
- Remove any test setup that relied on omniscient institutional truth reaching AI
- Verify the test still validates the intended political behavior

Note:
- Any golden that expects autonomous "Unknown institutional belief -> seek record -> consult -> act" behavior depends on tickets `-011` and `-012`. If those tickets are not landed yet, do not paper over the gap here with live truth shortcuts or bespoke test-only seeding.
- Any golden that uses social institutional propagation should now prefer the landed `-008` Tell path over bespoke institutional-belief injection when ordinary Tell is sufficient for the scenario.

### 6. Verification grep

After all changes, run a workspace-wide grep for direct live institutional reads in AI modules. There must be zero hits for:
- `world.office_holder` / `world.offices_held` in AI/belief-view context
- `world.member_of` / `world.members_of` in AI/belief-view context
- `world.support_declarations` in AI/belief-view context

Also verify the legacy planning snapshot fields are gone:
- `actor_support_declarations`
- `office_support_declarations`

Also verify the legacy trait seam is gone or intentionally narrowed:
- `fn office_holder(` in `crates/worldwake-sim/src/belief_view.rs`
- `fn factions_of(` in `crates/worldwake-sim/src/belief_view.rs`
- `fn support_declaration(` in `crates/worldwake-sim/src/belief_view.rs`
- `fn support_declarations_for_office(` in `crates/worldwake-sim/src/belief_view.rs`

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — replace live reads with belief store reads)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — remove old fields, use institutional beliefs)
- `crates/worldwake-ai/src/planning_state.rs` (modify — remove old support_declaration_overrides if replaced)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — remove any remaining live truth reads)
- `crates/worldwake-ai/src/goal_model.rs` (modify — consume shared office-register interpretation helper instead of local duplicate)
- `crates/worldwake-ai/src/search.rs` (modify — remove any remaining live truth reads)
- `crates/worldwake-ai/src/` shared institutional helper module or existing appropriate AI module (modify/new — single office-register interpretation path)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify — update setup with record/belief infrastructure)
- Other golden test files that exercise political behavior (modify)

## Out of Scope

- Authoritative office/faction/support relations (remain unchanged — they are world truth, not AI input)
- Non-institutional belief reads (entity beliefs, wound beliefs, etc. — unchanged)
- E16b, E17, E19 changes (they build on E16c's architecture when they are implemented)
- New golden test scenarios beyond what's needed for migration (can be added later)

## Acceptance Criteria

### Tests That Must Pass

1. All existing golden tests pass after migration
2. New golden test: agent consults office register, gains belief, then claims office
3. New golden test: agent witnesses support declaration, gains belief, acts on it
4. New golden test: agent with conflicted institutional belief suppresses political action
5. New golden test: agent with Unknown belief seeks ConsultRecord before political action
6. Workspace grep for live institutional reads in AI modules returns zero hits
7. `PerAgentBeliefView` institutional methods return belief-derived results, not live truth, for office holder, faction membership, and support declarations
8. The old trait-level institutional helper seam in `crates/worldwake-sim/src/belief_view.rs` is removed or narrowed so omniscient public-institutional reads cannot be reintroduced accidentally
9. Candidate generation and planner prerequisite evaluation both call the same office-register interpretation helper; no duplicate `OfficeRegister` folding logic remains in `candidate_generation.rs` and `goal_model.rs`
10. Existing suite: `cargo test --workspace`

### Invariants

1. No AI module depends on live public institutional helper queries (spec AC §8)
2. Both architectures are NOT preserved in parallel (Principle 26)
3. Authoritative office/faction/support state remains unchanged (only the AI reading path changed)
4. All information reaching AI is traceable to witness, report, or record consultation
5. Office-register interpretation is defined once and reused across candidate generation and planner gating, so the same institutional record cannot produce divergent vacancy semantics in different AI layers

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_offices.rs` — updated existing + new institutional belief scenarios
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` — belief-backed institutional query unit tests
3. `crates/worldwake-ai/src/goal_model.rs` or shared helper tests — office-register interpretation shared by candidate generation and planner gating
4. Verification script: grep for live institutional reads in AI crate

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace && cargo test --workspace`
