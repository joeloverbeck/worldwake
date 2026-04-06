# S51ARTISS-002: Planner ops and goal dispatch declarations

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new PlannerOpKind variants, classify_action_def mapping, live posting goal dispatch, and planner-facing posting payload substrate
**Deps**: S51ARTISS-001

## Problem

The planner still cannot plan autonomous artifact issuance. `S51ARTISS-001` landed inert `GoalDispatchDeclaration` placeholders for `PostBounty` / `PostNotice`, but there are still no posting planner ops, no action classification, and no searchable terminal posting path. More importantly, the current `GoalKind::PostBounty { target, posting_place }` shape is too weak to synthesize a lawful `post_bounty` action: the action requires concrete reward, proof, reward-source, and claim-place data. This ticket therefore owns both the live posting planner surface and the missing planner-facing posting payload substrate that bounty posting needs.

## Assumption Reassessment (2026-04-05)

1. `S51ARTISS-001` already landed inert `GoalDispatchKey::PostBounty` / `PostNotice` entries and `DECL_POST_BOUNTY` / `DECL_POST_NOTICE` in `crates/worldwake-ai/src/goal_dispatch_decl.rs:289-300`, but both still use `relevant_ops: NO_OPS` and `NoOpinion` strategies. Correction applied: this ticket now converts an existing inert dispatch surface into a live planner surface rather than adding it from scratch.
2. `PlannerOpKind` at `crates/worldwake-ai/src/planner_ops.rs:13-45` still has no `PostBounty` / `PostNotice` variants, and `build_semantics_table_classifies_registered_planner_action_defs()` still treats `post_bounty` / `post_notice` as intentionally unclassified at `planner_ops.rs:1608`.
3. `post_bounty` and `post_notice` action defs already exist in `crates/worldwake-systems/src/artifact_actions.rs:82-149` with `TargetSpec::ActorPlace` and payload override validators.
4. `GoalKind::PostNotice { topic, posting_place }` can already synthesize a lawful posting payload if the planner owns a canonical posting-context surface for optional header fields such as issuing authority, expiration, and jurisdiction.
5. `GoalKind::PostBounty { target, posting_place }` is not sufficient for lawful planner synthesis. `post_bounty` requires the full `PostBountyActionPayload` contract at `crates/worldwake-sim/src/action_payload.rs:336-346`, including `proof_requirement`, `reward_commodity`, `reward_quantity`, `reward_source`, and `claim_place`. Correction applied: this ticket owns the missing planner-facing posting substrate needed to carry those concrete bounty terms.
6. Search semantics for posting should be modeled as leaf-only progress-barrier actions, not as hypothetical artifact creation in planning state. Existing leaf-only goal families such as `ShareBelief`, `InvestigateViolation`, `Patrol`, `Accuse`, and `PunishAccused` already use that contract through `GoalModelFallback` plus `is_progress_barrier()` in `crates/worldwake-ai/src/goal_model.rs:915-978`.
7. The current ticket text points at `crates/worldwake-ai/src/search.rs`, but search now lives under `crates/worldwake-ai/src/search/`. Correction applied: focused search fallout belongs in `search/mod.rs` and `search/tests.rs`.

## Architecture Check

1. PlannerOpKind still wraps existing actions — no new action handlers. The planner learns to use `post_bounty` and `post_notice` through the standard op classification pipeline.
2. Posting should reuse the existing leaf-only progress-barrier planner contract rather than inventing hypothetical artifact entities in planning state. That keeps the planner honest about what it can prove before authoritative commit.
3. `PostBounty` needs concrete stored bounty terms in goal identity or another shared planner-facing substrate; planner-only default reward/proof/claim-place policy would violate Principle 3 and Principle 25.
4. Goal dispatch still uses the declarative registration system (S36) — no special-case planner hooks.
5. Payload override validators already exist on both actions, so planner-synthesized payloads are safely revalidated at action start.
6. No backward-compatibility shims.

## Verification Layers

1. Planner-facing posting substrate carries lawful bounty-posting payload data without planner-only defaults -> focused core + payload-synthesis tests
2. `classify_action_def` maps `post_bounty` → `PostBounty`, `post_notice` → `PostNotice` -> focused planner-ops test
3. GoalDispatchDeclaration for `PostBounty` / `PostNotice` is live and no longer inert -> declaration lookup test
4. Search can construct `Travel -> PostNotice` and `Travel -> PostBounty` as leaf-only progress-barrier plans with synthesized payloads -> focused search tests
5. Cross-layer: planner ops (AI) reference existing action defs and payload validators (sim/systems) -> classify + payload synthesis coverage

## What to Change

### 1. Add planner-facing posting substrate

Broaden the current S51 posting goal contract so `PostBounty` can carry lawful bounty-posting terms for planner payload synthesis. Reuse shared social-artifact types instead of inventing planner-only defaults.

Expected outcome:
- a shared posting-context model for artifact issuance goals
- `GoalKind::PostBounty` carries concrete `BountyTerms` plus posting context
- `GoalKind::PostNotice` carries posting context plus topic
- downstream direct constructors, display helpers, and dispatch tests updated to the new shape

### 2. Add PlannerOpKind variants and classify action defs

In `crates/worldwake-ai/src/planner_ops.rs`:
- add `PostBounty` and `PostNotice` to `PlannerOpKind`
- map action names `"post_bounty"` and `"post_notice"` in `classify_action_def()`
- remove the temporary “intentionally unclassified” treatment from the existing tests

### 3. Add planner semantics

Add leaf-only planner semantics for `PostBounty` / `PostNotice` in `semantics_for()`:
- `Travel(posting_place) -> PostBounty/PostNotice`
- no hypothetical artifact creation in planning state
- terminal returned through `ProgressBarrier` once the posting action is reached
- payload synthesis delegated to `GoalKindPlannerExt::build_payload_override()`

### 4. Register GoalDispatchDeclarations

Convert the existing inert declarations in `crates/worldwake-ai/src/goal_dispatch_decl.rs` into live posting declarations:
- `relevant_ops` include `Travel` plus the matching posting op
- strategies are corrected to the strongest honest current planner contract after reassessment, rather than left as `NO_OPS`

### 5. Synthesize posting payloads and progress-barrier termination

In `crates/worldwake-ai/src/goal_model.rs`:
- synthesize lawful `ActionPayload::PostBounty` and `ActionPayload::PostNotice` from the corrected goal data
- bind the posting ops to `posting_place`
- mark posting actions as progress barriers so search can return the terminal posting step without pretending the artifact already exists in hypothetical state

## Files to Touch

- `crates/worldwake-core/src/social_artifact.rs` (modify)
- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- downstream direct `GoalKind::PostBounty` / `PostNotice` constructor sites discovered by compile fallout (modify)

## Out of Scope

- Candidate generation — ticket 003
- Showcase scenario tuning and goldens — ticket 004
- Golden tests — ticket 004
- New action handlers (existing post_bounty/post_notice actions are reused)

## Acceptance Criteria

### Tests That Must Pass

1. The corrected posting goal substrate can represent a lawful `PostBounty` planner payload without planner-only defaults
2. `classify_action_def` maps `post_bounty` → `PlannerOpKind::PostBounty`
3. `classify_action_def` maps `post_notice` → `PlannerOpKind::PostNotice`
4. GoalDispatchDeclaration lookup for `PostBounty` / `PostNotice` returns live declarations with posting ops
5. Planner can find `Travel -> PostNotice` and `Travel -> PostBounty` progress-barrier plan shapes
6. Existing suite: `cargo test --workspace`

### Invariants

1. PlannerOpKind remains Copy
2. Posting goals carry concrete bounty terms and posting context rather than planner-only default reward/proof policy
3. Goal dispatch uses declarative registration — no special-case planner code
4. Payload override validators on `post_bounty` / `post_notice` ensure planner-synthesized payloads are revalidated

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — posting-goal substrate and `GoalKey` constructor coverage
2. `crates/worldwake-ai/src/planner_ops.rs` — classify_action_def and semantics tests for posting ops
3. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — declaration registration and lookup tests for live posting declarations
4. `crates/worldwake-ai/src/search/tests.rs` — focused planner search tests for `Travel -> PostNotice` and `Travel -> PostBounty` progress-barrier plan shapes
5. `crates/worldwake-ai/src/goal_model.rs` — payload synthesis and posting progress-barrier focused tests

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-05

What changed:
- Added the shared `ArtifactPostingContext` substrate in `crates/worldwake-core/src/social_artifact.rs` and re-exported it from `crates/worldwake-core/src/lib.rs`.
- Broadened `GoalKind::PostBounty` and `GoalKind::PostNotice` in `crates/worldwake-core/src/goal.rs` so planner-visible posting goals now carry lawful posting context, and `PostBounty` carries full `BountyTerms` instead of the old under-specified target/place pair.
- Added `PlannerOpKind::PostBounty` and `PlannerOpKind::PostNotice` plus live `classify_action_def()` mappings in `crates/worldwake-ai/src/planner_ops.rs`.
- Converted the inert posting declarations in `crates/worldwake-ai/src/goal_dispatch_decl.rs` into live `Travel + Post*` planner surfaces.
- Added posting payload synthesis, posting-place binding, posting progress-barrier handling, and colocated posting root synthesis in `crates/worldwake-ai/src/goal_model.rs`.
- Added focused planner/search proofs in `crates/worldwake-ai/src/search/tests.rs` for `Travel -> PostNotice` and `Travel -> PostBounty`, plus focused goal-model payload/root-synthesis coverage.
- Updated bounded downstream exhaustive handling in `crates/worldwake-ai/src/agent_tick/observation.rs`, `crates/worldwake-ai/src/failure_handling.rs`, and `crates/worldwake-cli/src/display.rs` so the new posting ops are compile-safe and truthfully represented.

Deviations from original plan:
- The ticket had to be corrected before coding because the original `PostBounty { target, posting_place }` shape could not lawfully synthesize `post_bounty` payloads under the live action contract.
- Focused search verification exposed a real lower-layer contradiction after the main planner wiring landed: posting goals still lacked colocated `ActorPlace` root synthesis, so search could remote-travel forever without surfacing the posting leaf once co-located. That production fix was absorbed inside this ticket’s corrected planner boundary.

Verification results:
- `cargo test -p worldwake-core`
- `cargo test -p worldwake-ai post_bounty_builds_payload_override_from_goal_terms -- --nocapture`
- `cargo test -p worldwake-ai grounded_goal_synthesizes_post_notice_root_targets_when_colocated_with_posting_place -- --nocapture`
- `cargo test -p worldwake-ai fulfill_post_notice_search_finds_travel_then_post_notice_progress_barrier -- --nocapture`
- `cargo test -p worldwake-ai fulfill_post_bounty_search_finds_travel_then_post_bounty_progress_barrier -- --nocapture`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
