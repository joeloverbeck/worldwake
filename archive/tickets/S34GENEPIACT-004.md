# S34GENEPIACT-004: ask_witness action handler, registration, and ask-memory architecture

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-core belief memory, worldwake-sim duration/belief-view plumbing, worldwake-systems ask_witness handler and registration
**Deps**: S34GENEPIACT-001 (core epistemic types), S34GENEPIACT-002 (payload types), S34GENEPIACT-003 (epistemic_actions.rs exists), [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md)

## Problem

Agents still cannot deliberately question a co-located witness about a belief subject. `AskWitnessPayload` already exists in `worldwake-sim`, but there is no live `ask_witness` action definition, no handler, no registry wiring, and no authoritative ask-memory contract for suppressing repeated witness queries.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the epistemic conversation contract across [crates/worldwake-systems/src/epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs), [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs), [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs), and [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs): action duration resolution, authoritative validation, belief transfer, and repeat-query suppression.
2. `AskWitnessPayload`, `ActionPayload::AskWitness`, typed accessors, and action-trace payload awareness already exist in [crates/worldwake-sim/src/action_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs) and [crates/worldwake-sim/src/action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs). This ticket must not re-add payload plumbing the codebase already has.
3. `VerificationDispositionProfile` and `ActionDomain::Epistemic` are already live. `epistemic_actions.rs` currently implements only `verify_belief`, and [crates/worldwake-systems/src/action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) registers only `verify_belief` on the epistemic side.
4. `DurationExpr::ActorVerificationDisposition` currently resolves only `VerificationDispositionProfile::verify_belief_duration_ticks` in [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs). The original ticket’s plan to use that same duration expression for `ask_witness` would silently ignore `witness_query_duration_ticks`. A separate duration expression is required for a clean contract.
5. `AgentBeliefStore` already has `told_beliefs` and `heard_beliefs`, but those lanes model content-sharing outcomes, not the act of asking a question. They cannot faithfully represent commodity-only queries or “asked and learned nothing” cases with the existing `TellMemoryKey` shape in [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs).
6. `RuntimeBeliefView` and `PerAgentBeliefView` expose tell-memory helpers (`tell_profile`, `told_belief_memory`, `recipient_knowledge_status`) but no asker-side memory surface for `ask_witness`. The original ticket’s affordance-level dedupe plan cannot be implemented honestly without either overloading tell memory or adding an ask-specific read surface.
7. The live Tell action does not reserve or occupy both participants. Its contract is actor-driven with same-place/alive commit revalidation via `TargetAtActorPlace(0)` and `TargetAlive(0)` in [crates/worldwake-systems/src/tell_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/tell_actions.rs). This ticket should mirror that current lawful runtime contract instead of claiming a new two-party occupancy system.
8. Current focused coverage already proves nearby patterns but no `ask_witness` behavior: `cargo test -p worldwake-systems -- --list` shows `epistemic_actions::tests::*verify_belief*` and many `tell_actions::tests::*`, but no `ask_witness` tests. `cargo test -p worldwake-sim -- --list` confirms the payload and duration infrastructure has unit-test coverage but no ask-specific duration resolver yet.
9. Mismatch + correction: the original ticket assumed `ask_witness` should reuse `ToldBeliefMemory` / `HeardBeliefMemory`. Reassessment shows that architecture is lossy for commodity-only and no-result queries. This ticket should introduce dedicated ask-memory keyed by the actual ask payload shape and use `VerificationDispositionProfile::ask_memory_retention_ticks` directly for ask dedupe.

## Architecture Check

1. Dedicated ask memory is cleaner than reusing tell memory because the canonical fact under suppression is “agent A asked witness B about topic T at tick N,” not “belief content X was told/heard.” That distinction matters for no-result queries, commodity-scoped queries, and future extensions.
2. A separate `ActorWitnessQueryDisposition` duration expression is cleaner than overloading `ActorVerificationDisposition`. It keeps the authoritative duration contract explicit and prevents `ask_witness` from accidentally inheriting `verify_belief` timing forever.
3. Matching Tell’s live same-place/alive revalidation model is cleaner than smuggling in ad hoc target reservations here. If the project later wants true two-party social occupancy, that should be a separate systemic ticket shared by Tell and AskWitness rather than a one-off epistemic hack.
4. No backwards-compatibility shims, no aliasing `ask_witness` onto Tell internals, and no “first matching entity” memory keys.

## Verification Layers

1. `ask_witness` action definition, duration source, and registry completeness -> focused `worldwake-systems` + `worldwake-sim` unit tests
2. Authoritative payload validation and affordance dedupe -> focused runtime/handler tests through `get_affordances()` and start validation
3. Successful ask transfers beliefs with `Report { from, chain_len: 1 }` provenance -> focused handler test plus authoritative belief-store assertions
4. Abort on target movement, death, or incapacitation -> focused action lifecycle tests with authoritative belief-store and ask-memory assertions
5. Ask-memory retention semantics use the ask-specific store and `VerificationDispositionProfile::ask_memory_retention_ticks` -> focused core/runtime test
6. Mixed-layer ticket, but the proof surface stays at focused unit/runtime tests. Planner and golden coverage remain in tickets 005-008.

## What to Change

### 1. Add dedicated ask-memory state

In `worldwake-core`, add ask-memory types and storage that model the act of asking directly:

- add `AskWitnessMemoryKey { counterparty, topic_entity, topic_commodity }`
- add `AskWitnessMemory { asked_tick }`
- store ask memories on `AgentBeliefStore`
- add helper(s) to record and query recent asks using `VerificationDispositionProfile::ask_memory_retention_ticks`

Do not reuse `TellMemoryKey`, `ToldBeliefMemory`, or `HeardBeliefMemory` for this ticket.

### 2. Add a dedicated witness-query duration expression

In `worldwake-sim`, add a new duration expression variant that resolves from `VerificationDispositionProfile::witness_query_duration_ticks` and cover it with the existing duration-expression tests.

### 3. Implement `ask_witness` in `epistemic_actions.rs`

Add:

- `register_ask_witness_action(defs, handlers)`
- `enumerate_ask_witness_payloads(...)`
- `validate_ask_witness_payload_authoritatively(...)`
- `start_ask_witness(...)`
- `tick_ask_witness(...)`
- `commit_ask_witness(...)`
- `abort_ask_witness(...)`

Handler contract:

- target is a same-place living agent
- payload must have at least one populated topic field
- commit transfers matching `known_entities` beliefs from target to actor
- transferred beliefs use `PerceptionSource::Report { from: target, chain_len: 1 }`
- if target has no relevant beliefs, commit still succeeds and records ask memory
- commit records actor-side ask memory keyed by the actual query topic
- no ask memory or belief transfer is recorded on abort

### 4. Wire runtime view + registry surfaces

- expose ask-memory lookups through `RuntimeBeliefView` / `PerAgentBeliefView`
- register/export `ask_witness` alongside `verify_belief`
- add `"ask_witness"` to the full action catalog test

## Files to Touch

- [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) (modify — add ask-memory storage/helpers)
- [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) (modify — add witness-query duration variant + tests)
- [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) (modify — add ask-memory runtime surface)
- [crates/worldwake-sim/src/per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) (modify — implement ask-memory runtime surface)
- [crates/worldwake-systems/src/epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs) (modify — add ask_witness handler + focused tests)
- [crates/worldwake-systems/src/action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) (modify — register action + catalog test)
- [crates/worldwake-systems/src/lib.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs) (modify — export registration helper)

## Out of Scope

- Planner op construction for `AskWitness` — ticket 005
- Candidate generation and ranking for `VerifyBelief` — tickets 006/007
- Golden E2E epistemic scenarios — ticket 008
- A generalized social-occupancy or conversation-reservation system shared across Tell and AskWitness
- Refactoring Tell memory into a broader generalized conversation-memory framework

## Acceptance Criteria

### Tests That Must Pass

1. `ask_witness` registers as an epistemic action with witness-query duration resolution
2. `ask_witness` transfers target beliefs with `Report { chain_len: 1 }` provenance
3. `ask_witness` commits successfully when target has no relevant beliefs and still records actor-side ask memory
4. `ask_witness` suppresses re-asking the same target/topic within `ask_memory_retention_ticks`
5. `ask_witness` rejects payloads where both `topic_entity` and `topic_commodity` are `None`
6. `ask_witness` aborts if target moves away before commit; no beliefs transferred and no ask memory recorded
7. `ask_witness` aborts if target dies before commit; no beliefs transferred and no ask memory recorded
8. `ask_witness` aborts if target becomes incapacitated before commit; no beliefs transferred and no ask memory recorded
9. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Epistemic ask actions update belief state and ask memory, not unrelated authoritative world state
2. Ask dedupe is keyed by the concrete ask topic, not by overloaded tell-content memory
3. `witness_query_duration_ticks` and `verify_belief_duration_ticks` remain distinct authoritative contracts
4. Determinism is preserved with ordered maps/sets only

## Test Plan

### New/Modified Tests

1. [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) — add ask-memory retention coverage so the dedupe contract is proven at the strongest storage layer
2. [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) — add witness-query duration resolution coverage so `ask_witness` cannot accidentally inherit `verify_belief` timing
3. [crates/worldwake-systems/src/epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs) — add focused ask_witness registration, validation, commit, dedupe, and abort-path tests
4. [crates/worldwake-systems/src/action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) — extend the required action catalog with `ask_witness`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-systems`
4. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
5. `cargo build --workspace`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - added dedicated actor-side ask memory in `AgentBeliefStore` via `AskWitnessMemoryKey` / `AskWitnessMemory`
  - added `DurationExpr::ActorWitnessQueryDisposition` so `ask_witness` resolves `witness_query_duration_ticks` directly
  - implemented and registered `ask_witness` in `epistemic_actions.rs`
  - exposed ask-memory lookups through runtime belief-view plumbing
  - updated the planner duration contract to account for the new duration expression
- Deviations from original plan:
  - did not reuse `ToldBeliefMemory` / `HeardBeliefMemory`; reassessment showed that design was lossy for commodity-only and no-result asks
  - did not introduce a new two-party occupancy/reservation model; `ask_witness` matches Tell’s live same-place/alive revalidation contract instead
  - widened scope beyond `worldwake-systems` to `worldwake-core`, `worldwake-sim`, and a small `worldwake-ai` exhaustiveness fix because the live architecture required it
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo test -p worldwake-sim` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo build --workspace` passed
