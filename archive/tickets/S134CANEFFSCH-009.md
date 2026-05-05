# S134CANEFFSCH-009: Justice, office, and artifact schemas

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — replaces empty-placeholder schemas with real category-owned `EffectSchema` literals across 12 justice/office/artifact actions and switches their commit handler bodies to `apply_effects_with_context(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md, archive/tickets/S134CANEFFSCH-008.md

## Problem

S134 deliverable D5 requires migrating the institutional-action family — justice (accuse, fine, exile in `justice_actions.rs`), office (bribe, threaten, declare_support, press_force_claim, yield_force_claim in `office_actions.rs`), and artifact (post_bounty, post_notice, claim_bounty, withdraw_bounty in `artifact_actions.rs`) — to declarative `EffectSchema` evaluation. This is a Large category by action count (12 actions) and by semantic surface (institutional artifacts, office-claim semantics, bounty-record creation/consumption, force-claim closure). The political-claim closure surface is exercised here through `press_force_claim`/`yield_force_claim`; the live closure checks remain in the category-owned authoritative step so the existing payload/current-world validation is preserved exactly. The planner continues to use the old `apply_hypothetical_transition` path until ticket 010; goldens for these actions must produce identical event logs.

## Assumption Reassessment (2026-05-04)

1. Justice/office/artifact registrations span 3 files in `crates/worldwake-systems/src/`:
   - `justice_actions.rs` — `register_accuse_action`, `register_fine_action`, `register_exile_action`
   - `office_actions.rs` — `register_office_actions` composite + `register_bribe_action`, `register_threaten_action`, `register_declare_support_action`, `register_press_force_claim_action`, `register_yield_force_claim_action`
   - `artifact_actions.rs` — `register_artifact_actions` composite + `register_post_bounty_action`, `register_post_notice_action`, `register_claim_bounty_action`, `register_withdraw_bounty_action`
2. After ticket 001, each `ActionDef` literal had `effect_schema: EffectSchema::empty()`. This ticket populated the 12 live justice/office/artifact definitions.
3. Office-claim semantics: `press_force_claim` and `yield_force_claim` exercise the political-claim substrate. The live validator/commit helper carries support, eligibility, force-law, duplicate-claim, incumbent, and local-membership checks through the authoritative category step; this ticket did not split those domain checks into generic `EffectPrecondition` variants.
4. Bounty/notice creation: `post_bounty` and `post_notice` create artifact entities with issuer, terms, reward source, proof requirements, location, expiration, contention state, reward encumbrance, and obligation tracker aftermath. Tickets 007 and 008 did not add a generic `CreateEntity` or reusable record step, so this ticket landed category-owned artifact steps instead.
5. Existing focused/unit coverage:
   - Per-file `#[cfg(test)]` blocks
   - Goldens — live coverage for this family is in `golden_offices.rs`, `golden_survival_justice.rs`, and `golden_survival_offices.rs`.
   - Conformance tests: `conformance_accuse` (line 1327), `conformance_declare_support` (line 1832), `conformance_press_force_claim` (line 1908) at `planner_conformance.rs`.
6. Composite registrations (`register_office_actions`, `register_artifact_actions`) wrap the individual register functions — confirm during reassessment whether `ActionDef` literals are constructed in the individual functions or in the composite (likely individual). The construction-site count for this ticket is roughly 13 (one per action).
7. Bitwise-identical event-log invariant: every justice event (`EventTag::Accuse`, `EventTag::Fine`, `EventTag::Exile`), every office event (`EventTag::Bribe`, `EventTag::SupportDeclared`, `EventTag::ForceClaimPressed`, etc.), and every artifact event (`EventTag::BountyPosted`, `EventTag::BountyClaimed`, `EventTag::NoticePosted`, etc.) must have identical timing and payload pre- and post-ticket.

## Architecture Check

1. Institutional-action declarative schemas align with FND-23 (Roles, Offices, and Institutions Are World State) and FND-25 (Social Artifacts Are First-Class) — every authoritative effect that a justice/office/artifact action produces becomes a typed schema step rather than handler-internal logic. Improves auditability for the political-claim closure surface (S133).
2. `press_force_claim` and `yield_force_claim` preserve the live closure boundary by delegating through category-owned authoritative effect steps that call the existing validation/mutation helpers. This keeps support declaration, visible vacancy/incumbent state, force-law succession eligibility, duplicate claim, and claim withdrawal checks aligned with the old handler.
3. Artifact-creation steps (`post_bounty`, `post_notice`) instantiate full artifact entities with all live metadata and aftermath (issuer, terms/content, reward source, proof requirements, expiration, location, contention state, reward encumbrance, and obligation tracker update). The category-owned artifact steps match the imperative handler's initialization.

## Verification Layers

1. Bitwise-identical event-log invariant → event-log delta on justice/office/artifact goldens.
2. Office-claim closure invariant → focused unit/runtime tests prove the schema-driven authoritative step rejects and commits through the same `validate_press_force_claim_context_in_world`, `validate_yield_force_claim_context_in_world`, `txn.add_force_claim`, and `txn.remove_force_claim` seams as before.
3. Artifact-creation invariant → event-log delta: `post_bounty` creates an artifact entity with all expected components (issuer, terms, reward source, location, expiration); `claim_bounty` consumes an artifact with all expected proof checks.
4. Conformance-tests parity → `conformance_accuse`, `conformance_declare_support`, `conformance_press_force_claim` continue to pass.
5. Canonical state hash invariant → soak: identical hashes on the three soak scenarios.

## What to Change

### 1. Construct `EffectSchema` literals for 3 justice actions

- **accuse**: `EffectStep::Accuse`, interpreted by the justice authoritative sink through the existing accusation validation, crime-register append, and institutional-belief projection helper.
- **fine**: `EffectStep::Fine`, interpreted by the justice authoritative sink through existing office-authority validation, fine arithmetic, controlled-commodity transfer, and verdict supersession helper.
- **exile**: `EffectStep::Exile`, interpreted by the justice authoritative sink through existing office-authority validation, faction-membership removal, hostility aftermath, and verdict supersession helper.

### 2. Construct `EffectSchema` literals for 5 office actions

- **bribe**: `EffectStep::Bribe`, interpreted by the office authoritative sink through the existing payload/target validation, transfer, and loyalty update helper.
- **threaten**: `EffectStep::Threaten`, interpreted by the office authoritative sink through existing combat-profile/courage comparison and loyalty-or-hostility aftermath.
- **declare_support**: `EffectStep::DeclareSupport`, interpreted by the office authoritative sink through existing eligibility/jurisdiction validation, support declaration, institutional belief projection, and target tagging.
- **press_force_claim**: `EffectStep::PressForceClaim`, interpreted by the office authoritative sink through existing force-law/jurisdiction/eligibility/duplicate-claim validation, force-claim write, incumbent-hostility aftermath, and target tagging.
- **yield_force_claim**: `EffectStep::YieldForceClaim`, interpreted by the office authoritative sink through existing local-claim validation, force-claim removal, and target tagging.

### 3. Construct `EffectSchema` literals for 4 artifact actions

- **post_bounty**: `EffectStep::PostBounty`, interpreted by the artifact authoritative sink through existing posting validation, social-artifact component construction, contention setup, reward encumbrance, obligation tracker update, and target tagging.
- **post_notice**: `EffectStep::PostNotice`, interpreted by the artifact authoritative sink through existing posting validation, notice artifact component construction, obligation tracker update, and target tagging.
- **claim_bounty**: `EffectStep::ClaimBounty`, interpreted by the artifact authoritative sink through existing bounty/proof/grant validation, reward transfer, encumbrance release, fulfilled-state mutation, contention cleanup, and target tagging.
- **withdraw_bounty**: `EffectStep::WithdrawBounty`, interpreted by the artifact authoritative sink through existing issuer validation, encumbrance release, withdrawn-state mutation, and target tagging.

### 4. Replace commit handler bodies with `apply_effects_with_context` delegation

Each `commit_*` handler in the 3 files shrinks to the standard delegation. Remove imperative bodies.

### 5. Add category-owned `EffectStep` variants

Landed variants: `Accuse`, `Fine`, `Exile`, `Bribe`, `Threaten`, `DeclareSupport`, `PressForceClaim`, `YieldForceClaim`, `PostBounty`, `PostNotice`, `ClaimBounty`, and `WithdrawBounty`. The shared sink trait default-rejects these steps with `Discrepancy::ImproperPlanningState`; only the owning authoritative module sinks implement them. Hypothetical parity remains ticket 010 scope.

## Files to Touch

- `crates/worldwake-systems/src/justice_actions.rs` (modify — 3 schemas, 3 commit handler body replacements)
- `crates/worldwake-systems/src/office_actions.rs` (modify — 5 schemas, 5 commit handler body replacements)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify — 4 schemas, 4 commit handler body replacements)
- `crates/worldwake-sim/src/effect_schema.rs` (modify — add the 12 category-owned `EffectStep` variants and default-rejecting `EffectSink` methods)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` (no change — the module-local authoritative sinks own these category steps)
- `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (no change — default rejection remains until ticket 010 implements hypothetical parity)

## Out of Scope

- Migrating non-justice/office/artifact actions (tickets 003–008).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Changing political-claim closure semantics, bounty lifecycle (S140 territory), or office-holder rules.
- Conformance test rewrite (ticket 010).

## Acceptance Criteria

### Tests That Must Pass

1. All justice/office/artifact-touching goldens produce bitwise-identical event logs.
2. Conformance tests `conformance_accuse`, `conformance_declare_support`, `conformance_press_force_claim` continue to pass.
3. Existing inline tests pass with the schema-driven path through the valid module filters `justice_actions`, `office_actions`, and `artifact_actions`.
4. Live justice/office survival goldens pass through `golden_offices`, `golden_survival_justice`, and `golden_survival_offices`; ignored long-run cases were run explicitly with `-- --ignored`.
5. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `press_force_claim`'s schema preconditions reject the same scenarios the imperative handler rejects (closure-boundary preservation per Rule 10).
2. `post_bounty`/`post_notice` create artifact entities with the same component set (issuer, terms, reward source, proof requirements, expiration) as the imperative handler creates today.
3. Bitwise-identical canonical state hash on the three soak scenarios.
4. Bounty-claim reward transfer uses the same source-treasury and amount as today.

## Test Plan

### New/Modified Tests

1. Per-file `#[cfg(test)]` blocks — existing commit/runtime tests now exercise schema-driven commit delegation, with registration tests asserting the landed category step per action family.
2. Existing goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems --lib justice_actions`
2. `cargo test -p worldwake-systems --lib office_actions`
3. `cargo test -p worldwake-systems --lib artifact_actions`
4. `cargo test -p worldwake-ai --test planner_conformance conformance_accuse`
5. `cargo test -p worldwake-ai --test planner_conformance conformance_declare_support`
6. `cargo test -p worldwake-ai --test planner_conformance conformance_press_force_claim`
7. `cargo test -p worldwake-ai --test golden_offices`
8. `cargo test -p worldwake-ai --test golden_survival_justice`
9. `cargo test -p worldwake-ai --test golden_survival_justice -- --ignored`
10. `cargo test -p worldwake-ai --test golden_survival_offices`
11. `cargo test -p worldwake-ai --test golden_survival_offices -- --ignored`
12. `cargo test -p worldwake-systems`
13. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-05.

- Added category-owned S134 `EffectStep` variants for the 12 justice/office/artifact actions and default-rejecting `EffectSink` methods for unsupported modes.
- Populated `ActionDef.effect_schema` for `accuse`, `fine`, `exile`, `bribe`, `threaten`, `declare_support`, `press_force_claim`, `yield_force_claim`, `post_bounty`, `post_notice`, `claim_bounty`, and `withdraw_bounty`.
- Replaced each commit handler body with `apply_effects_with_context(..., EffectMode::Authoritative)` delegation through a module-local authoritative sink.
- Preserved the existing domain mutation helpers as the authoritative sink implementations, so crime-register, justice disposition, support/force-claim, bounty artifact, reward encumbrance, contention, and obligation-tracker aftermath remain bitwise-aligned with the pre-migration path.
- Added focused registration assertions for the new category steps in the existing justice, office, and artifact test modules.
- `SAVE_FORMAT_VERSION` remains `66`: the new schema variants are registry-time `ActionDef` data and do not change persisted world/save state.

## Deviations

- The draft counted 13 actions; the live family has 12 actions: 3 justice, 5 office, and 4 artifact.
- The draft sketched generic preconditions/record/creation/mutation steps. Live reassessment showed those would flatten domain aftermath incorrectly, so the landed seam uses typed category-owned steps with authoritative sink implementations. Planner hypothetical interpretation remains ticket 010 scope.
- The drafted commands `cargo test -p worldwake-systems justice office artifact`, `cargo test -p worldwake-ai conformance_accuse conformance_declare_support conformance_press_force_claim`, `golden_bounty`, and broad `golden_survival` were rebound to valid live selectors and binaries.
- `./scripts/verify.sh` was not run because this was not a PR-push preparation pass; the CI-shaped clippy gate and the ticket-owned focused/broadened tests listed below were run directly.

## Verification Result

- Passed `cargo test -p worldwake-systems --no-run`
- Passed `cargo test -p worldwake-systems --lib justice_actions::tests::register_accuse_action_creates_public_crime_definition -- --exact`
- Passed `cargo test -p worldwake-systems --lib office_actions::tests::register_office_actions_creates_social_defs -- --exact`
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::register_artifact_actions_creates_expected_definitions -- --exact`
- Passed `cargo test -p worldwake-systems --lib justice_actions`
- Passed `cargo test -p worldwake-systems --lib office_actions`
- Passed `cargo test -p worldwake-systems --lib artifact_actions`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo test -p worldwake-ai --test planner_conformance conformance_accuse`
- Passed `cargo test -p worldwake-ai --test planner_conformance conformance_declare_support`
- Passed `cargo test -p worldwake-ai --test planner_conformance conformance_press_force_claim`
- Passed `cargo test -p worldwake-ai --test golden_offices`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_offices`
- Passed `cargo test -p worldwake-ai --test golden_survival_offices -- --ignored`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
