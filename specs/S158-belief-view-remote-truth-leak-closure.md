# S158 — Belief-View Remote-Truth Leak Closure

**Status:** Draft
**Type:** Correctness fix (no new state, systems, components, or feedback loops)
**Priority:** Highest — blocks adding new AI behavior until ignorance is durable.
**Foundations:** FND-7, FND-14, FND-14A, FND-16, FND-19, FND-27, FND-31
**Extends:** `archive/specs/S155-belief-view-boundary-correctness.md` (fixed
`effective_place` and the un-gated `can_control` belief-accessibility gate; this
spec closes the remaining economic/production/physical/contention leaks the first
iteration did not reach). Deferral consistent with
`archive/specs/S157-planner-snapshot-admission-provenance.md`, which already
declined the heavier `PlannerVisible<T>` / source-typed-trait refactor.

## Problem Statement

### Motivation

`crates/worldwake-sim/src/per_agent_belief_view.rs` is the single AI- and
player-facing knowledge wall. The human CLI action menu
(`crates/worldwake-cli/src/handlers/actions.rs`) and the AI planner both consult
it through `get_affordances()` and `PlanningSnapshot`. Several accessor methods
on the view return **current authoritative world state for remote,
non-co-located entities** after only a weak "known entity" gate (or no gate at
all). This is a direct FND-14 violation: an agent can plan around facts it never
perceived, was told, or remembered, and — because the CLI shares the path — the
human player inherits the same omniscience (FND-19).

### Evidence (verified against code on 2026-05-21)

The second-iteration audit
(`reports/ai-architecture-consolidation-second-iteration.md`) flagged this as the
load-bearing finding. Direct code inspection confirms the following leaks in
`per_agent_belief_view.rs`:

| Accessor | Leak | Current behavior |
| --- | --- | --- |
| `has_sale_listing` (2125) | Economic | Reads `world.has_component_sale_listing` with **no gate**. |
| `seller_for_sale_lot` (2113) | Economic / social | Reads `world` sale listing + stock assignment with **no gate**; returns current facility controller. |
| `listed_sale_lots_at` (2094) | Economic | Filters by belief-based `entities_at`, but reads `world` sale-listing + stock-assignment components on each lot without co-location/belief gate. |
| `has_production_job` (2207) | Production | Reads `world.has_component_production_job` with **no gate**. |
| `carry_capacity` (1850) | Physical | Reads `world.get_component_carry_capacity` with **no gate**. |
| `load_of_entity` (1856) | Physical | Wraps `load_of_entity(world, …)` with **no gate**. |
| Contention reads (`facility_queue_position` 1121, `facility_grant` 1127, `extraction_slot_queue_position` 1133, `actor_holds_extraction_slot_grant` 1144, `contention_queue_is_full` 1177) | Temporal | Read `world` queue/grant components with no method-level gate; mitigated only by caller-side filtering. |

### Corrections to the source report (do not re-introduce)

These audit claims were **refuted** by code inspection and must not be specced:

- **`direct_container` / `direct_possessor` are already correctly gated**
  (`knows_entity || has_authoritative_local_visibility || owned`, lines
  1832–1848). The audit's "Critical: container/possessor leak" is already fixed.
- **Per-field snapshot provenance is overstated as missing.**
  `planning_snapshot.rs::build_snapshot_entity` already resolves fields
  belief-first (`belief_backed.or_else(|| view.X)`) and gates `direct_container`
  on co-location. The leak is the leaky *view fallbacks*, not absent per-field
  handling. Fixing the view removes the snapshot leak-freezing without a
  `Sourced<T>`/`FieldSource` rewrite (audit §10/§14) — that static-typing refactor
  is explicitly **out of scope** (deferred Option C).

### Scope: social/control rights are out of this spec

The audit (and an earlier draft of this spec) also listed `can_control` (line
433) and `believed_rights` (line 420) as leaks. Reassessment against
`archive/specs/S155-belief-view-boundary-correctness.md` shows this would reopen a
**settled, documented design decision** and require infrastructure this spec
deliberately excludes:

- The `can_control` early-exit (lines 434–442) is the **lawful FND-14A
  co-located-unowned-physical-item shortcut** — S155 labels it
  "FND-14A co-location shortcut (legal)" and `docs/planner-contracts.md` §2
  blesses it. The `effective_place(actor) == effective_place(entity)` test *is*
  the co-location check; the shortcut never fires for remote entities. It must be
  preserved, not "fixed."
- S155 deliberately gated `can_control`/`believed_rights` behind belief
  *accessibility* and chose to retain the current-world rights/control *value*
  read, with **"no parallel `believed_can_control` method, so no fossil"**
  (S155 line 165), and its FOUNDATIONS table claims this satisfies FND-14A.
- Making the rights/control *value* belief-backed would require a believed
  rights/control/jurisdiction surface that does not exist:
  `EntityBeliefAspect` (`crates/worldwake-core/src/entity_belief_claim.rs:17`)
  carries `Owner`, `Holder`, `ContentionState`, `Activity`, `Inventory`, but **no
  Rights/Control aspect**. Building it is net-new belief infrastructure beyond a
  behavioral fix.

The stricter FND-14A reading (rights *values* belief-backed) is a legitimate
future concern but belongs in a separate spec that adds the believed-rights
`EntityBeliefAspect`. See Non-Goals.

### Key scoping decisions (brainstorm + reassessment 2026-05-21)

- **Behavioral fix only.** Make leaky accessors gate properly; do not introduce
  `Sourced<T>`, `FieldSource`, source-class traits, per-field snapshot source
  tags, or a believed-rights belief aspect.
- Proof is a **failing-first golden adversarial leak suite** (TDD: each test must
  fail against current `main`, pass after the fix).

## Non-Goals

- **Social/control rights value-belief-backing.** `can_control` and
  `believed_rights` keep S155's belief-accessibility gating and the lawful FND-14A
  co-located-unowned-item shortcut. Making effective-rights/control *values*
  belief-backed requires a new believed-rights/control/jurisdiction
  `EntityBeliefAspect` and is deferred to a future spec.
- **Static source-typing.** No `Sourced<T>` / `FieldSource` / per-field snapshot
  source tags (deferred Option C; consistent with S157).

## The Source-Class Rule

Every planner- and player-visible accessor in scope returns a value for an entity
only when one of the following holds:

1. **Self** — the entity is the observing agent.
2. **Same-tick co-located physical observation (FND-14A)** — the entity shares
   the agent's effective place *and the fact is a directly perceivable physical
   property*: kind, item-lot commodity/quantity, workstation tag, resource-source
   availability, container contents, encumbrance/load, carry capacity, the
   *existence* of a displayed sale listing and its commodity/quantity, and the
   observable busy/idle state of a co-located workstation.
3. **Direct possession** — the entity is directly possessed by the agent.
4. **Belief / memory** — the agent holds a belief or memory entry about the
   fact, carrying provenance and freshness (FND-15/16), backed by the appropriate
   existing surface (e.g., `EntityBeliefAspect::Activity` for production activity,
   `EntityBeliefAspect::ContentionState` for queue/grant state, or the
   opportunity-memory pathway for remote sale availability).

For any remote entity that satisfies none of these, the in-scope accessor returns
the belief-backed value if one exists, otherwise `None` / empty / `false`. It
must **never** fall back to current authoritative world state.

This is the forward-looking source-class principle. Social and relational facts
(seller/controller identity, effective rights, control, jurisdiction) are
belief-gated even when co-located (FND-14A) — but per the Scope note above, the
control/rights *value* path stays as S155 left it in this spec; only the
co-location-or-belief gating of the economic, production, physical, and contention
accessors is closed here.

## Deliverables

1. **`crates/worldwake-sim/src/per_agent_belief_view.rs`** — bring the in-scope
   accessors under the source-class rule:
   - Economic (`has_sale_listing`, `seller_for_sale_lot`, `listed_sale_lots_at`):
     return displayed-listing physical facts and seller/controller identity only
     for co-located lots; for remote lots return empty/None. Remote "where can I
     buy X" already flows through the existing opportunity-memory /
     `DemandObservation` pathway in candidate generation — do **not** build a
     believed-sale-listing surface; rely on that pathway for remote acquisition.
   - Production (`has_production_job`): co-located workstation busy/idle is
     observable; remote requires belief backed by `EntityBeliefAspect::Activity`.
   - Physical (`carry_capacity`, `load_of_entity`): co-located or directly
     possessed may read; remote requires belief.
   - Contention (`facility_queue_position`, `facility_grant`,
     `extraction_slot_queue_position`, `actor_holds_extraction_slot_grant`,
     `contention_queue_is_full`): gate at the method on co-location (the actor is
     present at the facility) or belief backed by
     `EntityBeliefAspect::ContentionState`; do not rely on caller discipline.
2. **`docs/planner-contracts.md` §2** — add an explicit "Planner-visible fields
   are source-scoped" subsection codifying the source-class rule and listing the
   economic/production/physical/contention accessors now under it. Note that the
   control/rights value path is unchanged (governed by the existing §2 control
   language) and that stricter value-belief-backing is deferred.
3. **`docs/spec-drafting-rules.md`** — add a rule: every new belief-view accessor
   must declare its source class (self / same-tick local physical / direct
   possession / belief-backed / public topology) and its stale/unknown behavior
   before implementation. Social/relational facts are belief-gated even when
   co-located.
4. **Golden adversarial leak suite** (`crates/worldwake-ai/tests/scenarios/`,
   extending the `belief_wall_trap` family) — see Section H proof surface.

## Authoritative-to-AI Impact Analysis

Tightening these accessors removes facts from the planner's belief view, so the
full decision cycle was traced (CLAUDE.md checklist):

1. `get_affordances` — affordances that depended on remote sale/job/queue truth
   must no longer appear; co-located and belief-backed affordances are unchanged.
2. `generate_candidates` — economic/restock/production candidates that depended on
   remote stock/job truth must now require belief or local observation; remote
   acquisition routes through the existing opportunity-memory pathway.
3. `search_plan` — strategic/tactical search degrades to belief-backed or
   exploratory behavior, not error, when a previously-leaked fact is now unknown.
4. `BestEffort` action start — gracefully handles a target whose remote state is
   now unknown.
5. `handle_plan_failure` — replans when local observation contradicts a stale
   belief, producing belief/blocker state (not silent retry).
6. Payload revalidation — untargeted actions with synthesized payloads still
   revalidate through the handler's validator; confirm no in-scope accessor used
   in `requested_affordance_matches` regresses.
7. ALL golden tests pass (`cargo test -p worldwake-ai`).

## FND-01 Section H Analysis

This is a correctness fix that removes unlawful information; it introduces no new
system, state, component, action, or feedback loop.

- **Information-path analysis:** Not applicable as a new path. The change
  *enforces* existing lawful paths: remote economic/production/temporal facts must
  arrive via perception (co-location), testimony, records, or memory carriers
  (FND-7/15), backed by existing surfaces (`EntityBeliefAspect::Activity`,
  `EntityBeliefAspect::ContentionState`, opportunity memory). The spec removes the
  unlawful "view reads current world" shortcut.
- **Positive-feedback analysis:** Not applicable. No amplifying loop is created or
  removed.
- **Concrete dampeners:** Not applicable. No feedback loop to dampen.
- **Stored-state vs. derived read-model list:** No new authoritative state. The
  belief view remains a **derived read-model** over (a) authoritative world state
  for lawful same-tick local physical observation and (b) the agent's
  authoritative belief/memory stores. The fix narrows which world reads the
  derived view is permitted to perform; it does not promote any derived value to
  truth (FND-27). Authoritative dispatch/commit continue to read `World` directly.
- **Planner-formalism analysis:** No formalism change. Plain GOAP and HTN
  selection both consume the same view; narrowing the view changes the *inputs*,
  not the search formalism. No goal becomes method-required.

### Proof surface (FND-31, failing-first)

Each golden must **first demonstrate the leak is reachable for a genuinely remote
(non-co-located) entity** — i.e. fail against current `main` — then pass after the
fix. One scenario per in-scope leak class:

| Scenario | Asserts |
| --- | --- |
| Remote seller delists unseen | Agent keeps stale "market sells X" belief or unknown; no instant delist knowledge; no candidate retraction without a carrier. |
| Remote seller restocks unseen | No new acquire candidate appears until testimony/record/local observation arrives (via the opportunity-memory pathway, not a direct view read). |
| Remote production job starts/finishes unseen | Planner does not learn the workstation became busy/free; `TargetLacksProductionJob`-driven affordances unchanged until observed. |
| Remote queue/grant changes unseen | Planner cannot optimize against an unseen queue; contention affordances unchanged. |
| Remote load/capacity changes unseen | Planner does not adjust route/trade/escort assumptions from remote encumbrance change. |
| AI-vs-Human affordance fingerprint | For each scenario above, swapping `ControlSource` between `Ai` and `Human` yields an identical lawful affordance set (FND-19). |

Negative controls (must still pass): co-located observation of a displayed
listing and a busy workstation still produces the correct affordance, proving the
fix does not over-suppress lawful FND-14A reads.
