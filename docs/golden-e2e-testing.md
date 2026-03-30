# Golden E2E Testing Conventions

Use this document when adding or revising tests under `crates/worldwake-ai/tests/golden_*.rs`.

It exists to keep golden assertions aligned with the architecture instead of drifting into brittle scheduler-coupled checks.
For the live mechanical inventory and docs-sync validation workflow, use `python3 scripts/golden_inventory.py --write --check-docs` and the generated artifacts at `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-map.md`.

## Assertion Hierarchy

Prefer the strongest, most semantic assertion surface available:

1. **Request-resolution traces**
   - Use for pre-start request binding or rejection facts.
   - Examples: "`RequestResolutionOutcome::RejectedBeforeStart` proved the request never reached authoritative start", "request bound through `ReproducedAffordance` before a later `StartFailed`".
2. **Authoritative world state**
   - Use for durable outcomes.
   - Examples: office holder, location, commodity totals, wound state, containment, relations.
3. **Action traces**
   - Use for lifecycle ordering and execution facts.
   - Examples: "`eat` committed before `declare_support`", "action started but never committed", "action aborted with reason".
4. **Decision traces**
   - Use for AI reasoning questions.
   - Examples: "candidate existed but was suppressed", "plan search exhausted frontier", "agent selected X over Y".
5. **Event log**
   - Use when event provenance, tags, or public record visibility is itself the contract.
   - Do not default to event-log ordering when action traces or authoritative state express the behavior more directly.

When multiple semantic surfaces could prove the invariant, prefer the earliest causal boundary that proves the contract. Only widen the golden to later execution or durable-state consequences when that later boundary is itself part of the promise under test.
For delivery or restock scenarios, do not overstate the durable ownership boundary. If the live architecture lawfully satisfies the contract by materializing stock at the destination or home market, assert that destination-local stock exists there instead of forcing "item remains in actor inventory" as the success condition.

## Needs-State Assertion Guidance

Needs values (bladder, hunger, thirst, fatigue, dirtiness) are **transient concrete state** (Principle 3: Concrete State Over Abstract Scores). A relief action resets the value at commit (e.g., `toilet` sets bladder to `pm(0)`), but basal metabolism immediately resumes accumulating drift on every subsequent tick.

Asserting `need == pm(0)` at an arbitrary tick count after relief will fail whenever basal drift has accumulated between the commit tick and the assertion tick.

### Preferred patterns

- **Break at commit**: loop ticks until action trace shows the relief action committed, then assert state immediately at that tick boundary. This is the strongest pattern because it asserts the reset at the exact causal moment.
- **Action trace proof**: assert that the relief action committed (proving the need was reset) rather than sampling authoritative state at a later tick. This is appropriate when the contract is "relief happened" rather than "need is at a specific value."
- **Bounded tolerance**: if testing post-commit state at a later tick, use `value <= basal_rate * max_ticks_since_commit` rather than exact equality. Name the basal rate and tick window explicitly in the test rationale.

### Transient vs. durable distinction

Not all consequences of a needs action are transient:

- **Transient**: the need value itself (bladder, hunger, etc.) — continues evolving from basal metabolism after reset.
- **Durable**: waste entity creation (persists as an `ItemLot`), dirtiness penalty from accidents (only removed by explicit wash action), deprivation wounds (persist until healed).

When a golden test cares about the durable consequence (e.g., waste was created, wound exists), assert the durable entity or component directly. When the test cares about the transient need value, use the break-at-commit or bounded-tolerance patterns above.

Reference: Principle 3 (Concrete State Over Abstract Scores) in `docs/FOUNDATIONS.md`.

## Ordering Rules

When a test needs ordering, state explicitly which ordering is the contract:

- strict tick separation
- action lifecycle ordering
- event-log ordering
- authoritative state transition ordering

Do not treat incidental tick-boundary details as the contract unless the system is intentionally specified that way.
If the scenario spans multiple layers, state which earlier layer drives the divergence and which later layer is only a downstream consequence.
If two actors can lawfully complete relevant actions in the same tick, do not rewrite that contract as "later tick" unless strict tick separation is the intended engine rule. In those cases, action-trace ordering should be asserted via the explicit `(tick, sequence_in_tick)` key on `ActionTraceEvent`.

Good:
- no `declare_support` commit while hunger remains `High-or-above`
- `eat` commits before `declare_support`

Bad:
- hunger relief must appear on a strictly earlier tick number than all later political commits

The first pair encodes the architectural rule. The second overfits to scheduler timing.

Do not use delayed authoritative installation as a proxy for earlier political-action ordering when succession or another lawful system can add delay between the action commit and the final office-holder mutation. In that case, prove the earlier ordering with action traces and prove the later durable consequence with authoritative world state.

Do not claim a "same-state, weight-only divergence" unless both compared branches are driven by comparable ranking substrates in the current architecture. If one branch depends on a pressure-scaled or priority-derived substrate and the other uses a flat motive or later system resolution, name that asymmetry explicitly in the ticket and in the test rationale.
Equal utility weights do not imply equal motive scores. Before claiming branch symmetry, a tie, or "priority-class only" divergence, validate the live arithmetic for the compared branches and name the concrete substrate that actually differs or stays equal: pressure, weight, promotions, caps, or other ranking inputs.

## Trace Guidance

### Use request-resolution traces when:

- proving whether a request was rejected before authoritative start
- proving which binding path (`ReproducedAffordance` vs `BestEffortFallback`) carried a request into start
- distinguishing "request never reached start" from "request reached start and then lawfully failed"
- debugging stale or retained concrete requests whose truth boundary is affordance reproduction rather than action execution

When request-resolution tracing exists for the scenario, do not claim pre-start rejection from missing action-trace events alone. Use `RequestResolutionOutcome::RejectedBeforeStart` directly for that boundary.
When a scenario involves stale or retained requests, state explicitly whether the contract is request-resolution rejection before start, authoritative `StartFailed` at start, or post-start abort after lawful start.

### Use action traces when:

- proving one action completed before another
- proving an action started, committed, aborted, or failed to start
- proving same-tick actions that are invisible to inter-tick active-action inspection
- proving same-tick cross-agent causal order without overfitting to tick numbers
- proving a committed `tell` targeted a specific `listener`/`subject` pair via `ActionTraceDetail::Tell`

### Use decision traces when:

- debugging why a goal did or did not appear
- proving suppression, ranking, or planner-search behavior
- distinguishing "candidate missing" from "candidate present but filtered/suppressed"
- proving negative AI invariants such as "this goal never appeared" or "this candidate was never generated"
- inspecting the final selected path via `planning.selection.selected_plan` and `planning.selection.selected_plan_source` when you need the chosen plan shape, terminal semantics, or whether the trace reflects a fresh search result, retained current plan, or snapshot-only continuation
- proving travel-led route selection when the contract is about the initial planned path rather than only eventual arrival
- proving social omission reasons such as `SpeakerHasAlreadyToldCurrentBelief` before any `tell` commit exists

When the contract is about candidate generation, ranking, suppression, or plan selection, do not infer the result indirectly from missing event-log entries or missing committed actions if a decision trace can prove it directly.
`archive/tickets/completed/S16S09GOLVAL-002.md` is the concrete example of this narrowing: the durable downstream outcome mattered less than the earlier changed-conditions selection boundary, so the golden was corrected to prove "first post-resolution selected goal is non-combat" instead of broad eat/heal follow-through.
`archive/tickets/completed/S16S09GOLVAL-004.md` is the travel-planning example of the same rule: the durable arrival/harvest outcome matters, but the ticket's actual promise starts earlier at the selected path boundary, so the golden proves both `selection.selected_plan.next_step` and the later Orchard Farm outcome instead of inferring route quality from arrival alone.
For conversation-memory crowd-out scenarios, prove the stale subject was omitted with the concrete social omission reason before claiming an untold subject survived truncation. The absence of a duplicate `tell` commit by itself is too weak because that could also arise from ranking loss, invalidation, or unrelated execution failure.
For social scenarios, action traces and decision traces answer different questions: action traces prove that a committed `tell` happened for a specific `listener`/`subject`, while decision traces prove why another `ShareBelief` candidate was omitted, suppressed, or never generated.
When a decision trace proves the selected outcome but still does not expose the concrete planner provenance needed to explain that outcome architecturally, do two things: drop to focused lower-layer tests for the immediate implementation work, and open a follow-up traceability ticket if that missing provenance matters to the architecture. Do not paper over that gap with ad-hoc debug output or weaker downstream assertions.
`archive/tickets/completed/S18PREAWAEME-003.md` is the concrete planner-surface example: the initial ticket narrative assumed a stale `ProduceCommodity` branch, but reassessment of the live operator surface showed the lawful chain belonged under `RestockCommodity`. The ticket was corrected first, then the golden was written against the live planner contract.

### Request-Resolution Boundary Examples

- `strict_request_records_resolution_rejection_without_start_attempt` in `crates/worldwake-sim/src/tick_step.rs` is the focused proof that a request can be rejected before start.
- `best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in `crates/worldwake-sim/src/tick_step.rs` is the focused proof that a request can bind first and then still hit authoritative `StartFailed`.
- `golden_care_pre_start_wound_disappearance_records_blocker` and `golden_local_trade_start_failure_recovers_via_production_fallback` in `crates/worldwake-ai/tests/` are golden examples of the later start-failure and reconciliation boundary, not proof of pre-start rejection.

### Recoverable Authoritative Start Failure

When the contract is "a lawful start rejection is recoverable," prove it in two steps:

- use an action trace to prove the action reached authoritative start and recorded `StartFailed`
- use the next AI tick's decision trace to prove `planning.action_start_failures` was consumed and the stale branch was cleared, blocked, or replaced

Do not treat "no later commit happened" as sufficient evidence of reconciliation. That symptom is too weak because it can also come from request-resolution rejection before start, candidate omission, ranking loss, plan-search failure, or unrelated execution failure.
Current golden examples of this proof shape include the care, production, trade, and political start-failure suites in `crates/worldwake-ai/tests/`.

### Use both when:

- the AI reasoning contract and the execution contract are both under test

For same-tick cross-agent chains, `events_at(tick)` and `events_for_at(actor, tick)` tell you which events happened within the tick, but not the contract by themselves. Use the recorded `sequence_in_tick` field when the assertion depends on relative order among those events.

### Use a cross-layer timeline when:

- you are debugging or asserting a mixed-layer chain and need one derived per-tick view across decision, action, politics, and explicitly selected event-log records
- you want a readable merged timeline without weakening the underlying assertions

Keep authoritative event-log selection explicit. Do not rely on helper heuristics to infer which authoritative records belong in the timeline.

## Determinism Pattern

New golden scenarios should usually add a deterministic replay companion test unless one of these is true:

- the scenario is intentionally non-deterministic by design
- the scenario is too small and redundant with an existing deterministic helper
- the owning ticket explicitly justifies why replay coverage is unnecessary

## Scenario Isolation

When a golden scenario is intended to prove one specific causal branch, document the scenario-isolation choice explicitly if the current architecture lawfully permits competing affordances that could also satisfy local needs or planner branching.

For political goldens under E16c, seed office-holder, faction-membership, and support-declaration knowledge through the institutional belief substrate or record consultation. Do not rebuild removed live-helper assumptions in test setup just to preserve an older political outcome.

State all of the following in the owning ticket/spec:

1. the intended branch or invariant under test
2. the lawful competing affordances the current architecture would otherwise allow
3. which unrelated lawful branches were intentionally removed from setup, and why they are outside the contract under test

This guidance exists to keep goldens honest, not to stage-manage outcomes. Remove unrelated lawful affordances only when they would obscure the invariant you are trying to prove. If the competing branch is part of the architecture contract, keep it and assert the branching behavior directly instead.

When the intended branch depends on authoritative arithmetic or cumulative mechanics, the owning ticket/spec must also state the concrete setup math that makes the branch reachable: the relevant delta, cadence, threshold, tolerance window, capacity, or other live formula inputs. Do not write these scenarios as narrative expectations alone.
For repeated threshold firing, wound accumulation, resource depletion, recovery gating, or similar cumulative mechanics, document the survival/failure envelope explicitly. If the intended branch is impossible under current formulas, correct the scenario numbers in the ticket/spec instead of weakening production behavior or papering over the mismatch with weaker assertions.
`archive/tickets/completed/S17WOULIFGOLSUI-001.md` is the concrete deprivation example: the clean fix was to adjust the scenario thresholds and above-critical hunger values so two lawful deprivation fires could occur under live arithmetic, not to weaken `worsen_or_create_deprivation_wound`.

For social goldens, document whether the speaker needs an explicit belief about the intended listener for `ShareBelief` to materialize. Blind-perception or heavily isolated setups often require explicit listener-belief seeding even when the agents are co-located.
For social goldens, also document subject choice explicitly. Agent subjects can create additional lawful `ShareBelief` branches around the subject's own changing state or location. If the contract is about resend suppression or a specific downstream office fact, prefer a non-agent subject unless the extra agent-subject branches are part of the invariant under test.
For spatial-planning goldens, document whether the contract includes the default planning budget itself. If it does, state that explicitly and remove nearer lawful alternatives from setup only when the invariant under test is route reachability from a branchy hub rather than competition among local food branches.
When a focused planning test is specifically about a planner failure boundary, assert the exact failure mode your scenario is meant to prove instead of only asserting "no plan". Use `BudgetExhausted`, `FrontierExhausted`, or another concrete planner-owned boundary as appropriate. Generic non-success is too weak because it also matches unrelated earlier contract breaks.

## Outdoor Place Affordance Trap

When designing golden scenarios that require an agent to travel for relief (bladder, dirtiness), be aware that `relieve_wilderness` is available at any place with an `OUTDOOR_RELIEF_TAGS` tag (Forest, Trail, Field, Farm, Road). The planner will prefer it over traveling to a distant latrine because it has zero travel cost.

Outdoor places in the prototype world (at least one tag in `OUTDOOR_RELIEF_TAGS`): EastFieldTrail (Trail + Field), OrchardFarm (Farm + Field), ForestPath (Forest + Trail), NorthCrossroads (Crossroads + Road), SouthGate (Gate + Road), BanditCamp (Camp + Forest).

Indoor places (no outdoor relief tags): VillageSquare (Village), GeneralStore (Store + Village), CommonHouse (Inn + Village), RulersHall (Hall + Village), GuardPost (Barracks + Village), PublicLatrine (Latrine + Village).

To force travel for relief:
- Start the agent at an indoor place (no wilderness relief available)
- Or use a different need driver (hunger, thirst) where the resource is distant

This generalizes: any scenario that relies on travel must ensure no local affordance satisfies the motivating goal at the starting place.

The canonical source is `OUTDOOR_RELIEF_TAGS` in `crates/worldwake-core/src/topology.rs`.

## Deprivation Ordering Trap

The tick execution order is: drain inputs → progress actions → run systems. Among systems, Needs runs first (`system_manifest.rs`), but **all actions complete before any system runs** (Principle 9: Scheduling, Simultaneity, and Tie-Breaking Are Part of the World Model).

This means an agent can complete a 1-tick travel action within the same tick **before** the needs system fires deprivation consequences. A deprivation accident (e.g., waste creation from bladder failure) is created at the agent's `effective_place` at the time the needs system runs — which may not be the agent's starting place if travel completed first.

This applies to any golden test where a system-level consequence must fire at a specific place. The agent's starting location is not guaranteed to be their location when systems execute in the same tick.

### Preferred patterns

- **Check agent's actual location**: use `h.world.effective_place(agent)` for waste or consequence location assertions rather than hardcoding the starting place. The waste appears where the agent is when the needs system runs, not where they were when the tick began.
- **Race the accident**: set `bladder_accident_tolerance_ticks` to 1 so the accident fires on the first critical tick, but accept that travel may still complete first within that tick.
- **No fully isolated indoor place exists**: every indoor prototype place is within planner budget of PublicLatrine. Deprivation accident tests cannot rely on isolation alone to prevent the agent from traveling away; they must use low tolerance values to trigger the accident before replanning can intervene.

### Why this happens (Principle 26)

The needs system reads state written by completed actions (Principle 26: Systems Interact Through State, Not Through Each Other). It does not coordinate with the travel system directly. If travel commits first (placing the agent at a new location) and then the needs system fires a deprivation consequence, that consequence lands at the new location because that is the authoritative placement when the system runs.

Reference: `system_manifest.rs` for tick execution order; Principle 9 (Scheduling) and Principle 26 (Systems Interact Through State) in `docs/FOUNDATIONS.md`.

## Multi-Hop Travel Observation

Multi-hop travel (e.g., VillageSquare → SouthGate → EastFieldTrail → OrchardFarm) creates one travel action per leg. Between legs, the agent replans (~1 tick gap). Tests counting total travel ticks must tolerate inter-leg gaps rather than breaking out of the observation loop after the first leg ends.

## Belief Seeding After Political State Changes

Politics runs before Perception in the tick loop (`system_manifest.rs`). When an agent is co-located during a political state transition (e.g., controller establishment, contested state activation), Perception projects the political event into the agent's institutional belief store in the same tick.

If a test later seeds a *different* institutional belief for the same key (e.g., seeding `contested: true` after the agent already perceived `controller: Some(A), contested: false`), the two observations coexist and `believed_force_controller` returns `Conflicted` instead of `Certain`.

**When the seeded belief agrees** with what perception projected (e.g., Suite 11 seeds the same controller perception already recorded), no conflict arises — both observations collapse to `Certain`.

**When the seeded belief disagrees** (e.g., Suite 12 seeds a contested state that contradicts the earlier uncontested observation), clear the stale entry before seeding:

```rust
let mut store = h.world.get_component_agent_belief_store(agent).cloned().unwrap();
store.institutional_beliefs.remove(
    &InstitutionalBeliefKey::ForceControllerOf { office },
);
let mut txn = new_txn(&mut h.world, tick.0);
txn.set_component_agent_belief_store(agent, store).unwrap();
commit_txn(txn, &mut h.event_log);
// Now seed_force_controller_belief will produce a clean Certain read.
```

This is not a hack — it models the agent updating their belief to newer information. The alternative (issuing all claims simultaneously) avoids intermediate observations but prevents testing sequential state transitions.

## Prototype World Topology Reference

The prototype world (`build_prototype_world`) defines directed travel edges between places. When issuing a human `RequestAction` for travel, the target must be a **directly adjacent** place — there is no multi-hop resolution for human inputs. The travel action's `TargetAdjacentToActor(0)` precondition will reject non-adjacent targets with a `StartFailed`.

| Place | Adjacent to (with travel time in ticks) |
|-------|----------------------------------------|
| VillageSquare | GeneralStore (1), CommonHouse (1), RulersHall (1), GuardPost (1), PublicLatrine (1), SouthGate (2) |
| GeneralStore | VillageSquare (1) |
| CommonHouse | VillageSquare (1) |
| RulersHall | VillageSquare (1) |
| GuardPost | VillageSquare (1) |
| PublicLatrine | VillageSquare (1) |
| SouthGate | VillageSquare (2), EastFieldTrail (3) |
| EastFieldTrail | SouthGate (3), OrchardFarm (2), NorthCrossroads (3) |
| OrchardFarm | EastFieldTrail (2) |
| NorthCrossroads | EastFieldTrail (3), ForestPath (4) |
| ForestPath | NorthCrossroads (4), BanditCamp (4) |
| BanditCamp | ForestPath (4) |

Note: OrchardFarm is **not** adjacent to VillageSquare. The shortest path is VillageSquare → SouthGate → EastFieldTrail → OrchardFarm (7 ticks). For human-controlled departure scenarios, use an adjacent place like GeneralStore (1 tick).

The canonical source is `PROTOTYPE_EDGE_SPECS` in `crates/worldwake-core/src/topology.rs`. If new edges are added, update this table.

## Force Installation Tracing

When the force-control system determines a `desired_controller` but the installation gate blocks (the controller cannot yet be installed as `office_holder`), the politics trace emits an additional `ForceInstallationDeferred` event alongside the normal resolution outcome (e.g., `ForceControllerMaintained`). This makes the installation gate's reasoning observable without requiring engine source code inspection.

The deferral reason (`ForceInstallationDeferralReason`) names the specific gate condition that blocked:

- `OtherLiveClaimants { controller, blocking_claimants }` — other alive claimants exist besides the controller (even if absent from the jurisdiction). Installation requires all live claimants to be the controller.
- `HoldIncomplete { held_ticks, required_ticks }` — the uncontested hold period has not yet elapsed.
- `NotUncontestedThisTick` — the controller was not uncontested this tick.

When debugging "why didn't the controller get installed as holder?", check for `ForceInstallationDeferred` events in the politics trace before reading engine source.

## Same-Tick Ordering for 1-Tick Actions

When travel to an adjacent place completes within the same tick as the departure (e.g., 1-tick travel to GeneralStore), the intra-tick ordering is:

1. **Input processing**: travel starts → actor enters transit → `effective_place` becomes `None`
2. **AI input production**: rival AI detects vacancy → generates goals → issues actions
3. **Action progression**: travel commits → actor placed at destination; rival actions commit

This means the departure and the rival's first observation of vacancy happen within the same tick. Decision trace assertions about "before departure" must use strict `<` on tick number, not `<=`, because the rival's ClaimOffice generation at the departure tick is causally correct — the controller has already entered transit before the AI runs.

## Ticket Precision

Golden-related tickets must follow `docs/precision-rules.md` for all technical claims.
Additionally, golden tickets should name the exact scenario gap and state whether it is missing focused coverage, missing golden coverage, or both.

## Verification Commands

Typical verification sequence:

1. targeted test name
2. owning golden test binary
3. crate suite
4. docs inventory refresh/validation via `python3 scripts/golden_inventory.py --write --check-docs`
5. repo verification baseline via `scripts/verify.sh`

If a stricter lint or broader suite is required, state that explicitly in the ticket.

## Scenario Metadata Authoring

Every `// Scenario` block in `golden_*.rs` should include structured metadata
that the generator (`scripts/golden_inventory.py`) extracts into documentation.

### Required Format

Place structured keys after the scenario header, within the same `//` comment
block, before the first `fn` definition:

    // ---------------------------------------------------------------------------
    // Scenario XX: Short Descriptive Title
    // ---------------------------------------------------------------------------
    //
    // Systems: Needs, AI, Travel
    // GoalKinds: ConsumeOwnedCommodity, AcquireCommodity(SelfConsume)
    // ActionDomains: Needs, Travel, Production
    // Places: VillageSquare, OrchardFarm
    // Principles: 3, 7, 20
    //
    // Setup: Brief description of the initial world state and agent
    //   configuration. Focus on what makes this scenario unique.
    //
    // Proves: What emergent behavior does this scenario demonstrate?
    //   State the architectural claim, not just "agent does X". Each
    //   point should name what system interaction is being proven.
    //
    // Chain: The cross-system causal chain from trigger to outcome.
    //   Use arrows: pressure -> goal -> plan -> action -> consequence.

### Key Reference

| Key | Content | Format |
|-----|---------|--------|
| Systems | Which system modules are exercised | Comma-separated |
| GoalKinds | Which GoalKind variants are tested | Comma-separated, with qualifiers in parens |
| ActionDomains | Which ActionDomain values are covered | Comma-separated |
| Places | Which prototype topology places are used | Comma-separated |
| Principles | Which Foundation Principles are tested | Comma-separated numbers |
| Setup | Initial world state description | Free-form prose, multi-line via indented continuation |
| Proves | What emergent behavior is demonstrated | Free-form prose, multi-line |
| Chain | Cross-system causal chain | Free-form prose with `->` arrows |

### Prose Standard

- **Setup**: What makes this scenario's initial state unique? Name concrete
  values (pm(800), Quantity(4)) when they are load-bearing for the scenario.
- **Proves**: What architectural claim does this scenario lock down? Frame each
  point as "X proves Y" rather than "agent does Z". Focus on cross-system
  emergence, not single-system behavior.
- **Chain**: Trace the causal chain from trigger to final outcome using arrows.
  Name system boundaries when crossing them.

Keep each field concise — 1-4 lines.

### Multi-Line Continuation

Continue a key's value by indenting the next `//` line with 3+ spaces:

    // Setup: Hungry agent at Village Square with no food.
    //   Orchard Farm has apples via OrchardRow + ResourceSource.
    //   The shortest route requires 3 travel legs.

### Header Placement

The `// Scenario` header must be placed near the `fn golden_*` test functions
it covers, not near builder functions hundreds of lines away. The parser
associates `fn golden_*` names with the most recent `// Scenario` header.

### Regenerating Docs

After editing scenario metadata, run:

    python3 scripts/golden_inventory.py --write --check-docs

This regenerates `docs/generated/golden-scenario-map.md`,
`docs/generated/golden-e2e-inventory.md`, and
`docs/generated/golden-coverage-matrix.md`. Commit all three.
