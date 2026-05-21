# S164BELVIEKIN-003: Gate bandit faction-policy accessors

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `bandit_flee_wound_threshold` / `bandit_camp_establishment_ticks` in the per-agent belief view (sim)
**Deps**: None

## Problem

`bandit_flee_wound_threshold` and `bandit_camp_establishment_ticks`
(`per_agent_belief_view.rs:611-621`) read
`world.get_component_bandit_faction_policy(faction)` for **any** faction with **no
accessibility gate**. Every planner-visible call site today passes
`bandit_factions_of(actor)` with `actor == self.agent` (planning_snapshot.rs:701-716,
pressure.rs:77-79, planning_state.rs:1255-1260) — the actor's own/believed factions,
filtered through the self/belief-gated `factions_of` (`:1571`) — so the reads are
lawful self-state today. But the accessor signature invites a future caller to pass
an arbitrary faction and silently leak that faction's hidden behavioral policy. This
ticket closes the footgun without changing lawful current behavior.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The two accessors (`crates/worldwake-sim/src/per_agent_belief_view.rs:611-621`)
   currently map `world.get_component_bandit_faction_policy(faction)` with no gate.
   `bandit_factions_of` (`:1592-1601`) filters `factions_of` (`:1571`) — which returns
   world factions only for `self.agent`, else institutional-belief membership — to
   bandit factions. The correct gate is membership of the **observing agent**:
   `self.bandit_factions_of(self.agent).contains(&faction)`.
2. The existing inline test
   `bandit_policy_entity_methods_read_from_authoritative_faction_policy`
   (`per_agent_belief_view.rs:6178`) asserts the current ungated read. Closing the
   footgun is an intentional behavior change (a footgun, not a bug being worked
   around), so this test must be updated to assert the gated contract — return the
   policy for the agent's own bandit faction, `None` for an arbitrary faction. This is
   not adapting a test to a bug: the prior behavior was the latent leak being removed.
3. Boundary under audit: belief-view accessor source class. After the gate the source
   class is self/belief-backed faction membership (own/believed factions only).
4. Call-site behavior is unchanged: all live callers pass `bandit_factions_of(actor)`
   with `actor == self.agent`, so every faction they iterate is already in the gate
   set. No planner candidate, ranking, or snapshot value changes (FND-14B inputs
   unchanged for lawful callers).

## Architecture Check

1. Gating on the observing agent's own/believed bandit-faction membership matches the
   real-world fact a bandit can know — their own gang's behavioral policy — and denies
   knowledge of an arbitrary faction's hidden parameters. This is the same self/belief
   path the codebase already enforces for `factions_of`, so the gate reuses existing
   lawful infrastructure rather than inventing a new one.
2. No backward-compat shim: the ungated path is removed, not aliased.

## Verification Layers

1. Own-faction policy still readable → focused unit test: agent in a bandit faction
   reads its `flee_wound_threshold` / `establishment_duration_ticks`.
2. Arbitrary-faction policy denied → focused unit test: a faction the agent is not a
   member of returns `None` for both accessors.
3. Single-layer ticket (belief-view accessor): no action-trace/event-log surface; the
   negative illegal-path is covered in the ticket 005 systemic-validation note as a
   footgun-closure assertion, but the accessor contract itself is proven at the
   focused-unit layer here.

## What to Change

### 1. Gate both accessors on the observing agent's bandit-faction membership

In `bandit_flee_wound_threshold` and `bandit_camp_establishment_ticks` (`:611-621`),
return `None` unless `self.bandit_factions_of(self.agent).contains(&faction)`. When
the gate passes, read `world.get_component_bandit_faction_policy(faction)` as today.

### 2. Update the existing test to the gated contract

Rename/extend `bandit_policy_entity_methods_read_from_authoritative_faction_policy`
(`:6178`) to assert: own-faction read returns the policy; non-member faction returns
`None`.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — `:611-621` + test `:6178`)

## Out of Scope

- `entity_kind` and the last-seen carrier (tickets 001/002).
- `facility_controller_at` (ticket 004).
- Any change to authoritative `BanditFactionPolicy`, `factions_of`, or
  `bandit_factions_of`.

## Acceptance Criteria

### Tests That Must Pass

1. An agent that is a member of a bandit faction reads that faction's policy from both
   accessors.
2. Both accessors return `None` for a faction the observing agent is not a member of.
3. Existing planner-snapshot / pressure paths are unaffected (lawful callers pass the
   agent's own factions).
4. Existing suite: `cargo test -p worldwake-sim`.

### Invariants

1. The accessors never return a policy for a faction outside the observing agent's
   own/believed bandit-faction membership.
2. Lawful current behavior (own-faction reads at existing call sites) is preserved.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — update `:6178` for the gated
   contract; add a non-member-returns-`None` case.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-sim -p worldwake-ai`
3. `./scripts/verify.sh`
