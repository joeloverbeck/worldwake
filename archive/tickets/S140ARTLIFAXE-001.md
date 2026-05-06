# S140ARTLIFAXE-001: Axis types, ArtifactHeader refactor, ArtifactState elimination, workspace-wide consumer migration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `ArtifactHeader` shape, `ArtifactState` removal, `EventTag::ArtifactTransition` addition, `BelievedArtifactState` restructure, `EntityBeliefClaim::ArtifactState` → `EntityBeliefClaim::Artifact` rename, `SAVE_FORMAT_VERSION` 70→71, all workspace consumer migrations
**Deps**: specs/S140-artifact-lifecycle-axes.md (D1, D2, D4, D5; partial D3 for mechanical mutation-site substitution)

## Problem

Today `ArtifactState` (`crates/worldwake-core/src/social_artifact.rs:55-61`) is a single 5-variant discriminator (`Active, Fulfilled, Expired, Withdrawn, Destroyed`) collapsing five orthogonal concerns FND-25A says must vary independently: existence, visibility, legal effect, credibility, and actionability. The collapse means an exonerated accusation has no representation distinct from "expired", an expired-but-still-posted bounty cannot remain inspectable as required by FND-25A, and a refuted rumor cannot persist for FOUNDATIONS Scenario G's exoneration chain. This foundation ticket replaces the flat enum with five typed orthogonal axis fields on `ArtifactHeader`, adds the supporting transition event, eliminates `ArtifactState` per FND-28 (no shim), and migrates every workspace consumer in a single compile-coherent step. Subsequent tickets layer the lifecycle handler refactor (002), the planner observability (003), the unified scenario authoring surface (004), the observer rendering (005), and the E2E goldens (006) on top.

## Assumption Reassessment (2026-05-06)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ArtifactState` lives at `crates/worldwake-core/src/social_artifact.rs:55-61` with 5 variants. `ArtifactHeader` (`social_artifact.rs:5-13`) currently has 7 fields: `kind, issuer, issuing_authority, created_at, expires_at, state, jurisdiction`. Existing inline tests in `social_artifact.rs`: `social_artifact_types_satisfy_required_traits:147`, `artifact_header_roundtrips_through_bincode:163`, `bounty_and_notice_types_roundtrip_through_bincode:180`, `artifact_posting_profile_default_matches_spec_defaults:226`. Existing `artifact_lifecycle.rs` inline tests: `artifact_lifecycle_system_expires_active_artifact_at_expiration_tick:194`, `artifact_lifecycle_system_leaves_nonexpiring_artifact_active:222`, `artifact_lifecycle_system_does_not_expire_before_expiration_tick:250`, `bounty_ttl_expiry_releases_encumbrance:278`. Existing `artifact_actions.rs` inline tests: `claim_bounty_transfers_reward_and_fulfills_bounty:2670`, `claim_bounty_consumes_encumbrance_and_transfers_lot_to_claimant:2756`, `withdraw_bounty_releases_encumbrance_without_transfer:2841`, `claim_bounty_rejects_second_claimant_in_race_mode:2917`, `claim_bounty_depleted_source_fails_and_bounty_stays_active:2998`, `claim_bounty_rejects_when_proof_is_insufficient:3099`, `claim_bounty_rejects_when_bounty_is_already_fulfilled:3152`, `claim_bounty_affordance_targets_known_remote_bounty_by_identity:3213`. All require updates to construct/assert against the new axis shape per the spec's Migration Map.
2. Spec deliverables D1, D2, D4, D5 plus the Migration Map define the per-instance variant→axis mapping. `SAVE_FORMAT_VERSION` is currently `70` at `crates/worldwake-sim/src/save_load.rs:6` and is bumped to `71` by this ticket; the migration is real-bump per FND-28 (Non-Goal 4 in the spec). `EntityBeliefClaim::ArtifactState(Option<BelievedArtifactState>)` lives at `crates/worldwake-core/src/entity_belief_claim.rs:42` and is renamed to `EntityBeliefClaim::Artifact(Option<BelievedArtifactState>)` here so the dead `ArtifactState` symbol does not survive in the variant name.
3. **Cross-system shared abstraction boundary**: `ArtifactHeader` is the single authoritative typed shape for posted artifacts; consumers in `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, and `worldwake-cli` read it via component getters. The boundary under audit is the type contract of `ArtifactHeader` plus the `EntityBeliefClaim::ArtifactState`-currently-named belief-side mirror of the same fact. Workspace-wide grep confirms 17 files reference `\bArtifactState\b` (5 in core, 3 in systems, 8 in ai, 1 in cli) and 10 files reference `BelievedArtifactState` (3 in core, 1 in systems, 6 in ai); per the no-shim FND-28 constraint, all sites must migrate in this single ticket so the workspace builds end-to-end.
4. **Adjacent-contradiction classification**: Reassess-spec found that `expires_at`, `issuing_authority`, and `jurisdiction` were originally proposed for elimination in the spec's draft `ArtifactHeader` shape but are heavily consumed (e.g., `artifact_actions.rs:455,460`). The spec's published Migration Map keeps these fields on the refactored header — this ticket follows the published Migration Map; the originally-proposed elimination is excluded from scope as a reassessment-driven correction. `created_at` is also preserved (no rename to `issued_at`). No new top-level `place` or `artifact_id` field is introduced.
5. **Information-path refactor stance**: The same lifecycle fact (e.g., "is this artifact actionable?") today travels through a single path: `header.state == ArtifactState::Active` (direct read of authoritative state). After this ticket, the canonical path is `header.actionability == ArtifactActionability::Actionable`. There is no temporary mixed-state coexistence; `ArtifactState` is removed in this same ticket, not deferred.
6. **Cross-axis writes at action commit handlers** (`artifact_actions.rs:1193,1293,1382,1497`) are placeholders in this ticket: where the existing code wrote a single `ArtifactState`, the post-001 code writes both the proximate axis (e.g., `legal_effect = Fulfilled`) and the cascaded axis (e.g., `actionability = Closed`) directly. Ticket S140ARTLIFAXE-002 replaces the cascaded direct write with an event-driven cascade through `artifact_lifecycle_system`'s actionability handler stage. The placeholder is named at each cross-axis-write site with a `// S140-001 placeholder, replaced by S140ARTLIFAXE-002` comment so 002's reviewer can locate the cleanup sites mechanically.

## Architecture Check

1. The single-shot foundation scope is forced by FND-28's no-shim rule and the workspace-compile constraint. Splitting `ArtifactState` removal across tickets would require either (a) a derived-view shim that violates FND-28 and creates two live authoritative representations of the same fact, or (b) an extended period where the workspace fails to compile. Both are worse than a Large but mechanically-bounded ticket.
2. `BTreeSet<EntityId>` collection types on `ArtifactVisibility::Private` and `ArtifactCredibility::Disputed` (rather than `SmallVec<EntityId, N>`) preserves the determinism invariant from CLAUDE.md (`BTreeMap`/`BTreeSet` only in authoritative state) and avoids adding `smallvec` to `worldwake-core`'s minimal dependency set (`serde, bincode, blake3`).
3. `Copy → Clone` migration on `ArtifactHeader`: the `BTreeSet` payloads force dropping the `Copy` derive. The 9 existing struct-literal construction sites are migrated to `clone()` at usage points where copy semantics were assumed. No widening of the type's mutability surface.
4. The `EntityBeliefClaim::ArtifactState` variant is renamed to `EntityBeliefClaim::Artifact` to keep the `\bArtifactState\b` no-shim grep guard (landed by ticket S140ARTLIFAXE-006) clean — leaving the legacy variant name would force the guard to special-case the substring.

## Verification Layers

1. `ArtifactHeader` field-set contract → focused unit/runtime test (`social_artifact.rs` inline `artifact_header_roundtrips_through_bincode` updated to construct the 5-axis shape and verify bincode round-trip; new unit test asserting all 5 axis fields default to the historical `Active` shape per the Migration Map).
2. `ArtifactState` symbol absence (no live alias) → grep guard, exercised by ticket S140ARTLIFAXE-006's `scripts/check_no_artifact_state.sh`. This ticket establishes the post-condition; 006 enforces it.
3. `EventTag::ArtifactTransition` decode roundtrip → focused unit test in the event-tag module (or `event_log` decoding tests) constructing an `ArtifactTransitionPayload` and verifying bincode roundtrip.
4. `BelievedArtifactState` per-axis shape → focused unit test verifying the belief mirror carries the same 5 axis values an observer could have read directly (FND-15 fidelity).
5. Existing TTL-expiry behavior preserved at `artifact_lifecycle_system` → existing inline tests (`artifact_lifecycle_system_expires_active_artifact_at_expiration_tick`, `bounty_ttl_expiry_releases_encumbrance`) updated to assert axis values rather than `ArtifactState::Expired`. Behavior is unchanged; the assertion surface migrates.
6. Single-layer scope notes: this ticket is an authoritative-type migration. The cross-axis event-driven cascade is verified at S140ARTLIFAXE-002; the planner-side observability is verified at 003; E2E behavior is verified at 006. Mapping each invariant to its strongest available proof surface here rather than attempting to assert event-driven cross-axis flow before its handler exists.

## What to Change

### 1. New axis enums + supporting Copy enums in `worldwake-core/src/social_artifact.rs`

Define five axis enums per spec D1 with the exact variants and payloads listed. Define six supporting Copy enums (`DestructionCause, SuspensionReason, RevocationReason, ProofKind, BlockerReason, CloseCause`) with the initial-variant sets named in the spec. Use `BTreeSet<EntityId>` for `ArtifactVisibility::Private.audience` and `ArtifactCredibility::Disputed.contradicting`. Verify all derives (`Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`) are satisfied. `ProofKind` mirrors existing `ProofRequirement` variant set; the existing `ProofRequirement` enum is preserved unchanged (S140 does not rename it; the dual existence is documented inline since `ProofKind` belongs to actionability while `ProofRequirement` belongs to bounty terms).

### 2. New transition payload types in `worldwake-core/src/social_artifact.rs`

Define `AxisName`, `ArtifactAxisValue`, `ArtifactTransitionPayload` per spec D2. `ArtifactAxisValue` is a sum type carrying the variant of whichever axis is transitioning. `ArtifactTransitionPayload` carries `(artifact: EntityId, axis: AxisName, prior: ArtifactAxisValue, new: ArtifactAxisValue, cause_event: Option<EventId>, at: Tick)`.

### 3. `ArtifactHeader` refactor (drop `state`, add 5 axis fields, drop `Copy`)

Replace `state: ArtifactState` with five typed axis fields per spec D1. Preserve `kind, issuer, issuing_authority, created_at, expires_at, jurisdiction` per the Migration Map. Drop `Copy` derive (forced by `BTreeSet<EntityId>` on `Private` and `Disputed`); keep `Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`. Update all 9 struct-literal construction sites to populate the new fields with the Migration Map defaults (typically `existence: Exists, visibility: Posted { place }, legal_effect: Active { expires_at }, credibility: Credible, actionability: Actionable`). Migrate 9 sites: `social_artifact.rs:164` (test), `artifact_actions.rs:1193,1382` (post handlers), plus the construction sites discovered during implementation in scenario/test code. At any site that previously copied the header, migrate to `.clone()`.

### 4. Remove `ArtifactState` enum

Delete the `pub enum ArtifactState { ... }` block at `social_artifact.rs:55-61` and remove its `assert_traits::<ArtifactState>` line in the inline test. Update all imports across the workspace (`use worldwake_core::ArtifactState;` lines must be deleted).

### 5. Add `EventTag::ArtifactTransition` variant in `worldwake-core/src/event_tag.rs`

Add a new `ArtifactTransition` variant alongside the existing 44 variants. Confirm the variant participates in serde derives identically to siblings. Update any decode/dispatch tables in `worldwake-sim/src/event_log` (or wherever event-tag dispatch lives) so the new variant carries `ArtifactTransitionPayload` decoding.

### 6. `BelievedArtifactState` restructure in `worldwake-core/src/belief.rs`

Replace the current single-`state: ArtifactState` field on `BelievedArtifactState` with the five axis fields mirroring `ArtifactHeader`. Preserve other belief fields. Update all 10 consumer files across `worldwake-core, worldwake-systems, worldwake-ai` to construct/read the new shape. The belief mirror carries the same five axis values an observer could have read directly; FND-15 provenance metadata stays as-is on the enclosing belief envelope.

### 7. `EntityBeliefClaim::ArtifactState` → `EntityBeliefClaim::Artifact` variant rename in `worldwake-core/src/entity_belief_claim.rs:42`

Rename the variant. Grep workspace-wide for both `EntityBeliefClaim::ArtifactState` (qualified form) and unqualified usage via `use EntityBeliefClaim::*;` patterns. Update every match arm and construction site.

### 8. Workspace-wide `ArtifactState` consumer migration per Migration Map

Walk every site that reads `header.state` or constructs `ArtifactState::*`. Per the Migration Map (in spec):

- `header.state == ArtifactState::Active` → choose the axis the site cares about. Most planner-gate equality checks become `header.actionability == ArtifactActionability::Actionable`. Most lifecycle/belief reads become `matches!(header.legal_effect, ArtifactLegalEffect::Active { .. })`.
- `header.state = ArtifactState::Expired` (lifecycle TTL at `artifact_lifecycle.rs:43`) → `header.legal_effect = ArtifactLegalEffect::Expired { expired_at: tick };` and `header.actionability = ArtifactActionability::Closed { closed_at: tick, cause: CloseCause::LegalEffectExpired };` (placeholder cross-axis write — tagged with the comment per assumption-reassessment item 6; replaced in S140ARTLIFAXE-002).
- `header.state = ArtifactState::Fulfilled` (`artifact_actions.rs:1497`) → `header.legal_effect = ArtifactLegalEffect::Fulfilled { fulfilled_at, by, evidence };` plus placeholder `header.actionability = ArtifactActionability::Closed { closed_at, cause: CloseCause::BountyFulfilled };`.
- `header.state = ArtifactState::Withdrawn` (`artifact_actions.rs:1293`) → `header.legal_effect = ArtifactLegalEffect::Revoked { revoked_at, by: issuer, reason: RevocationReason::IssuerWithdrawal };` plus placeholder `header.actionability = ArtifactActionability::Closed { closed_at, cause: CloseCause::Revoked };`.
- `state: ArtifactState::Active` in struct literals (e.g., `scenario/mod.rs:983,1751`, `artifact_actions.rs:1193`) → expand to the 5 axis defaults per the Migration Map (via the new fields).

Affected files with known sites (full list discovered during implementation):

- `crates/worldwake-systems/src/artifact_lifecycle.rs:24,43,124,160,216,244,272,307`
- `crates/worldwake-systems/src/artifact_actions.rs:651,683,829,1193,1293,1382,1497,1880,2735,2827,2907,3094,3173,3249`
- `crates/worldwake-systems/src/perception.rs:855,6293,6342,6383,6474`
- `crates/worldwake-core/src/belief.rs:7240,7264,7349,7375` (BelievedArtifactState construction)
- `crates/worldwake-ai/src/candidate_generation.rs:650,7951,8017,8076,8155` (planner gate at 650 → `actionability == Actionable`; others case-by-case)
- `crates/worldwake-ai/src/exhaustion.rs:411,1056,1104,1159,1211`
- `crates/worldwake-ai/src/goal_model.rs:1191,8265,8334,8578,8657,8742`
- `crates/worldwake-ai/src/ranking.rs:3685`
- `crates/worldwake-ai/src/route_threat.rs:64,297`
- `crates/worldwake-cli/src/scenario/mod.rs:983,1751`
- `crates/worldwake-ai/tests/golden_offices.rs:2249`
- `crates/worldwake-ai/tests/golden_survival_justice.rs:329`

### 9. `SAVE_FORMAT_VERSION` 70 → 71 in `worldwake-sim/src/save_load.rs:6`

Bump the constant. Pre-S140 saves are explicitly not loadable post-S140 per spec Non-Goal 4 (FND-28). Update version-check assertions (`save_load.rs:101,129,132,1075`) and any save-format-migration scaffolding to reflect the new value. No inverse migration shim is provided.

### 10. Update existing tests to assert axis shape

Update inline tests in `social_artifact.rs, artifact_lifecycle.rs, artifact_actions.rs, perception.rs, exhaustion.rs, ranking.rs, route_threat.rs, goal_model.rs` plus the two integration tests `golden_offices.rs:2249`, `golden_survival_justice.rs:329` to assert axis values rather than `ArtifactState` variants. The tests are migrating the assertion surface, not changing the contract.

## Files to Touch

- `crates/worldwake-core/src/social_artifact.rs` (modify — define 5 axis enums + 6 supporting Copy enums + `AxisName, ArtifactAxisValue, ArtifactTransitionPayload`; refactor `ArtifactHeader`; remove `ArtifactState`)
- `crates/worldwake-core/src/event_tag.rs` (modify — add `ArtifactTransition` variant)
- `crates/worldwake-core/src/belief.rs` (modify — `BelievedArtifactState` restructure)
- `crates/worldwake-core/src/entity_belief_claim.rs` (modify — variant rename `ArtifactState` → `Artifact`)
- `crates/worldwake-core/src/lib.rs` (modify — adjust pub re-exports for new axis types and removed `ArtifactState`)
- `crates/worldwake-systems/src/artifact_lifecycle.rs` (modify — TTL mutation + tests)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify — 4 mutation sites with placeholder cross-axis writes; ~14 read sites; tests)
- `crates/worldwake-systems/src/perception.rs` (modify — `BelievedArtifactState` construction)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — planner gate + 4 other reads)
- `crates/worldwake-ai/src/exhaustion.rs` (modify — equality check sites)
- `crates/worldwake-ai/src/goal_model.rs` (modify — equality check sites)
- `crates/worldwake-ai/src/ranking.rs` (modify — equality check sites)
- `crates/worldwake-ai/src/route_threat.rs` (modify — equality check sites + `BelievedArtifactState` construction)
- `crates/worldwake-ai/src/search/tests.rs` (modify — shared planner test fixtures migrated to believed artifact axes)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — 2 construction sites at lines 983, 1751)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` 70→71 + version-check assertions)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify — line 2249 assertion)
- `crates/worldwake-ai/tests/golden_survival_justice.rs` (modify — line 329 assertion)
- No-change cited file: `crates/worldwake-sim/src/event_log` was checked during implementation; `EventTag` indexing is generic and did not require a dispatch-table edit for `ArtifactTransition`.

## Out of Scope

- `artifact_lifecycle_system` 5-stage refactor and event-driven cross-axis cascades — covered by S140ARTLIFAXE-002.
- `Discrepancy::ArtifactNotActionable` variant addition and decision-trace axis surfacing — covered by S140ARTLIFAXE-003.
- Unified `ArtifactDef` scenario authoring (`NoticeDef` rename + payload sum type) — covered by S140ARTLIFAXE-004.
- Observer Section 11 (Artifact Lifecycle) rendering — covered by archive/tickets/S140ARTLIFAXE-005.md.
- E2E goldens and `\bArtifactState\b` regression grep guard — covered by S140ARTLIFAXE-006.
- `SaleListing` migration into the artifact taxonomy — explicit Non-Goal in spec.
- Promoting accusations to `ArtifactHeader`-backed artifacts — explicit Non-Goal in spec.
- Broadening planner actionability gating beyond the existing `FulfillBounty` site — explicit Non-Goal in spec.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core --lib social_artifact` — inline tests pass with axis-shape assertions.
2. `cargo test -p worldwake-systems --lib artifact_lifecycle` — TTL expiry tests pass with axis-value assertions; behavior unchanged.
3. `cargo test -p worldwake-systems --lib artifact_actions` — fulfillment, withdrawal, expiry, and claim-rejection tests pass with axis-value assertions.
4. `cargo test -p worldwake-ai --test golden_offices` — passes with axis-value setup.
5. `cargo test -p worldwake-ai --test golden_survival_justice` — passes with axis-value setup.
6. Existing suite: `cargo test --workspace`.

### Invariants

1. The symbol `\bArtifactState\b` does not appear in `crates/` after this ticket lands (enforced by S140ARTLIFAXE-006's grep guard, but verified visually here as the final post-condition of step 4).
2. `ArtifactHeader` carries exactly the 5 axis fields plus the 6 preserved identity/jurisdiction fields; no additional or dropped fields beyond what the Migration Map prescribes.
3. `BelievedArtifactState`'s shape mirrors the public axis fields of `ArtifactHeader`; the belief mirror does not carry any axis the live header does not.
4. `SAVE_FORMAT_VERSION == 71` post-ticket; pre-S140 saves (version 70 or lower) cannot deserialize per FND-28.
5. `EntityBeliefClaim` no longer carries an `ArtifactState`-named variant.
6. `ArtifactPostingProfile` (S97) is unchanged — its TTL semantics continue to govern the legal-effect `Active → Expired` transition.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/social_artifact.rs` (modify) — extend `artifact_header_roundtrips_through_bincode` to construct the 5-axis shape; add a unit test `artifact_header_axis_defaults_match_migration_map` asserting that the historical `ArtifactState::Active` mapping (`Exists | Posted | Active | Credible | Actionable`) is producible.
2. `crates/worldwake-core/src/social_artifact.rs` (modify) — add a unit test `artifact_transition_payload_roundtrips_through_bincode` for the new `ArtifactTransitionPayload` shape.
3. `crates/worldwake-core/src/belief.rs` (modify) — extend the artifact snapshot projection tests so the belief mirror asserts the 5-axis values.
4. `crates/worldwake-systems/src/artifact_lifecycle.rs` (modify) — update 4 inline tests to assert `legal_effect == Expired { .. }` rather than `state == Expired`.
5. `crates/worldwake-systems/src/artifact_actions.rs` (modify) — update inline tests to assert axis-value shape after fulfillment, withdrawal, claim rejection, and remote-bounty affordance paths.

### Commands

1. `cargo test -p worldwake-core --lib social_artifact -- --nocapture` (axis-shape unit tests)
2. `cargo test -p worldwake-systems --lib artifact_lifecycle` (TTL tests)
3. `cargo test -p worldwake-systems --lib artifact_actions` (commit-handler tests)
4. `cargo test -p worldwake-ai --test golden_offices` (normal non-ignored binary run)
5. `cargo test -p worldwake-ai --test golden_survival_justice` (normal non-ignored binary run)
6. `cargo test --workspace` (full workspace gate — confirms the cross-crate consumer migration compiles and passes)
7. `./scripts/verify.sh` (CI gate — fmt, full tests, active-goal guard, clippy, all-target clippy, scenario-coverage)

## Outcome

Completed on 2026-05-06.

- Replaced the flat `ArtifactState` enum with five orthogonal lifecycle axes on `ArtifactHeader`: existence, visibility, legal effect, credibility, and actionability.
- Added `AxisName`, `ArtifactAxisValue`, `ArtifactTransitionPayload`, and `EventTag::ArtifactTransition`, with focused bincode coverage.
- Migrated `BelievedArtifactState` to mirror the five header axes and renamed the belief-claim lane from `ArtifactState` to `Artifact`.
- Migrated workspace consumers, scenario spawning, AI candidate/search/ranking/exhaustion fixtures, perception, lifecycle expiry, bounty fulfillment/withdrawal handlers, and the two named golden assertion sites.
- Bumped `SAVE_FORMAT_VERSION` from 70 to 71. Pre-S140 saves remain rejected by the current format-version gate.

## Deviations

- `SuspensionReason` is defined in `worldwake_core::social_artifact` but is not crate-root re-exported because the crate root already exports an unrelated planner `SuspensionReason`. This avoids a public name collision while keeping the spec-named type in its owning module.
- `ClaimValue` now has a local `#[allow(clippy::large_enum_variant)]` because the per-axis `BelievedArtifactState` is intentionally carried inline in the persisted belief-claim payload. Boxing would add indirection solely to satisfy lint sizing and would make the current-format shape less direct.
- S140-001 keeps the ticket-specified direct cross-axis placeholder writes at expiry, withdrawal, and fulfillment sites. The event-driven cascade remains owned by `S140ARTLIFAXE-002`.

## Verification Result

- Passed `cargo test -p worldwake-core --lib social_artifact -- --nocapture`.
- Passed `cargo test -p worldwake-systems --lib artifact_lifecycle`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions`.
- Passed `cargo test -p worldwake-ai --test golden_offices`.
- Passed `cargo test -p worldwake-ai --test golden_survival_justice`.
- Passed `cargo test --workspace`.
- Passed `./scripts/verify.sh`, whose live gates are `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- Passed `rg -n '\bArtifactState\b' crates -g '*.rs'` with zero matches after the migration.
