# Triage — Cluster 1 Gameplay Mechanics, First Iteration (2026-05-25)

**Source:** `reports/cluster-1-gameplay-mechanics-improvements-first-iteration.md`
(ChatGPT-Pro, 739 lines, at `main` SHA `a83cd87617a48e767c2bd53abd66117367cf4b6f`).
The author fetched files directly from the SHA rather than relying on stale code
search.

## Verdict

All 16 load-bearing factual claims I tested verified accurate against current
`main` (Wash budget-exhaustion exclusion in `survival-scattered`/`survival-contested`;
`abort_noop` on every needs action except sleep; `promotable_contention_kind`
recognizes Harvest/Craft/Corpse/Care but no needs actions; `emit_wash_goal` exists;
`GoalPlanningBudget::SELF_CARE` exists and Wash uses it; both forensic tests exist;
`survival-drive-escalation` co-locates Wash with everything else). The report aligns
exactly with the project's own `docs/gameplay-mechanic-deepening-roadmap.md` lines
159–170, which already names these gaps as "Not Yet Proven Enough" and calls the
Wash budget-exhaustion exclusion "a first-class deepening target."

Triage turned on **structure**, not correctness. The report recommended one combined
spec with two slices; this triage **split into two specs** to match project
per-spec-per-concern convention (S60–S66, S165–S168) and to give each a distinct
blast radius. Six of the report's P0/P1 proposals were accepted; one (P1.3 recovery
memory blockers) was deferred; one (P1.4 player UI legibility) was narrowed to a
single per-spec assertion since `S158`/`S162`/`S163` already cover the architecture.

## Accepted

- **`archive/specs/S172-wash-discovery-budget-closure.md`** (P0.1, P0.5 Wash branch, P1.4
  narrowed) — closes the known Wash budget-exhaustion exclusion in
  `survival-scattered`/`survival-contested`; pins the source-class table for the
  Wash candidate path under FND-14B; pins the four lawful Wash decision-trace
  branches; generalizes the belief-only Wash regression beyond drive-escalation;
  adds one CLI POV assertion against remote-basin-state leak. No new authoritative
  state. **FND-3/7/8/14/14A/14B/16/19/26/29A/31.**
- **`archive/specs/S173-self-care-interruption-occupancy.md`** (P0.2, P0.3, P0.4, P0.5
  remaining branches, P1.1 subsumed, P1.2 in-spec config) — defines per-action-family
  interruption contracts; introduces `SelfCareOccupancy` on `WashBasin` and latrine
  `Place`; replaces `abort_noop` with explicit abort handlers for the five
  cluster-1 actions; extends `promotable_contention_kind` to classify Wash and
  Toilet as exclusive use (reuses S44/S142 substrate, no parallel queue); proves
  repeated-interruption deprivation collapse end-to-end via Scenario E.
  **FND-1/3/4/8/9/10/11/19/21/26/28/29/29A/31.**

Both specs sit in a **held** adjunct wave (`specs/IMPLEMENTATION-ORDER.md`)
alongside the held `S60`–`S66` gameplay specs. Activation requires an explicit
user directive lifting the gameplay hold; the prior AI-architecture wave's
exclusion of gameplay specs is preserved here.

## Dismissed / deferred

- **Report's "one combined spec, two slices" recommendation** — split into two
  specs (S172, S173) to match project convention and give each spec a clean
  per-concern blast radius. The same architectural principle binds them, but
  Slice 1 is planner+scenario and Slice 2 touches the action framework, occupancy
  state, and a wider scenario surface — different reviewers, different test
  surfaces.
- **P1.3 Recovery memory for interrupted self-care** — deferred. Agents replan
  from current observation each tick; revalidation already rejects disconfirmed
  basins. A typed memory or trace-derived blocker is a worthwhile P2 once the
  first collision proof lands, but not load-bearing for it.
- **P2.1 Disease / infection / odor / social shame / privacy / etiquette /
  bathroom politics** — deferred per the report's own recommendation.
- **P2.2 Complex sanitation economy** — deferred per the report's own
  recommendation.
- **P2.3 Full shelter redesign** — deferred per the report's own
  recommendation; `SleepSurface`/`SleepSlot` scarcity remains a future spec
  trigger if a scenario proves it matters.
- **P2.4 Full adjacent-cluster redesign (pursuit, obligation, trade, theft,
  justice, combat, escort)** — used in S173 only as pressure sources for
  interruption; redesign of any of these clusters is out of scope.
- **`WashSessionProgress` duration-based partial-Wash carrier** (Section 7
  alternative in the report) — deferred per the report's own recommendation
  ("partial Wash can be interesting, but without a durable session/progress
  carrier it becomes invisible arithmetic").
- **Patience-threshold negotiation / social-rank arbitration / queue-jumping
  policy** (P1.2 elaborations) — first pass uses FCFS via existing S44 grant
  expiry; P1.2 is honored as in-spec profile config, not as a separate policy
  system.

## Reaffirmed (already addressed elsewhere)

- **`archive/specs/S128-sleep-episode-place-quality.md`** — Sleep interruption
  contract (`SleepEpisode` + `WakeReason::LocalDisturbance`) is the precedent S173
  mirrors for Wash and Toilet. No re-litigation.
- **`archive/specs/S129-place-dirtiness-facility-wear.md`** — `WashBasinState`
  per-facility clean-water and dirtiness state already exists and is the input
  S172 audits.
- **`archive/specs/S44-generalized-contention-substrate.md`** +
  **`archive/specs/S142-contention-event-inspectability.md`** — the contention
  substrate S173 extends (not replaces). No parallel queue.
- **`archive/specs/S81-golden-gaps-simulation-remediation.md`** +
  **`archive/specs/S17-wound-lifecycle-golden-suites.md`** — deprivation death
  via `DeathCause::NeedDeprivation` and consolidating deprivation wounds already
  proven for sustained hunger. S173 Scenario E adds the repeated-interruption
  variant on top of this existing substrate.
- **`archive/specs/S158-belief-view-remote-truth-leak-closure.md`** +
  **`archive/specs/S162-belief-view-source-gate-hardening.md`** +
  **`archive/specs/S163-cli-player-pov-boundary.md`** — the player-POV belief-view
  architecture S172/S173 use for their single CLI assertion. No new accessor needed.
- **`archive/tickets/S116DRIESCSUS-009.md`** — the archived ticket that recorded
  the Wash budget-exhaustion exclusion as a known live issue. S172 closes it.

## Follow-ups identified, not actioned

- **P1.3 recovery memory blockers** — re-evaluate after S173 lands. If repeated
  retry of disconfirmed basins shows up as decision-trace noise, a typed
  blocker memory becomes worthwhile.
- **Sleep-surface scarcity** — re-evaluate if a scenario reveals place-capacity
  is insufficient for sleep contention. Would extend `SelfCareUseKind` and
  introduce `SleepSurface`.
- **Wilderness-relief scarce affordances** — currently location-flexible. If a
  future scenario models specific scarce relief spots (privacy, hidden cover),
  classification extends similarly.
- **Adjacent-cluster pressure scenarios** — once S172/S173 land, a scenario
  exercising trade/pursuit/obligation as interruption sources would prove the
  loop under realistic adjacent-cluster load. Out of scope for this iteration.
