# S164BELVIEKIN-003: Gate bandit faction-policy accessors

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `bandit_flee_wound_threshold` / `bandit_camp_establishment_ticks` in the per-agent belief view (sim)
**Deps**: None

## Problem

Before this ticket, `bandit_flee_wound_threshold` and `bandit_camp_establishment_ticks`
(`per_agent_belief_view.rs:611-621`) read
`world.get_component_bandit_faction_policy(faction)` for **any** faction with **no
accessibility gate**. The planner-visible call sites passed
`bandit_factions_of(actor)` with `actor == self.agent` (planning_snapshot.rs:701-716,
pressure.rs:77-79, planning_state.rs:1255-1260) — the actor's own/believed factions,
filtered through the self/belief-gated `factions_of` (`:1571`) — so those reads were
lawful self-state. The old accessor signature still invited a later caller to pass an
arbitrary faction and silently leak that faction's hidden behavioral policy. This
ticket closed the footgun without changing lawful call-site behavior.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Before this ticket, the two accessors
   (`crates/worldwake-sim/src/per_agent_belief_view.rs:611-621`) mapped
   `world.get_component_bandit_faction_policy(faction)` with no gate.
   `bandit_factions_of` (`:1592-1601`) filters `factions_of` (`:1571`) — which returns
   world factions only for `self.agent`, else institutional-belief membership — to
   bandit factions. The correct gate is membership of the **observing agent**:
   `self.bandit_factions_of(self.agent).contains(&faction)`.
2. The existing inline test was renamed from
   `bandit_policy_entity_methods_read_from_authoritative_faction_policy` to
   `bandit_policy_entity_methods_are_gated_to_own_bandit_factions` and now asserts
   the gated contract — return the policy for the agent's own bandit faction, `None`
   for an arbitrary faction. This was not adapting a test to a bug: the prior behavior
   was the latent leak being removed.
3. Boundary under audit: belief-view accessor source class. After the gate the source
   class is self/belief-backed faction membership (own/believed factions only).
4. Call-site behavior remained unchanged: all live callers pass
   `bandit_factions_of(actor)` with `actor == self.agent`, so every faction they
   iterate is already in the gate set. No planner candidate, ranking, or snapshot
   value changed for lawful callers (FND-14B inputs unchanged).

## Architecture Check

1. Gating on the observing agent's own/believed bandit-faction membership matches the
   real-world fact a bandit can know — their own gang's behavioral policy — and denies
   knowledge of an arbitrary faction's hidden parameters. This is the same self/belief
   path the codebase already enforces for `factions_of`, so the gate reuses existing
   lawful infrastructure rather than inventing a new one.
2. No backward-compat shim: the ungated path is removed, not aliased.

## Verified Layers

1. Own-faction policy still readable → focused unit test: agent in a bandit faction
   reads its `flee_wound_threshold` / `establishment_duration_ticks`.
2. Arbitrary-faction policy denied → focused unit test: a faction the agent is not a
   member of returns `None` for both accessors.
3. Single-layer ticket (belief-view accessor): no action-trace/event-log surface; the
   negative illegal-path is covered in the ticket 005 systemic-validation note as a
   footgun-closure assertion, but the accessor contract itself is proven at the
   focused-unit layer here.

## Landed Changes

### 1. Gated both accessors on the observing agent's bandit-faction membership

In `bandit_flee_wound_threshold` and `bandit_camp_establishment_ticks`, the view now
returns `None` unless `self.bandit_factions_of(self.agent).contains(&faction)`. When
the gate passes, it reads `world.get_component_bandit_faction_policy(faction)` as
before.

### 2. Updated the existing test to the gated contract

The focused test is now
`bandit_policy_entity_methods_are_gated_to_own_bandit_factions`. It asserts that the
agent's own bandit faction returns the policy and a non-member bandit faction returns
`None`.

## Landed Files

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — `:611-621` + test `:6178`)

## Out of Scope

- `entity_kind` and the last-seen carrier (tickets 001/002).
- `facility_controller_at` (ticket 004).
- Any change to authoritative `BanditFactionPolicy`, `factions_of`, or
  `bandit_factions_of`.

## Acceptance Result

### Tests Passed

1. An agent that is a member of a bandit faction reads that faction's policy from both
   accessors.
2. Both accessors return `None` for a faction the observing agent is not a member of.
3. Existing planner-snapshot / pressure paths are unaffected (lawful callers pass the
   agent's own factions).
4. Existing suites passed: `cargo test -p worldwake-sim`, `cargo test -p worldwake-ai`,
   and `./scripts/verify.sh`.

### Invariants

1. The accessors never return a policy for a faction outside the observing agent's
   own/believed bandit-faction membership.
2. Lawful current behavior (own-faction reads at existing call sites) is preserved.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — update `:6178` for the gated
   contract; add a non-member-returns-`None` case.

### Commands Run

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-22.

- Added the faction-membership source gate to both bandit faction-policy accessors on
  `PerAgentBeliefView`.
- Preserved own-faction policy reads for the observing agent.
- Added the non-member negative case to the existing inline belief-view test.

## Verification Result

- Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::bandit_policy_entity_methods_are_gated_to_own_bandit_factions -- --exact`
- Passed `cargo test -p worldwake-sim per_agent_belief_view`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test -p worldwake-ai`
- Passed `./scripts/verify.sh`
