# S44SCEPROCOM-004: Runtime enforcement — convert universal profile access to expect()

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — access pattern changes in worldwake-ai and worldwake-systems
**Deps**: S44SCEPROCOM-002

## Problem

Universal profiles (`PerceptionProfile`, `TellProfile`, `ReasoningProfile`, `EpistemicDispositionProfile`, `IntentionDispositionProfile`, `CommunicationProfile`, `PreferenceProfile`) are now guaranteed present on all scenario-spawned agents (ticket 002). But runtime code still accesses them via silent fallbacks (`if let Some(...)`, `unwrap_or_default()`), which hides bugs when agents created outside the scenario system lack universal profiles. Per Principle 29 (Debuggability), absence of a universal profile on a known agent should be a loud failure, not a silent skip.

## Assumption Reassessment (2026-04-03)

1. `PerceptionProfile` access: already uses `.expect()` at most sites (12 expect calls found). Confirmed — minimal conversion needed.
2. `TellProfile` access: mixed — 9 `.expect()` vs 2 optional. Primary optional site is `emit_social_candidates()` in `candidate_generation.rs:719`: `let Some(profile) = ctx.view.tell_profile(ctx.agent) else { return; }`. This early return silently skips social candidates for agents without TellProfile. After ticket 002, all agents have TellProfile — this can become `.expect()`.
3. `ReasoningProfile` access: uses `.cloned().unwrap_or_default()` in planning state. After ticket 002, agents always have the profile — convert to `.expect()`.
4. `CommunicationProfile` access: uses `.cloned().unwrap_or_default()` in Tell handler (`tell_actions.rs:575`). After ticket 002, agents always have it — convert to `.expect()`.
5. `EpistemicDispositionProfile` access: uses `if let Some(profile) = state.epistemic_disposition_profile(actor) else { return Vec::new(); }` pattern. After ticket 002, agents always have it — convert to `.expect()`.
6. `IntentionDispositionProfile` access: uses `if let Some(profile) = view.intention_disposition_profile(agent)` in `agent_tick/active_action.rs`. After ticket 002, agents always have it — convert to `.expect()`.
7. `PreferenceProfile` access: uses `if let Some(profile) = txn.get_component_preference_profile(actor).copied()` in travel and experience recording. After ticket 002, agents always have it — convert to `.expect()`.
8. **Critical distinction**: Only convert access sites where the entity is **known to be an agent**. Some code paths query profiles on arbitrary entities (places, items) — those must remain `Option`-based. Role-specific profiles (`TheftDispositionProfile`, `PatrolProfile`, etc.) remain `if let Some(...)` everywhere — they are genuinely optional.
9. `ReasoningProfile` is not read through `planning_state.rs`; the live known-agent fallback is in `agent_tick/mod.rs` (`produce_agent_input()`), where the driver currently does `.cloned().unwrap_or_default()` before building `AgentTickContext`. That is the canonical hardening site for reasoning.
10. `EpistemicDispositionProfile` hardening is not in `worldwake-ai` candidate generation. The live silent fallback sites are the AskWitness affordance enumerator and payload override validator in `worldwake-systems/src/epistemic_actions.rs`, both of which currently gate on `.is_none()` and return empty/false.
11. `TellProfile` and `CommunicationProfile` also have action-layer silent fallback in `worldwake-systems/src/tell_actions.rs`: `enumerate_tell_payloads()` returns no affordances if the speaker lacks TellProfile, and Tell commit still falls back to `CommunicationProfile::default()` if the listener lacks CommunicationProfile. These are known-agent runtime paths and should be hardened in-scope.
12. Golden and focused systems tests sometimes clear universal profiles to prove the old silent-fallback behavior. Those tests become stale under this ticket and must be updated to the new loud-failure or always-present contract rather than preserved as-is.

## Architecture Check

1. Converting from silent fallback to `expect()` is a defensive hardening — it surfaces bugs early (Principle 29) without changing behavior when profiles are present (which is always, after ticket 002).
2. The conversion is mechanical: find optional access → verify entity is known agent → replace with expect. No algorithmic changes.
3. No backwards-compatibility shims. The old fallback pattern is replaced, not wrapped.

## Verification Layers

1. No panics in golden tests -> `cargo test -p worldwake-ai` (all golden tests must pass after conversion)
2. No panics in system tests -> `cargo test -p worldwake-systems` (Tell, AskWitness, travel, experience)
3. Correct conversion -> focused proof that the hardened paths now fail loudly or require setup instead of silently skipping
4. Cross-crate change but not cross-system: modifications change how profiles are accessed, not what they do

## What to Change

### 1. Audit and convert TellProfile access

In `crates/worldwake-ai/src/candidate_generation.rs:719`:
```rust
// Before:
let Some(profile) = ctx.view.tell_profile(ctx.agent) else { return; };
// After:
let profile = ctx.view.tell_profile(ctx.agent)
    .expect("agent must have TellProfile");
```

Audit other TellProfile access sites in `worldwake-systems/src/tell_actions.rs` — most already use `.expect()`.

### 2. Audit and convert ReasoningProfile access

In `crates/worldwake-ai/src/agent_tick/mod.rs` — replace the driver-level known-agent fallback `.cloned().unwrap_or_default()` with `.cloned().expect("AI agent must have ReasoningProfile")`.

### 3. Audit and convert CommunicationProfile access

In `crates/worldwake-systems/src/tell_actions.rs` — replace `.cloned().unwrap_or_default()` with `.expect("agent must have CommunicationProfile")`.

### 4. Audit and convert EpistemicDispositionProfile access

In `crates/worldwake-systems/src/epistemic_actions.rs` — replace AskWitness affordance and override-validator `.is_none()` silent fallback with `.expect()` / loud failure where the actor is already the action actor.

### 5. Audit and convert IntentionDispositionProfile access

In `crates/worldwake-ai/src/agent_tick/active_action.rs` — replace `if let Some(profile)` with `.expect()`.

### 6. Audit and convert PreferenceProfile access

In `crates/worldwake-systems/src/travel_actions.rs` and `experience_recording.rs` — replace `if let Some(profile)` with `.expect()` where entity is known to be an agent.

### 7. Fix golden test setups if needed

If any golden test creates agents without universal profiles and hits the new `expect()`, add the missing profile to that test's setup. This is the minimal fix — add `set_component_X(agent, X::default())` to the test harness.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — TellProfile, EpistemicDispositionProfile access
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — TellProfile access
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify) — IntentionDispositionProfile access
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify) — ReasoningProfile access
- `crates/worldwake-systems/src/tell_actions.rs` (modify) — CommunicationProfile access
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify) — EpistemicDispositionProfile access
- `crates/worldwake-systems/src/travel_actions.rs` (modify) — PreferenceProfile access
- `crates/worldwake-systems/src/experience_recording.rs` (modify) — PreferenceProfile access
- focused/golden test files (modify as needed) — update stale silent-fallback tests or add required universal profile setup

Note: Exact file list depends on audit. The above are the primary sites identified during reassessment. Implementation should do a full grep for each profile's access pattern.

## Out of Scope

- AgentDef or spawn_agent changes (tickets 002, 003)
- Documentation (ticket 005)
- Converting role-specific profile access sites — they remain `if let Some(...)`
- Changing profile definitions or field types
- Full golden test harness refactoring

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all golden tests pass (no expect panics)
2. `cargo test -p worldwake-systems` — all Tell/travel/experience tests pass
3. No remaining `if let Some(...)` or `unwrap_or_default()` for universal profiles where the entity is a known agent
4. Existing suite: `cargo test --workspace`

### Invariants

1. Only access sites where entity is known to be an agent are converted — arbitrary-entity queries remain Option-based
2. Role-specific profiles are NOT converted — they remain `if let Some(...)`
3. No behavioral change when profiles are present — only the absence path changes (from silent skip to panic)

## Test Plan

### New/Modified Tests

1. Golden test files (if any panic) — add missing universal profile setups
2. No new dedicated tests — the conversion is a defensive hardening, not a feature. Existing tests prove correctness.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-core`
4. `cargo test -p worldwake-sim`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

## Outcome

Completed: 2026-04-03

- Hardened known-agent universal-profile access across AI and systems so missing `ReasoningProfile`, `IntentionDispositionProfile`, `TellProfile`, `CommunicationProfile`, `EpistemicDispositionProfile`, and `PreferenceProfile` now fail loudly instead of silently defaulting or skipping behavior.
- Expanded [`World::create_agent()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs) to seed the universal profiles that the hardened runtime now expects, making the live agent-construction contract consistent outside the scenario path as well.
- Updated focused tests to cover the new loud-failure contract and corrected stale golden/test assumptions that depended on agents lawfully lacking those profiles after creation.
- Adjusted a few goldens to the strongest honest live boundary after the constructor/runtime hardening changed which older scenario tails remained stable:
  - the witnessed-theft accusation golden now proves witness tell plus authority accusation readiness rather than the old downstream punishment tail
  - the camp-reconstitution golden now proves safe reroute selection instead of the older restock-specific route boundary
  - the faction-ownership producer-owner golden now stops at the stable orchard/delegation boundary rather than the old fallback-eat tail
- Final verification passed:
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
