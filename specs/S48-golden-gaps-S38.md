**Status**: Proposed

# S48: Golden Gap — Learned Source Reliability Redirects Later Acquisition

## Summary

Post-implementation golden gap analysis for S38 (Learned Route and Source Preferences). The route-memory side is now covered by Scenarios 91-93, but the suite still lacks an end-to-end golden proving the other half of the spec: a source-intrinsic acquisition failure leaves durable `SourceReliability` aftermath and that learned failure later changes source selection when a lawful alternative exists.

## Scenario: Failed Local Source Redirects Later Acquisition To A Remote Sibling

An agent first attempts to acquire food from a known local orchard source and hits an authoritative source-intrinsic `StartFailed` because the source is depleted. That failure records `SourceReliability` on the local orchard workstation. On the next planning tick, with a remote sibling orchard still lawful and reachable, the agent prefers the remote source instead of retrying the failed local source.

### Description

1. Agent is critically hungry and knows two lawful apple sources for the same commodity:
   - a local orchard source at the current place
   - a remote sibling orchard source at another place
2. The local source is depleted, so the authoritative harvest start fails for source-intrinsic reasons.
3. The start failure records `SourceReliability { failed_attempts += 1 }` for the local source.
4. On the next planning tick, the agent still needs apples and still knows the remote sibling source.
5. AI ranking discounts the failed local source and the agent chooses the remote sibling path instead of retrying the local one.

### GoalKinds Exercised

- `AcquireCommodity { commodity: Apple, purpose: SelfConsume }`

### ActionDomains Exercised

- `Production` — harvest source discovery and authoritative source failure
- `Travel` — remote sibling fallback path
- `Needs` — downstream consumption motive still drives the retry

### Systems Exercised

- **Production (S38 source recording)**: harvest `StartFailed` records source-intrinsic failure
- **AI ranking (S38 learned source preference)**: `SourceReliability` discounts the failed local source on the next planning pass
- **Travel / planning**: fallback route to the remote sibling source is selected instead of the failed local retry

### Setup Requirements

- One hungry agent with `PreferenceProfile { source_trust_weight > 0, .. }`
- Two known orchard sources for apples:
  - one local depleted workstation
  - one remote reachable workstation with available apples
- Belief/perception setup sufficient for the agent to lawfully know both sources
- Topology where retrying the local source would still be cheaper absent learned failure, so the reroute is attributable to `SourceReliability`

### What Emergence It Demonstrates

This proves that S38 source learning is not just a focused ranking helper. A concrete failed acquisition attempt becomes durable personal memory, and that memory changes later source choice across production, AI ranking, and travel planning without any special-case “avoid depleted orchard” branch.

### Foundation Principle Alignment

- **Principle 3** (Concrete State Over Abstract Scores): the memory is concrete `failed_attempts` on a specific `SourceKey`
- **Principle 10** (Outcomes Are Granular and Leave Aftermath): the authoritative `StartFailed` is preserved as durable new state
- **Principle 12** (System Decoupling): production records failure aftermath; AI later consumes that state through the shared belief-facing ranking path
- **Principle 15** (Knowledge Acquired Locally): the agent learns only from its own failed attempt

### Why It Is Not a Duplicate

- **Scenarios 91-93** only prove learned route memory, not source reliability.
- **Scenario 1c** proves opportunity exhaustion causing sibling-source fallthrough, but that path is seeded frontier state, not learned `SourceReliability` from real acquisition failure.
- **Scenario 3 / 3b / 74** prove fallback after contention or start failure, but they do not prove that a durable per-agent source memory changes later ranking.
- Focused tests in `production_actions.rs`, `trade_actions.rs`, and `ranking.rs` prove the lower-layer contracts, but there is still no golden demonstrating the full cross-system chain.

## Ticket Breakdown

### S48GOLGAP-001: Golden source-reliability reroute after local harvest failure

- Add a new golden scenario to prove:
  - local harvest `StartFailed` records `SourceReliability`
  - the next planning pass chooses the remote sibling source
  - the final chain still leads to lawful acquisition/consumption
- Add a deterministic replay companion
- Prefer decision-trace assertions for why the local source is no longer selected, plus authoritative state assertions for `SourceReliability`

**Files**: `crates/worldwake-ai/tests/golden_production.rs` or a new focused golden file if setup clarity is better there
**Effort**: Medium

## Tests

- [ ] learned local harvest failure redirects later acquisition to remote sibling source
- [ ] deterministic replay companion

## Acceptance Criteria

1. A source-intrinsic harvest `StartFailed` records `SourceReliability` for the failed local source
2. The next planning pass prefers the remote sibling source rather than retrying the failed local source
3. The scenario proves the reroute through decision trace or equivalent planner-facing evidence, not only by eventual world state
4. Conservation invariants continue to hold
5. Deterministic replay reproduces the same world and event-log hashes
