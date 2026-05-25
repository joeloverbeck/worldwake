# Triage — AI Architecture Improvements, Fourth Iteration (2026-05-25)

**Source:** `reports/ai-architecture-improvements-fourth-iteration.md`
(ChatGPT-Pro, run against current `main` SHA `e0ec83d4` — the post-S170
state). Follows the same SHA-pinned current-tree audit pipeline used for the
third iteration.

## Verdict

The report is a **stop-condition audit**, not a proposal slate. Its primary
output is the recommendation to stop AI-architecture cycles. Its §12
explicitly says: *"No new AI-architecture proposals are warranted by
current-main evidence."* The triage therefore focuses on (a) verifying the
report's factual base and (b) critically reassessing whether the recommended
"stop now" verdict and the report's "watchlist (deferred)" classification
hold up against FOUNDATIONS, especially for items the audit acknowledges as
real seams.

**Factual base: solid.** Every load-bearing CONFIRMED-as-fixed claim
verifies on current `main`. One factual inaccuracy (`BeliefStatus` enum is 5
variants, not 7; the report's audit table line lists statuses including
"unknown" and "low-confidence" that exist only via `BeliefRead::Unknown` and
permille fields). One coverage gap in the audit (`portfolio_weights_profile`
appears as unmapped in 16 scenarios, `risk_weight_profile` in 1; the audit
named only four unmapped fields). Neither inaccuracy changes the report's
verdict.

**Verdict adjustment: 1 accept, 7 dismiss/reaffirm.** I concur with the
report's "stop" recommendation on six of its seven watchlist/dismissed items
but disagree on one: the report classifies the **learned-update →
decision-trace edge** as future polish; verification against current code
shows it is a structural FND-22A / FND-29 / FND-31 gap that S170 only
half-closed (store mutations are inspectable; store reads at ranking time
are not). A single focused spec (**S171**) closes it without reopening a
larger architecture wave.

## Claim verification (independent agents on current `main`)

- **Three-provider verification registry, local-only, trace-visible**:
  CONFIRMED. `verification_provider/mod.rs:59-75` (registry + dispatch),
  `ask_witness_provider.rs:23-29` (same-place witness filter),
  `consult_record_provider.rs:27-33` (same-place record filter),
  `search_place_provider.rs:24-26` (effective-place equality), and
  `decision_trace.rs:203-207` (`verification_provider` /
  `verification_rejections` trace fields).
- **Goal-level `AskWitness` is the only `GoalKind` verification variant;
  `ConsultRecord` and `SearchPlace` are repair-seam only**: CONFIRMED.
  `goal.rs:62-183` enumerates `GoalKind` — only `AskWitness { witness,
  topic }` exists at goal level. `ConsultRecord` / `SearchPlace` are
  reachable only through `verification_need_for_breach` in
  `agent_tick/execution.rs:521-560` (repair-internal). The proactive /
  reactive asymmetry between the three carriers is real.
- **S170 provenance fields present on all five named stores and preserved
  through save/load**: CONFIRMED. Five stores verified:
  `learned_opportunity_memory.rs:5-20`, `route_preference.rs:14-21, 85-90`,
  `discrepancy.rs:68-83`, `blocker_memory.rs:212-229`,
  `testimony_reliability.rs:20-62`. Bincode roundtrip tests in each module
  confirm preservation.
- **Learned-update → decision causality "indirect"**: CONFIRMED as
  STRUCTURAL, not polish. `ranking.rs:439-460`
  (`learned_opportunity_bonus`) reads `entry.expires_tick`, returns `u32`,
  discards `entry.source` / `entry.observed_tick` / `entry.observed_at`.
  `ranking.rs:413-437` (`repair_memory_bonus`) returns `u32`, discards the
  matched `BreachSignature` and `success_count`. `goal_model.rs:2212-2218`
  (`RankedDriveGoalProvenance`) has no learned-context field.
  `decision_trace.rs` contains zero `LearnedOpportunity*` references.
  Existing precedent: `RankedGoalSummary` already carries
  `source_reliability_discount` and `competition_discount` attribution
  records for the **discount** axes; the two **bonus** axes have no
  equivalent.
- **HTN `StageHint`-only, `RequiredActionLeaf` guarded**: CONFIRMED.
  `htn/method_schema.rs:56-64`, `htn/registry.rs:110-124` (negative test
  shipped by S160).
- **Partial-plan skeleton revalidation gates execution**: CONFIRMED.
  `partial_plan_revalidation.rs:11-43`.
- **PerAgentBeliefView enforces FND-14A split**: CONFIRMED.
  `per_agent_belief_view.rs:501-592` (local-physical exception),
  `1291-1343` (self-only social), `1486-1497`
  (`intention_disposition_profile` self-only).
- **`BeliefStatus` distinguishes 7 categories**: PARTIAL. Enum is five
  variants (`Certain / Probable / Stale / Disputed / Contradicted` at
  `belief_view.rs:83-89`); the missing two ("unknown", "low-confidence")
  are represented through `BeliefRead::Unknown` and the `confidence:
  Permille` field, not as `BeliefStatus` variants. Architecturally sound;
  classification difference only.
- **Four named scenario coverage warnings classify as support fields**:
  CONFIRMED. `scenario-coverage.md:8-41`; `scenario-roadmap.md:74-81`. Two
  additional unmapped fields not named in the audit:
  `portfolio_weights_profile` (16 scenarios) and `risk_weight_profile`
  (1 scenario). Both are classification debt of the same kind, not
  behavioural gaps.

## Per-item triage

### Accepted (1 spec)

- **Watchlist item: learned-update → decision-trace edge** (audit §§ 6, 14
  "Learned updates affect decisions traceably"). **Accept** as **S171**
  (`specs/S171-learned-context-decision-trace-edge.md`).
  - *Why I disagree with the audit's deferral*: The audit characterises
    this as polish ("nicety, not a blocker") and recommends deferral until
    gameplay surfaces a concrete failure. Independent verification shows
    the gap is structural at ranking time: two bonus functions return
    `u32` only, discarding entry identity/provenance before the caller
    sees it. FND-22A's "experience path" test fails today inspectably,
    without a scenario. FND-31's "causal reason, not plausible outcome"
    contract — which the audit itself lists at §16 line 282 under
    *"Unacceptable remaining risks"* — is unenforceable in goldens
    without the trace surface S171 adds. Leaving the gap open undermines
    S170's investment: the store-mutation half of the provenance chain is
    inspectable, the store-read half is not.
  - *Scope*: Two new attribution structs, one extension to existing
    `SourceReliabilityDiscount`, two field additions to `RankedGoalSummary`,
    return-shape change on two bonus functions, observer formatter
    additions. Zero behaviour change; byte-identical rankings pre/post.
    Compiler-driven refactor, no new abstractions.
  - *FND citations*: FND-3, FND-22A, FND-26, FND-27, FND-28, FND-29,
    FND-29A, FND-31.

### Dismissed (concur with audit, no new spec)

- **Watchlist item: goal-level verification companion polymorphism**
  (audit §§ 6, 7, 8, 11). **Dismiss**. Asymmetry between proactive
  `GoalKind::AskWitness` and reactive `ConsultRecord` / `SearchPlace` is
  real and verified, but no current scenario demonstrates it produces
  wrong behaviour. Adding `GoalKind::ConsultRecord` / `GoalKind::SearchPlace`
  would expand the proactive-verification action space speculatively and
  change agent behaviour without a concrete need. Gameplay specs S60–S66
  (currently held) are the natural place for this to surface as a
  concrete failure. The audit's "leave guarded until a scenario blocks"
  stance is correct here.
- **Watchlist item: scenario coverage warning classification** (audit §§
  9, 13, 17). **Dismiss as architectural**. Routine classification of
  authored agent fields into canonical / support / fixture / obsolete
  buckets is documentation hygiene, not architecture. Add the two
  audit-missed fields (`portfolio_weights_profile`,
  `risk_weight_profile`) to the same classification queue. No spec.
- **Watchlist item: diagnostics-as-CI-gate** (audit §§ 8, 9, 11).
  **Dismiss**. `scripts/verify.sh` already gates the necessary surfaces.
  A broad-dashboard CI gate is churn without a current-main defect.
  Matches the second- and third-iteration triage dismissals of the same
  proposal class.
- **Watchlist item: candidate / opportunity emitter+compiler
  unification** (audit § 8). **Dismiss**. No concrete mismatch proven;
  parallel paths are functioning. Premature unification.
- **Watchlist item: method-required HTN promotion** (audit §§ 7, 8).
  **Dismiss**. Correct gating per FND-31; needs dedicated proof suite
  first.
- **"Do not do" list: runtime LLM agents / RL training / global manager
  AI / hidden story rails** (audit §§ 3, 13, 19). **Dismiss as
  out-of-scope and FOUNDATIONS-incompatible**. Concur fully with audit;
  not candidates for any spec.
- **"Do not do" list: re-opening S165–S170 or S60–S66** (audit §§ 3, 13,
  19). **Dismiss**. Concur. S165–S170 are landed and verified; S60–S66
  remain held per standing user directive.

### Reaffirmed (no new spec; existing work covers)

- **Audit's core "stop AI-architecture cycles" recommendation**. **Mostly
  concur, with S171 as the lone exception.** After S171 lands, the
  AI-architecture queue can honestly close.

## Sibling-iteration cross-reference

- Second-iteration triage
  (`docs/triage/2026-05-22-ai-architecture-improvements-second-iteration-triage.md`)
  set up the "act better under uncertainty" frame and accepted S165–S168.
- Third-iteration triage
  (`docs/triage/2026-05-25-ai-architecture-improvements-third-iteration-triage.md`)
  extended the verification axis (S169) and closed store-side provenance
  gaps (S170). It explicitly deferred decision-effect trace coupling as a
  S170 non-goal pending future evidence.
- This (fourth-iteration) triage takes up that deferred work as S171, on
  the basis that the FND test failure is now inspectable from current
  code without waiting for a scenario.

## Follow-up work not actioned

- Scenario coverage warning classification (six fields total). Owner: doc
  hygiene queue. No spec.
- Goal-level verification companion polymorphism. Owner: deferred until
  S60–S66 (or a future gameplay scenario) surfaces a concrete blocker.
