# S152COGARCSEE-005: Spawn-time archetype resolution

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — scenario spawn resolution (`worldwake-cli`)
**Deps**: archive/tickets/S152COGARCSEE-001.md, archive/tickets/S152COGARCSEE-002.md, S152COGARCSEE-003, S152COGARCSEE-004

## Problem

This ticket lands the resolver that turns an archetype assignment policy (or per-agent override) into concrete profile values at agent spawn: draw the archetype deterministically, look up the template, apply deltas to the existing universal profiles, clamp, scale the backoff fields, insert disabled methods, set `CognitiveArchetypeComponent`, and emit `PersonalityAssigned` with the resolved-profile hash. This is the only emission site for the new event.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `spawn_agent` (`crates/worldwake-cli/src/scenario/mod.rs:590`) currently takes `(txn, recipes, agent_def, names, agent_locations)` — no RNG, no index — and applies universal profiles via `unwrap_or_default()` (e.g. `mod.rs:607+`). The agent-spawning loop is `for agent_def in &def.agents` (`mod.rs:313`), with no `enumerate`. The simulation `DeterministicRng` is constructed at `mod.rs:1493` (`DeterministicRng::new(Seed(seed_from_u64(def.seed)))`) — *after* the spawn loop. This ticket must construct a dedicated archetype-assignment RNG before the loop and thread a stable `agent_index` (via `.enumerate()`).
2. Existing inline spawn tests that exercise universal-profile application and will need extension/assertion updates: `test_spawn_agents_receive_default_universal_profiles` (`mod.rs:3560`), `test_spawn_agent_applies_authored_schema_context_profile` (`mod.rs:3688`), `test_spawn_agent_applies_authored_s151_profiles` (`mod.rs:3714`).
3. Mixed-layer boundary under audit: scenario-load resolution writes authoritative profile components + the `CognitiveArchetypeComponent` (ticket 002) and emits the `PersonalityAssigned` event (ticket 003). The resolved-profile hash uses `canonical::hash_serializable` over the archetype-affected profiles (`canonical.rs:51`).
4. (Cumulative arithmetic) The resolver applies signed deltas to `Permille` fields and clamps to [0,1000] via `Permille::new(...).unwrap_or(...)`-style clamping; integer fields (`max_plan_depth: u8`, `observation_budget: u8`, `ask_memory_retention_ticks: u32`) clamp to non-negative ranges. `backoff_ticks_scale: BackoffScalePermille` multiplies each of the eleven `*_backoff_ticks`/`*_block_ticks` fields: `scaled = (orig as u64 * scale.value() as u64 / 1000) as u32`. Per-agent sub-seed is derived by hashing `def.seed` with `agent_index` (e.g. feed both into the `seed_from_u64`-style derivation), not naked addition (FND-2).
5. (Heuristic/precedence) Explicit author overrides take precedence over archetype deltas: the delta applies to the scenario-authored base value when present, otherwise the profile default. `AgentDef.archetype` (Explicit) bypasses the policy draw. Document this two-layer precedence (per-field override > archetype delta; explicit archetype > policy draw) in What to Change.
6. (Scenario isolation) Determinism requires a stable per-agent index independent of `BTreeMap` iteration; use the `def.agents` slice position as `agent_index`.

## Architecture Check

1. A dedicated assignment RNG constructed before the spawn loop, keyed by `def.seed` + stable index, gives reproducible assignment without entangling the simulation RNG ordering. Applying deltas to existing fields means the resolved values flow through the unchanged consuming systems — no new runtime read path.
2. No backwards-compatibility shim: the default policy is applied when none is authored (FND-28), not a legacy "all-identical" path.

## Verification Layers

1. Same scenario + seed → identical archetype per agent → identical resolved profile values -> golden/focused determinism test (authoritative world state). (Full golden in ticket 008.)
2. Resolved deltas land on the correct existing profile fields with correct clamping -> focused spawn unit test reading the resolved `CognitiveProfile`/`RiskWeightProfile`/etc.
3. `PersonalityAssigned` emitted exactly once per agent with `source` and hash -> event-log delta assertion in a focused spawn test.
4. Explicit `AgentDef.archetype` override pins the archetype regardless of policy -> focused spawn unit test.

## What to Change

### 1. Thread assignment RNG + index into spawn

Construct an archetype-assignment `DeterministicRng` from `def.seed` before the agent loop; change the loop to `for (agent_index, agent_def) in def.agents.iter().enumerate()`; pass `agent_index` (and the assignment seed/RNG) into `spawn_agent`.

### 2. Resolve the archetype

If `agent_def.archetype` is `Some`, use it (`source = Explicit`). Else draw from the effective `ArchetypeAssignmentPolicy` (`def.archetype_assignment_policy.clone().unwrap_or_default()`) using a per-agent sub-seed derived from `def.seed` + `agent_index` (`source = Policy(policy)`).

### 3. Apply the template

Look up `template_for(archetype)`. Apply each delta to the authored-or-default base of the target profile; clamp; apply `backoff_ticks_scale` to all backoff/block tick fields; insert `method_disable` ids into `AgentSchemaContextProfile.disabled_methods`. Set the resolved profile components.

### 4. Set component and emit event

Set `CognitiveArchetypeComponent { archetype }`. Compute `resolved_profile_hash` via `hash_serializable` over the resolved archetype-affected profiles. Emit `PersonalityAssigned` carrying `PersonalityAssignedPayload { agent, archetype, seed, source, resolved_profile_hash }`.

## Files to Touch

- `crates/worldwake-cli/src/scenario/mod.rs` (modify — spawn loop, `spawn_agent` signature/body, resolver, inline tests at `:3560`/`:3688`/`:3714`)

## Out of Scope

- The `EventTag` variant / `EventPayload` field (ticket 003) and component registration (ticket 002).
- Observer rendering (006) and diagnostics (007).
- Any change to consuming systems — they read the resolved existing fields unchanged.

## Acceptance Criteria

### Tests That Must Pass

1. Two agents under `DefaultUniformFive` with the same seed get identical archetypes and identical resolved profile values across runs.
2. A Cautious agent's resolved backoff ticks exceed a Bold agent's (backoff scale applied); a Cautious agent's resolved `RiskWeightProfile.threat_aversion` exceeds default.
3. `AgentDef.archetype: Some(Greedy)` yields archetype Greedy with `source = Explicit`, ignoring the policy.
4. Exactly one `PersonalityAssigned` event per agent at spawn.
5. Per-field author overrides take precedence over archetype deltas.
6. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Assignment is deterministic and replayable from `def.seed` + agent index (FND-2); no naked-addition seed derivation.
2. Resolved `Permille` fields stay within [0,1000]; integer fields stay non-negative (clamped).
3. Archetype changes no action set or affordance — only resolved profile values (FND-19).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/mod.rs` (`#[cfg(test)]`) — determinism, delta application + clamp, explicit override, single-emission, override precedence.
2. Extend `test_spawn_agents_receive_default_universal_profiles` (`:3560`) to assert default-policy archetype resolution.

### Commands

1. `cargo test -p worldwake-cli scenario`
2. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
3. `./scripts/verify.sh`
