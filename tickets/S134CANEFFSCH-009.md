# S134CANEFFSCH-009: Justice, office, and artifact schemas

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — replaces empty-placeholder schemas with real `EffectSchema` literals across 13 justice/office/artifact actions and switches their commit handler bodies to `apply_effects(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, S134CANEFFSCH-002

## Problem

S134 deliverable D5 requires migrating the institutional-action family — justice (accuse, fine, exile in `justice_actions.rs`), office (bribe, threaten, declare_support, press_force_claim, yield_force_claim in `office_actions.rs`), and artifact (post_bounty, post_notice, claim_bounty, withdraw_bounty in `artifact_actions.rs`) — to declarative `EffectSchema` evaluation. This is a Large category by action count (13 actions) and by semantic surface (institutional artifacts, office-claim semantics, bounty-record creation/consumption, force-claim closure). The political-claim closure surface (S133's territory) is exercised here through `press_force_claim`/`yield_force_claim` — preconditions must encode the closure boundary precisely. The planner continues to use the old `apply_hypothetical_transition` path; goldens for these actions must produce bitwise-identical event logs.

## Assumption Reassessment (2026-05-04)

1. Justice/office/artifact registrations span 3 files in `crates/worldwake-systems/src/`:
   - `justice_actions.rs` — `register_accuse_action`, `register_fine_action`, `register_exile_action`
   - `office_actions.rs` — `register_office_actions` composite + `register_bribe_action`, `register_threaten_action`, `register_declare_support_action`, `register_press_force_claim_action`, `register_yield_force_claim_action`
   - `artifact_actions.rs` — `register_artifact_actions` composite + `register_post_bounty_action`, `register_post_notice_action`, `register_claim_bounty_action`, `register_withdraw_bounty_action`
2. After ticket 001, each `ActionDef` literal has `effect_schema: EffectSchema::empty()`. This ticket populates real schemas.
3. Office-claim semantics: `press_force_claim` and `yield_force_claim` exercise the political-claim substrate (S133 territory). The schema's preconditions must encode the closure boundary precisely (per `docs/precision-rules.md` Rule 10):
   - **Support declaration**: BeliefHeld claim about support.
   - **Visible-vacancy loss**: precondition on no-existing-officeholder for the contested office.
   - **Succession resolution**: institutional precondition.
   - **Office-holder mutation**: the step itself.
4. Bounty/notice creation: `post_bounty` and `post_notice` create artifact entities with issuer, terms, reward source, proof requirements, location, expiration. Schema needs full artifact-creation semantics — likely uses `EffectStep::CreateEntity` (from ticket 007) or a more specific `CreateRecord` variant (from ticket 008).
5. Existing focused/unit coverage:
   - Per-file `#[cfg(test)]` blocks
   - Goldens — `golden_accuse_*.rs`, `golden_fine_*.rs`, `golden_exile_*.rs`, `golden_office_*.rs`, `golden_bounty_*.rs`, `golden_post_notice_*.rs`. Enumerate during reassessment.
   - Conformance tests: `conformance_accuse` (line 1327), `conformance_declare_support` (line 1832), `conformance_press_force_claim` (line 1908) at `planner_conformance.rs`.
6. Composite registrations (`register_office_actions`, `register_artifact_actions`) wrap the individual register functions — confirm during reassessment whether `ActionDef` literals are constructed in the individual functions or in the composite (likely individual). The construction-site count for this ticket is roughly 13 (one per action).
7. Bitwise-identical event-log invariant: every justice event (`EventTag::Accuse`, `EventTag::Fine`, `EventTag::Exile`), every office event (`EventTag::Bribe`, `EventTag::SupportDeclared`, `EventTag::ForceClaimPressed`, etc.), and every artifact event (`EventTag::BountyPosted`, `EventTag::BountyClaimed`, `EventTag::NoticePosted`, etc.) must have identical timing and payload pre- and post-ticket.

## Architecture Check

1. Institutional-action declarative schemas align with FND-23 (Roles, Offices, and Institutions Are World State) and FND-25 (Social Artifacts Are First-Class) — every authoritative effect that a justice/office/artifact action produces becomes a typed schema step rather than handler-internal logic. Improves auditability for the political-claim closure surface (S133).
2. `press_force_claim` schema preconditions must precisely match the closure boundary the existing handler asserts — per `docs/precision-rules.md` Rule 10, naming whether the closure is "support declaration / visible-vacancy loss / succession resolution / office-holder mutation". Schema preconditions encode the support-declaration and visible-vacancy-loss checks as `EffectPrecondition` variants; the office-holder-mutation is the step itself.
3. Artifact-creation steps (`post_bounty`, `post_notice`) instantiate full artifact entities with all S25/FND-25 metadata (issuer, terms, reward source, proof requirements, expiration). The schema's `CreateEntity` (or `CreateRecord`) step's component-set must match the imperative handler's full initialization — bitwise-identical creation events.

## Verification Layers

1. Bitwise-identical event-log invariant → event-log delta on justice/office/artifact goldens.
2. Office-claim closure invariant → focused unit/runtime test (per Rule 10): `press_force_claim`'s schema precondition rejects when the closure boundary the existing handler rejects (no support, existing office-holder, etc.). Cite the exact symbols checked in both the AI/belief layer and the authoritative law/action layer during reassessment.
3. Artifact-creation invariant → event-log delta: `post_bounty` creates an artifact entity with all expected components (issuer, terms, reward source, location, expiration); `claim_bounty` consumes an artifact with all expected proof checks.
4. Conformance-tests parity → `conformance_accuse`, `conformance_declare_support`, `conformance_press_force_claim` continue to pass.
5. Canonical state hash invariant → soak: identical hashes on the three soak scenarios.

## What to Change

### 1. Construct `EffectSchema` literals for 3 justice actions

- **accuse**: preconditions — `CoLocated { actor: accuser, target: accused }`, evidence-presence precondition. Steps — record creation (the accusation), `EmitEvent { tag: EventTag::Accuse }`.
- **fine**: preconditions — actor's authority precondition (RoleAuthority or office-claim), `CoLocated`. Steps — commodity transfer from target to authority's treasury, `EmitEvent { tag: EventTag::Fine }`.
- **exile**: preconditions — authority precondition, `CoLocated`. Steps — exile-marker mutation on target, `EmitEvent { tag: EventTag::Exile }`.

### 2. Construct `EffectSchema` literals for 5 office actions

- **bribe**: preconditions — `CoLocated`, payload's commodity-quantity available. Steps — `Transfer { source: actor, dest: target, commodity: bribe_commodity, quantity }`, belief-mutation step (target's loyalty/influence updated), `EmitEvent { tag: EventTag::Bribe }`.
- **threaten**: preconditions — `CoLocated`, optional weapon precondition. Steps — fear-mutation step, `EmitEvent { tag: EventTag::Threaten }`.
- **declare_support**: preconditions — actor's eligibility, target office's existence. Steps — support-claim record creation, `EmitEvent { tag: EventTag::SupportDeclared }`.
- **press_force_claim**: preconditions per Rule 10 — closure-boundary preconditions naming support-count threshold, visible-vacancy state, succession-rule applicability. Steps — office-holder mutation (the closure), `EmitEvent { tag: EventTag::ForceClaimPressed }`.
- **yield_force_claim**: preconditions — actor's existing claim. Steps — claim-withdrawal record mutation, `EmitEvent { tag: EventTag::ForceClaimYielded }`.

### 3. Construct `EffectSchema` literals for 4 artifact actions

- **post_bounty**: preconditions — actor's authority/treasury, place-suitability for posting. Steps — `CreateEntity { kind: Bounty, place, components: { issuer, terms, reward source, proof requirements, expiration } }`, `EmitEvent { tag: EventTag::BountyPosted }`.
- **post_notice**: analogous to post_bounty for notice artifacts.
- **claim_bounty**: preconditions — bounty-existence, proof-acceptance, `CoLocated` with bounty board. Steps — bounty-state mutation (claimed), reward transfer from issuer's treasury to claimant, `EmitEvent { tag: EventTag::BountyClaimed }`.
- **withdraw_bounty**: preconditions — issuer-identity, bounty-existence. Steps — bounty-state mutation (withdrawn), reward refund, `EmitEvent { tag: EventTag::BountyWithdrawn }`.

### 4. Replace commit handler bodies with `apply_effects` delegation

Each `commit_*` handler in the 3 files shrinks to the standard delegation. Remove imperative bodies.

### 5. Add new `EffectStep` variants if needed

Likely candidates surfaced by this ticket:
- `RoleAuthority` precondition variant (already in ticket 001's enum sketch).
- `MutateRelation { relation, source, target, value }` step for office-holder mutation, support-claim, etc. (if not yet covered by `CreateEntity` + entity-update). Confirm during reassessment.

If needed, add to `effect_schema.rs` and implement in both sink impls.

## Files to Touch

- `crates/worldwake-systems/src/justice_actions.rs` (modify — 3 schemas, 3 commit handler body replacements)
- `crates/worldwake-systems/src/office_actions.rs` (modify — 5 schemas, 5 commit handler body replacements)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify — 4 schemas, 4 commit handler body replacements)
- `crates/worldwake-sim/src/effect_schema.rs` (modify if `EffectStep` needs `MutateRelation`, `RoleAuthority` precondition refinement, or other variants)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` and `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (modify if new sink methods are added)

## Out of Scope

- Migrating non-justice/office/artifact actions (tickets 003–008).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Changing political-claim closure semantics, bounty lifecycle (S140 territory), or office-holder rules.
- Conformance test rewrite (ticket 010).

## Acceptance Criteria

### Tests That Must Pass

1. All justice/office/artifact-touching goldens produce bitwise-identical event logs.
2. Conformance tests `conformance_accuse`, `conformance_declare_support`, `conformance_press_force_claim` continue to pass.
3. `cargo test -p worldwake-systems justice office artifact` — existing inline tests pass with the schema-driven path.
4. `cargo test -p worldwake-ai golden_survival` — soak goldens produce identical canonical state hashes.
5. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `press_force_claim`'s schema preconditions reject the same scenarios the imperative handler rejects (closure-boundary preservation per Rule 10).
2. `post_bounty`/`post_notice` create artifact entities with the same component set (issuer, terms, reward source, proof requirements, expiration) as the imperative handler creates today.
3. Bitwise-identical canonical state hash on the three soak scenarios.
4. Bounty-claim reward transfer uses the same source-treasury and amount as today.

## Test Plan

### New/Modified Tests

1. Per-file `#[cfg(test)]` blocks — modify existing tests to exercise schema-driven path; add focused tests covering closure-boundary precondition rejection for `press_force_claim` and proof-failure rejection for `claim_bounty`.
2. Existing goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems justice office artifact`
2. `cargo test -p worldwake-ai conformance_accuse conformance_declare_support conformance_press_force_claim`
3. `cargo test -p worldwake-ai golden_office golden_bounty`
4. `cargo test -p worldwake-ai golden_survival`
5. `./scripts/verify.sh`
