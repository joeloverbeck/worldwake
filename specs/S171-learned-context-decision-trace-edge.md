# S171: Learned-Context Decision-Trace Edge

**Status**: DRAFT

## Problem Statement

S170 (`archive/specs/S170-learned-state-provenance-hardening.md`, COMPLETED
2026-05-25) closed four provenance gaps in the learned-state *stores*:
`LearnedOpportunityMemory::OpportunityEntry.source`,
`RoutePreference::record_safe` event provenance, `DiscrepancyEntry::source`,
`Blocker::source`. Every learned mutation now carries an inspectable origin
field (`Event(EventId)` or a domain-specific `ReadPhaseInference` / `Inferred`
sentinel).

The fourth-iteration ChatGPT-Pro audit
(`reports/ai-architecture-improvements-fourth-iteration.md` §§ 4, 5, 6, 14)
flags as a watchlist seam that the **consumption** side of these stores is
still trace-opaque: the auditor can answer "what produced this learning
update?" from the stored field, but cannot answer "which learned update
affected this later decision?" from the trace. The audit characterises the
gap as future polish. Independent verification on current `main` shows the
gap is **structural at ranking time** and undermines S170's investment in
provenance:

1. **`learned_opportunity_bonus`**
   (`crates/worldwake-ai/src/ranking.rs:439-460`) looks up
   `context.learned_opportunity_memory.opportunities.get(&opportunity)`,
   reads only `entry.expires_tick`, and returns `u32`. The matched
   `OpportunityEntry`'s `source`, `observed_tick`, and `observed_at` are
   discarded before return. The caller in `memory_motive_bonus`
   (`ranking.rs:397-411`) only sees the integer.
2. **`repair_memory_bonus`** (`ranking.rs:413-437`) looks up
   `context.repair_memory.repairs.get(&signature)`, reads
   `entry.expires_tick` and `entry.success_count`, and returns `u32`. The
   `BreachSignature` consulted and the success-count history are discarded.
3. **`SourceReliabilityDiscount`**
   (`crates/worldwake-ai/src/decision_trace.rs:773-780`) is the existing
   sibling attribution carrier for the testimony-reliability discount path.
   It records `source_entity`, `commodity`, `failure_ratio_permille`,
   `pre_discount_motive`, `post_discount_motive`. It does **not** record any
   provenance from the actual source-reliability entry that the ranking path
   consults. Live S171LEACONTDEC-002 reassessment corrected the original draft:
   the discount path reads `SourceReliability.sources: BTreeMap<SourceKey,
   ReliabilityRecord>`, not `TestimonyReliabilityEntry::provenance_events`.
   S171LEACONTDEC-004 owns landing a lawful source-reliability provenance
   producer for this discount axis.

Symmetric precedent exists: `RankedGoalSummary`
(`crates/worldwake-ai/src/decision_trace.rs:691-715`) already carries
`source_reliability_discount: Option<SourceReliabilityDiscount>` and
`competition_discount: Option<CompetitionDiscount>` for the **discount**
axes. The two **bonus** axes (learned-opportunity, repair-memory) have no
equivalent attribution record, and the discount axis that does has no link
to its consulted provenance events. `decision_trace.rs` contains zero
references to `LearnedOpportunity*` anywhere.

`RankedDriveGoalProvenance` (`crates/worldwake-ai/src/goal_model.rs:2212-2218`)
captures `motive_inputs: Vec<RankedDriveMotiveInput>` — concrete drive
pressure inputs — but has no field for "this ranking was boosted by learned
memory entry X acquired at tick T from event E."

## Concrete FND Test Failures

The gap fails three FOUNDATIONS tests directly. None requires a scenario to
manifest — the failure is inspectable from current trace content.

- **FND-22A test** (`docs/FOUNDATIONS.md:284`): *"If the explanation for a
  changed future choice is only 'the AI learned,' without an inspectable
  experience path and a concrete stored update, the design is cheating."*
  Today the **stored update** is inspectable (S170 delivered that), but the
  **experience path** from store entry to ranked candidate is not exposed in
  any trace structure. An auditor asking "did learned opportunity memory
  change this agent's selection?" can only answer by reconstructing the
  bonus arithmetic from external knowledge of the formula.
- **FND-29 test** (`docs/FOUNDATIONS.md:383`): *"For any nontrivial event
  chain, you must be able to inspect both the causal path and the knowledge
  path separately."* The causal path (decision trace, ranked candidates) is
  inspectable. The knowledge path (learned store mutations) is inspectable in
  saves. The **edge between them** — the moment ranking consulted a specific
  learned entry — is invisible.
- **FND-31** (`docs/FOUNDATIONS.md:435`): *"A golden passes constitutionally
  only if it proves the authored causal reason, or explicitly accepts a
  named alternative lawful branch."* A scenario asserting "this candidate
  won because of learned context" has no trace field to assert against
  today. Goldens can assert ranking *outcomes* but not the learned-context
  *reason* for the outcome; the fourth-iteration audit lists this exact
  failure mode under "Unacceptable remaining risks" (§16, line 282: "any
  gameplay golden that passes by plausible outcome instead of causal
  reason") yet leaves the supporting trace surface absent.

## Phase and Status

Adjunct Wave: AI Architecture Improvements — Third Iteration (single-spec
wave). Direct sequel to S170 (Learned-State Provenance Hardening, archived
2026-05-25). Independent of S169. No dependency on S60–S66; gameplay specs
remain excluded.

## Crates

- `worldwake-ai` (primary)
  - `src/ranking.rs` — bonus-return-shape change for two functions; thread
    new attribution into `RankedGoalSummary` construction.
  - `src/decision_trace.rs` — three new attribution structs
    (`LearnedOpportunityBonusAttribution`,
    `RepairMemoryBonusAttribution`, plus `SourceReliabilityDiscount` field
    additions); `RankedGoalSummary` field additions; formatter additions
    for observer/CLI output.
- `worldwake-cli` — **no changes**. Observer reads `RankedGoalSummary.motive_source_contributions` for slot derivation (`bin/observer.rs:1207-1213`); the new attribution fields are not read there. Trace text containing the new attribution summaries flows through observer's existing decision-trace dump path unchanged in shape.
- `worldwake-sim` — save-format version bump only. `SourceReliabilityDiscount`
  is serialized through `AgentDecisionRuntime.agenda_state`, so adding fields
  to that carrier changes the current runtime save shape.
- `worldwake-core` — **changes only if required by S171LEACONTDEC-004**.
  Learned-opportunity and repair-memory stores already carry the fields that
  S171LEACONTDEC-002 threads forward. Source-reliability discount provenance
  requires a lawful carrier on the live `ReliabilityRecord` path; it must not
  be synthesized from testimony reliability.

## Dependencies

- **Completed**: S170 (provides `LearnedOpportunitySource`,
  `TestimonyReliabilityEntry.provenance_events`, and the other provenance
  fields S171 surfaces).
- **No new dependencies on S60–S66.**
- **Does not depend on S169.**

## Design Goals

1. **Symmetric attribution coverage.** Every ranking-time adjustment that
   reads a learned-state store records an attribution carrier in
   `RankedGoalSummary`. Today: two discount axes carry attribution; two
   bonus axes do not. After S171: all four carry attribution.
2. **Provenance edges, not formulae.** Each attribution carrier surfaces
   the *identity* of the consulted store entry (`LearnedOpportunitySource`,
   `BreachSignature`, testimony provenance event id) plus pre/post motive
   values. Auditors reconstruct the bonus by reading the trace, not by
   re-deriving the formula off-trace.
3. **No behaviour change.** Ranking scores, selected candidates, decision
   ordering, and tick-by-tick agent behaviour remain byte-identical pre and
   post S171. The change is trace-content-only.
4. **Domain-specific attribution types, not a unified abstraction.** Two
   new attribution structs (one per bonus axis). Per FND-3 and the same
   anti-sludge reasoning S170 cited when rejecting a unified
   `LearnedStateUpdate` trait.
5. **Reuse existing attribution-carrier pattern.** Match the field shape
   of `SourceReliabilityDiscount` / `CompetitionDiscount`: concrete typed
   fields, `Serialize + Deserialize`, surfaced through `RankedGoalSummary`,
   rendered by the existing observer formatters.
6. **Save/load equivalence preserved for current data only.**
   `RankedGoalSummary` does not currently derive `Serialize`/`Deserialize`
   (only `Clone`, `Debug`), so the new bonus-attribution fields on that trace
   summary raise no save/load migration concern. `SourceReliabilityDiscount`,
   however, is serialized through `AgentDecisionRuntime.agenda_state`, so the
   new source-reliability provenance fields advance the current save format.
   Per FND-28, no old-save compatibility shim is added.

## Non-Goals

1. **No new goal-level verification kinds.** The audit's other watchlist
   seam (`GoalKind::AskWitness` is the sole entity-belief verification
   goal; `ConsultRecord` / `SearchPlace` remain repair-seam-only) is out of
   scope per triage decision; this spec is provenance-surfacing only.
2. **No unified `LearnedStateUpdate` abstraction.** Explicitly dropped, per
   S170's same reasoning.
3. **No new behaviour, no new bonus formulae, no new ranking inputs.** The
   bonus functions return the same `u32` they did pre-S171, plus a sibling
   attribution. Score arithmetic is untouched.
4. **No route-preference / discrepancy / blocker attribution.** Those
   learned stores are not read at ranking time on current `main`. If a
   future change wires them into ranking, that change carries its own
   attribution responsibility.
5. **No diagnostics-as-CI-gate.** The fourth-iteration audit recommends
   against a broad diagnostics dashboard; S171 respects that.
6. **No back-compat for older trace dumps.** Per FND-28, the new fields are
   added directly; consumers update or are removed.

## FOUNDATIONS Alignment

| Principle | Application |
|---|---|
| FND-3 (concrete state over abstract scores) | Two domain-specific attribution structs (one per bonus axis), not one shared `BonusAttribution` trait. Each field is concrete and typed (`LearnedOpportunitySource`, `BreachSignature`, `EventId`), not an opaque `u32` aggregate. |
| FND-22A (learning provenance has accountable origin and scope) | Closes the second half of the FND-22A "experience path" requirement. S170 made store updates accountable; S171 makes store *reads* at ranking time accountable. Together, the chain `event → learned-store mutation → ranking adjustment → selected candidate` is fully inspectable. |
| FND-26 (state-mediated systems) | No new cross-crate call paths. `worldwake-ai` continues to read learned stores; the threading is intra-crate from `ranking.rs` to `decision_trace.rs`. |
| FND-27 (derived summaries are caches) | `RankedGoalSummary` is a derived per-decision trace record. The new attribution fields are derived caches over learned-store contents, valid only for the tick that produced them; the stores remain authoritative. |
| FND-28 (no fossils) | No shim, no `serde(alias)`, no parallel attribution path. `RankedGoalSummary` field additions are direct; consumers (observer, CLI, tests) update accordingly. |
| FND-29 (debuggability) | "Why did this agent prefer this candidate?" — the answer now includes "boosted by learned opportunity entry X (observed at tick T from event E)" when applicable, without re-deriving the bonus arithmetic. |
| FND-29A (causal history is queryable) | The trace record permanently carries the learned-store entry identity at ranking time; replay/save-load preserves the consultation record alongside the selected candidate. |
| FND-31 (validation and falsification) | Enables the missing golden-assertion surface: a scenario can now assert "candidate C was selected because of learned context attribution L," not merely "candidate C was selected." Removes the trace-side blocker to the audit's own "Unacceptable remaining risks" item about plausible-outcome goldens. |

## Section H Causal Analyses

This spec changes no simulation behaviour: ranking scores, selected
candidates, decision ordering, and tick-by-tick agent state are
byte-identical pre and post S171. The only changes are additive trace
fields. Section H analyses apply as follows.

- **Information-path analysis.** *Not applicable.* No new agent-visible
  information path is created. Trace records are diagnostic surfaces under
  the observer/debug boundary (`worldwake-cli/src/bin/observer.rs`,
  consistent with FND-29 and the audit's §4 "AI consumes state/beliefs/
  traces" boundary). Agents do not read decision traces.
- **Positive-feedback analysis.** *Not applicable.* No new amplifying
  loops; bonus values and the candidate they boost are unchanged.
  Attribution is a passive byproduct.
- **Concrete dampeners.** *Not applicable* (no feedback loops).
- **Stored-state vs. derived read-model list.**
  - *Derived* (new): `LearnedOpportunityBonusAttribution`,
    `RepairMemoryBonusAttribution`, the added fields on
    `SourceReliabilityDiscount`, the
    `Vec<LearnedOpportunityBonusAttribution>` / `Option<…>` fields on
    `RankedGoalSummary`. All are per-tick derivations over the learned
    stores; recompute from the stores yields the same result.
  - *Stored* (versioned runtime carrier changed): the new
    `SourceReliabilityDiscount` fields are serialized when an agenda entry
    carrying that discount is saved in `AgentDecisionRuntime.agenda_state`.
  - *Stored* (unchanged): the learned stores themselves
    (`LearnedOpportunityMemory`, `RepairMemory`, `TestimonyReliability`)
    remain the sole authoritative source. No state is duplicated.
- **Planner-formalism analysis.** *Not applicable.* No planner change; no
  HTN method change.
- **Causal-equivalence contract.** *Not applicable.* No new
  cache/compression surface and no offscreen simulation. The new fields
  are forward-only diagnostic surfaces.
- **Systemic-validation analysis.** *Required.* See Validation below. The
  contract is byte-identical ranking output pre and post, plus a focused
  attribution-coverage assertion for every learned-store read site.

## Deliverables

### D1. `LearnedOpportunityBonusAttribution`

Add to `crates/worldwake-ai/src/decision_trace.rs` (sibling to existing
`SourceReliabilityDiscount` block at lines 773-780):

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LearnedOpportunityBonusAttribution {
    /// The opportunity key that matched the consulted entry. Equal to the
    /// candidate's `(goal_key, anchor)` pair.
    pub opportunity: OpportunityKey,
    /// Provenance of the consulted `OpportunityEntry`. Either a specific
    /// world event id or `ReadPhaseInference` per S170's
    /// `LearnedOpportunitySource`.
    pub entry_source: LearnedOpportunitySource,
    pub entry_observed_tick: Tick,
    pub entry_expires_tick: Tick,
    pub pre_bonus_motive: u32,
    pub post_bonus_motive: u32,
}
```

### D2. `RepairMemoryBonusAttribution`

Add to `decision_trace.rs`:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairMemoryBonusAttribution {
    /// The signature that matched the consulted repair-memory entry.
    pub signature: worldwake_core::BreachSignature,
    pub entry_success_count: u32,
    pub entry_expires_tick: Tick,
    pub pre_bonus_motive: u32,
    pub post_bonus_motive: u32,
}
```

### D3. `SourceReliabilityDiscount` provenance fields

Extend the existing struct at `decision_trace.rs:774-780`:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceReliabilityDiscount {
    pub source_entity: EntityId,
    pub commodity: CommodityKind,
    pub failure_ratio_permille: u32,
    pub pre_discount_motive: u32,
    pub post_discount_motive: u32,
    // NEW
    /// Number of provenance events currently retained for the consulted
    /// source-reliability entry. `0` when the entry exists without lawful
    /// event provenance or when a projected in-tick discount has no committed
    /// event id yet.
    pub provenance_event_count: u32,
    /// Most-recent provenance event id retained for the consulted
    /// source-reliability entry, or `None` when
    /// `provenance_event_count == 0`.
    pub most_recent_provenance_event: Option<EventId>,
}
```

### D4. `RankedGoalSummary` field additions

Extend `RankedGoalSummary` (`decision_trace.rs:691-715`):

```rust
pub struct RankedGoalSummary {
    // existing fields unchanged…
    pub source_reliability_discount: Option<SourceReliabilityDiscount>,
    pub competition_discount: Option<CompetitionDiscount>,
    // NEW
    pub learned_opportunity_bonus: Option<LearnedOpportunityBonusAttribution>,
    pub repair_memory_bonus: Option<RepairMemoryBonusAttribution>,
    // existing fields unchanged…
}
```

The `Default` impl gains the two `None` fields.

### D5. Bonus-function return-shape change

In `crates/worldwake-ai/src/ranking.rs`:

```rust
// Replace fn signatures (lines 439-460 and 413-437).
fn learned_opportunity_bonus(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
    base_motive: u32,
) -> (u32, Option<LearnedOpportunityBonusAttribution>) { … }

fn repair_memory_bonus(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
    base_motive: u32,
) -> (u32, Option<RepairMemoryBonusAttribution>) { … }
```

Each returns `(0, None)` when no entry matched or the entry has expired.
Each returns `(bonus, Some(attribution))` when a bonus is applied; the
attribution carries the consulted entry's identity and the pre/post motive
values.

`memory_motive_bonus` (line 397) is updated to thread the two attributions
back to its caller without summing them into the integer (the integer
remains the sum of the two bonuses, exactly as today).

### D6. Threading attributions into `RankedGoalSummary`

The candidate-ranking site at `ranking.rs:290-301` builds each per-candidate
`AgendaEntry` via `AgendaEntry::pending(...)`; `summarize_ranked_goal`
later projects that agenda entry into `RankedGoalSummary`. The local bindings
`source_reliability_discount` (line 277), `competition_discount` (line
282), and `provenance` (line 273) are already in scope. The two new
bonus-attribution fields therefore populate by extending the agenda-entry
carrier and then copying those fields in `summarize_ranked_goal`. The
`SourceReliabilityDiscount` construction currently remains placeholder-only
for provenance (`0` / `None`) until S171LEACONTDEC-004 lands the lawful
source-reliability carrier. The S171LEACONTDEC-002 reassessment proved the
original `TestimonyReliabilityEntry` read sketch was false for the live
discount path.

### D7. Decision-trace formatter additions

The decision-trace rendering of per-candidate suffixes lives in
`crates/worldwake-ai/src/decision_trace.rs` itself, where the existing
discount suffixes are concatenated into the trace text. Extend that path:

1. Add two new module-private formatter functions alongside
   `format_competition_discount_summary` (`decision_trace.rs:2429`) and
   `format_source_reliability_discount_summary` (`decision_trace.rs:2440`):
   - `format_learned_opportunity_bonus_summary(&LearnedOpportunityBonusAttribution) -> String`
     printing source-kind, observed/expires ticks, pre/post motive.
   - `format_repair_memory_bonus_summary(&RepairMemoryBonusAttribution) -> String`
     printing signature summary, success-count, expires tick, pre/post motive.
2. Extend `format_source_reliability_discount_summary` to print
   `provenance_event_count` and `most_recent_provenance_event` when
   `provenance_event_count > 0`.
3. At each existing suffix-concatenation site that consumes a
   `RankedGoalSummary` — `decision_trace.rs:317-325` (the dominant
   selected-candidate summary), `decision_trace.rs:1794-1802`, and
   `decision_trace.rs:2110` — add two new `*_suffix` bindings mirroring the
   existing `source_reliability_suffix` / `competition_suffix` pattern and
   thread the two new bonus suffixes into the same concatenated output.

Observer (`worldwake-cli/src/bin/observer.rs`) does not need code changes;
it consumes the rendered trace text through the existing dump pipeline,
and the new suffixes appear automatically.

### D8. Test fixture migration

`decision_trace.rs:3121-3140` has `sample_competition_discount` /
`sample_source_reliability_discount` test helpers. Add
`sample_learned_opportunity_bonus_attribution` and
`sample_repair_memory_bonus_attribution`; update existing
`sample_source_reliability_discount` to populate the two new fields with
representative values.

## Validation

Per FND-31, the systemic-validation contract for this spec is **trace
content coverage** plus **byte-identical ranking equivalence**, not a new
golden behaviour scenario.

### V1. Bonus-attribution coverage assertion

A new focused test in `ranking.rs` tests asserts:

- For every ranking decision where `learned_opportunity_bonus` returned a
  non-zero `u32`, the corresponding `RankedGoalSummary.learned_opportunity_bonus`
  is `Some(_)` with `post_bonus_motive == pre_bonus_motive + bonus`.
- Symmetrically for `repair_memory_bonus` / `RankedGoalSummary.repair_memory_bonus`.
- For every `SourceReliabilityDiscount` emitted after S171LEACONTDEC-004, if
  the underlying source-reliability entry has lawful event provenance,
  `provenance_event_count > 0` and `most_recent_provenance_event ==
  Some(last_event_id)`.

Extend the existing
`learned_opportunity_memory_boosts_matching_opportunity_only_while_live`
test (`ranking.rs:5883`) to also assert the attribution structure.

### V2. Byte-identical ranking equivalence

The existing golden and focused ranking tests that assert
`RankedGoalSummary.motive_score`, `priority_class`, `provenance`,
`source_composite`, `feasibility`, `acquisition_quantity`, and agenda
order **continue to pass without modification**. The two new bonus
attribution fields are additive — D5's tuple return preserves the same
integer that the pre-S171 `u32` return delivered, so `memory_motive_bonus`
sums to the same value and the downstream score arithmetic is unchanged.
Add one focused test in `ranking.rs` that constructs a ranking with
non-zero learned-opportunity and repair memory and asserts
`post_bonus_motive == pre_bonus_motive + bonus` for each axis when the
matching entry is live.

### V3. No new behaviour-changing golden required

S171 is not a new golden-class spec. No scenario rolls back, no agent
behaviour changes. The existing golden corpus continues to apply.

### Negative cases (illegal paths S171 must not produce)

- Attribution carrier present (`Some(_)`) when the underlying bonus was
  `0`. Forbidden because it would imply a phantom learned-context read.
- Attribution carrier absent (`None`) when the underlying bonus was
  non-zero. Forbidden because it is precisely the gap S171 closes.
- `most_recent_provenance_event` referencing an `EventId` not present in
  the consulted source-reliability entry's lawful provenance carrier.
  Forbidden because attribution must be sourced from the actual consulted
  entry, not synthesised.
- Any score arithmetic change. Forbidden by Design Goal 3 and V2.

## Open Questions

None. The deliverables are direct compiler-driven refactors of existing
typed return paths.

## Evidence Sources

- `reports/ai-architecture-improvements-fourth-iteration.md` §§ 4 (current
  architecture map), 5 (FOUNDATIONS matrix, FND-22A / FND-29 / FND-31
  classifications), 6 (S170 specific verdict), 14 (proof matrix row
  "learned updates affect decisions traceably"), 16 (line 282
  "unacceptable remaining risks" item on plausible-outcome goldens).
- `docs/triage/2026-05-25-ai-architecture-improvements-fourth-iteration-triage.md`
  (companion triage record for this wave; written alongside this spec).
- `docs/FOUNDATIONS.md` §22A (line 284 test), §29 (line 383 test), §29A,
  §31 (line 435 contract).
- `archive/specs/S170-learned-state-provenance-hardening.md` (predecessor
  closing the store-side gap; non-goal §2 explicitly defers the
  decision-effect trace coupling that this spec now lands).
- Current-code citations:
  `crates/worldwake-ai/src/ranking.rs:397-460, 494-510, 770-793, 5883`;
  `crates/worldwake-ai/src/decision_trace.rs:691-780, 2429-2450, 3121-3140`;
  `crates/worldwake-ai/src/goal_model.rs:2200-2225`;
  `crates/worldwake-core/src/learned_opportunity_memory.rs:5-20`;
  `crates/worldwake-core/src/testimony_reliability.rs:7-62`.
