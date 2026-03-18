**Status**: PENDING

# Indefinite Action Re-Evaluation

## Summary

Convert the `defend` action from indefinite duration to finite duration with agent-driven renewal. Currently, `defend` runs forever once started — no tick limit, no interrupt trigger. If the agent's motivation for defending changes (hostile target dies, leaves, or danger pressure drops), the agent remains locked in the defend stance because the interrupt system only fires for *higher-priority* goal switches, and the agent's own wound pressure keeps danger elevated even when no external threat exists.

This deadlock violates Principle 19 (Intentions Are Revisable Commitments): the agent cannot abandon or revise a commitment whose assumptions have been broken by new information.

## Discovered Via

Golden E2E emergent tests (S07 care interaction coverage). A pre-wounded fighter near a hostile target selected `ReduceDanger → defend` as its highest-priority goal. Defend ran indefinitely — the fighter never attacked, never looted, never self-healed. The agent was permanently deadlocked in a defensive stance against a target it could have killed.

## Foundation Alignment

- **Principle 19** (Intentions Are Revisable Commitments): "Agents must monitor the assumptions beneath an active intention and suspend, revise, or replace that intention when new local evidence invalidates it." An agent defending against a dead target is not monitoring its assumptions.
- **Principle 1** (Maximal Emergence): Emergent behavior requires agents to respond to *current* state. An agent stuck in a stale action cannot participate in emergence.
- **Principle 8** (Every Action Has Preconditions, Duration, Cost, and Occupancy): The defend action currently violates the duration requirement — it has no defined endpoint. Every action should have a finite duration or explicit renewal mechanism.
- **Principle 18** (Resource-Bounded Practical Reasoning): Indefinite actions bypass the decision cycle entirely. The agent never re-evaluates whether defending is still the best use of its time.

## Phase

Phase 3: Information & Politics (design fix, no phase dependency)

## Crates

- `worldwake-systems` (defend action definition and handler)
- `worldwake-ai` (interrupt evaluation, if modified)

## Dependencies

None. All prerequisite infrastructure exists.

## Design Options

### Option A: Finite Defend with Renewal (Recommended)

Convert defend from an indefinite action to a finite-duration action (e.g., 8–12 ticks, configurable per combat profile). When the action completes, the agent re-enters the decision cycle and either re-selects defend (if danger persists) or switches to a different goal (if the threat resolved).

**Advantages**:
- No new infrastructure needed — finite actions already work
- Agent naturally re-evaluates every cycle
- Duration is a concrete parameter on `CombatProfile`, maintaining agent diversity (Principle 20)
- Consistent with how all other actions work

**Disadvantages**:
- Brief window at renewal boundary where agent is undefended — but this is realistic and creates emergence (attackers can exploit timing gaps)

**Implementation**:
1. Add `defend_stance_ticks: NonZeroU32` field to `CombatProfile` (default: `nz(10)`)
2. Change defend `ActionDef` duration from `Indefinite` to `Finite(defend_stance_ticks)`
3. Update `start_gate.rs` if any defend-specific start validation exists
4. Update all existing `CombatProfile::new()` call sites to include the new parameter

### Option B: Periodic Interrupt Re-Evaluation

Keep defend indefinite but add a periodic interrupt check every N ticks. The interrupt system would force a re-evaluation of whether the agent's current goal is still optimal.

**Advantages**:
- Minimal change to action framework
- Could apply to any future indefinite action

**Disadvantages**:
- Introduces a magic number (re-evaluation period N) with no physical grounding
- Violates Principle 10 (dampeners must be physical, not numeric clamps)
- Doesn't solve the root cause (indefinite actions bypassing the decision cycle)

### Option C: Event-Driven Interrupts

Trigger re-evaluation when specific state changes occur (entity death, entity departure, hostility removal).

**Advantages**:
- Most responsive — immediate re-evaluation on relevant events

**Disadvantages**:
- Requires event→affected-agent routing infrastructure that doesn't exist yet
- Complex to implement — must enumerate all relevant state changes
- Risk of over-triggering (every tick has state changes)

## Recommendation

**Option A (Finite Defend with Renewal)**. It is the simplest, most Foundation-aligned approach. The agent re-evaluates every cycle because the action naturally expires. The renewal boundary creates emergence opportunities (timing-sensitive combat). The duration is a concrete per-agent parameter (Principle 20).

## Deliverables

### 1. Add `defend_stance_ticks` to `CombatProfile`

New field controlling how many ticks a single defend action lasts before the agent must re-decide.

```rust
pub struct CombatProfile {
    // ... existing fields ...
    pub defend_stance_ticks: NonZeroU32, // NEW — default nz(10)
}
```

Update `CombatProfile::new()` to accept the new parameter. Update all existing call sites (golden test helpers, production code).

### 2. Change defend action duration from Indefinite to Finite

In the defend `ActionDef` construction (likely in `build_full_action_registries` or equivalent), set the duration to `Finite` using the agent's `defend_stance_ticks` from their `CombatProfile`. The `ActionDuration` should be resolved at action start time from the actor's profile.

### 3. Tests

**Unit test** (worldwake-systems): Defend action with `defend_stance_ticks: nz(5)` completes after 5 ticks.

**Golden test** (worldwake-ai): Agent defending against a hostile target that dies mid-defend naturally re-evaluates after defend expires and switches to a different goal (loot, self-care, or idle).

**Regression**: All existing combat golden tests must still pass. The `golden_reduce_danger_defensive_mitigation` test may need its tick budget adjusted if it relies on indefinite defend duration.

## Information-Path Analysis (FND-01 Section H)

Not applicable — this changes action duration semantics, not information flow.

## Stored State vs. Derived

- **Stored**: `defend_stance_ticks` field on `CombatProfile` (authoritative per-agent parameter)
- **Derived**: Nothing new. The defend action's remaining duration is already tracked by the scheduler's `ActionDuration` state machine.

## Positive-Feedback Analysis

No new feedback loops introduced. The finite defend actually *breaks* a potential deadlock loop (defend → danger persists → defend again → forever) by forcing periodic re-evaluation.
