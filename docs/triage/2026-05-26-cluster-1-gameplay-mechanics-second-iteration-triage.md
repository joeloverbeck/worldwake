# Triage — Cluster 1 Gameplay Mechanics, Second Iteration (2026-05-26)

**Source:** `reports/cluster-1-gameplay-mechanics-improvements-second-iteration.md`
(ChatGPT-Pro, 1352 lines, at `main` SHA `299d64c25fb45dc9ab69b295162949d8c8442606`).
The author fetched files directly from the SHA rather than relying on stale code
search or the project's uploaded manifest (which the report itself flagged as
unreliable evidence). This is the follow-up iteration to the 2026-05-25
first-iteration triage that produced S172 + S173 (now archived).

## Verdict

All 12 load-bearing factual claims I tested verified accurate against current
`main` (`SelfCareOccupancy::Sleep` enum case exists but the sleep action does not
write it; `MetabolismProfile.exhaustion_collapse_ticks` is a profile field with no
consumer; `goal_schema.rs::DECL_SLEEP` uses `FeasibilityStrategy::AlwaysLikely`;
`WashBasinState.dirtiness_level` is incremented but never read for any gating;
`LatrineFullness` raises place dirtiness past a threshold but does not block toilet
preconditions; `item_decay` archives ground items, not stored caches;
`apply_deprivation_consequences` has branches for hunger/thirst/bladder but no
fatigue; `SleepEpisode` carries the full duration/recovery/wake-condition shape
described; `WakeReason::LocalDisturbance` is a bare variant with no structured
cause; `SurvivalForensicExtractor.CriticalWindowFrame` captures exhaustion state
and blockers but no failed-rest records; `ActionTraceDetail::SelfCareInterrupted`
exists as described; all referenced scenarios exist).

Triage turned on **benefit and prematurity**, not correctness. Two of the
report's P0 proposals were accepted as new specs (S174 sleep/shelter, S175
fatigue collapse). Two P0 proposals were deferred to future waves (concrete
degradation consequence layer, environmental exposure) because they need S174's
rest substrate as their meaningful consumer. Two P0/P1 proposals were folded
into S174's scope rather than treated as separate specs (forensics extension,
rest-site memory). One P1 proposal was dismissed as substrate-complete via S44 +
S173 (multi-agent contention; what's missing is scenario authoring, not a new
spec). Six P2/P3 proposals were reaffirmed against pending S60–S66.

## Critical reassessment of the report

The report's diagnosis is accurate but its proposal surface is slightly over-broad
for the actual delta:

- **Survival Failure Forensics (P0 D)** is presented as a sibling of the sleep
  and fatigue P0 themes, but the specific asks (failed-recovery records,
  structured wake reasons, source/facility failure classifications) are
  extensions of `S120-survival-critical-window-forensics` and properly belong in
  S174's trace requirements. Treating it as a separate spec inflates the wave
  artificially.
- **Multi-Agent Contention for All Survival Affordances (P1 G)** is largely
  substrate-complete. S44 + S173 provide the contention queue and self-care
  occupancy. What's missing is scenario proof — and that's part of S174's
  golden contract for sleep-surface contention. Wash/Latrine multi-agent
  collision proof remains an open scenario-authoring item but is not a spec.
- **Concrete Survival Degradation (P0 C)** is real but premature. The carriers
  exist via archived S129 and S130. The missing consequence wiring (basin
  effectiveness, latrine blocking, food spoilage) needs S174's rest substrate
  as the meaningful consumer — a crowded shelter degrades faster, dirty basins
  matter more when other rest options are scarce. S174 was the first landing
  prerequisite.
- **The S60–S66 reaffirmation breadth** correctly identifies pending prior art
  but creates an impression of wide gaps. The actual delta from the current
  state is narrower: two new specs (S174 + S175) and a watchlist.

The report's two core architectural insights — sleep is the weakest major
self-care family, and `exhaustion_collapse_ticks` is an unwired profile field —
are both correct and motivate the accepted specs.

## Accepted

- **`archive/specs/S174-shelter-sleep-surfaces-safe-rest.md`** (P0 A, P0 D folded in,
  P1 F folded in, P1 G subsumed via scenarios) — introduces `RestCapacity` +
  `RestOccupancy` components on `EntityKind::Place`, splits Sleep goal schema
  into `KnownRestSite` belief-backed + `RoughSleep` AlwaysLikely fallback,
  restructures `WakeReason::LocalDisturbance` with typed `SleepFailureCause`
  payload, adds `ActionTraceDetail::SleepInterrupted`, extends
  `SurvivalForensicExtractor` with `FailedRestOpportunity` records, adds
  `rough_sleep_recovery_floor` profile field. Five scenarios cover known-rest
  contention, multi-slot contention, structured-cause interruption, player-POV
  symmetry, and the feed for S175. **FND-1/3/4/7/8/9/10/14/14A/14B/19/20/21/26/28/29/29A/31.**

- **`specs/S175-fatigue-collapse-and-failed-rest-traceability.md`** (P0 B) —
  wires the unimplemented `MetabolismProfile.exhaustion_collapse_ticks` into
  `apply_deprivation_consequences`, adds `DeprivationKind::Exhaustion` as the
  third sibling of `Starvation` and `Dehydration`, extends death-cause
  attribution to map Exhaustion wounds to
  `DeathCause::NeedDeprivation { need: Fatigue }`, adds
  `CriticalWindowReport.exhaustion_collapse_observed` forensic flag. Three
  scenarios cover collapse cascade, recovery-before-collapse dampener, and
  profile-field liveness. **FND-1/3/4/8/10/11/19/26/28/29/29A/31.**

Both specs sit in a **held** adjunct wave (`specs/IMPLEMENTATION-ORDER.md`)
alongside the held `S60`–`S66` gameplay specs. Activation requires an explicit
user directive lifting the gameplay hold; the prior wave's exclusion of
gameplay specs is preserved.

## Deferred to future waves

- **Concrete Survival Degradation Consequence Layer** (Report P0 C: water
  quality / food spoilage / latrine blocking / basin effectiveness) — verified
  factually accurate but premature. The carriers exist via archived S129 +
  S130; the missing consequence wiring needs S174's rest substrate as the
  meaningful consumer. Defer to a follow-up wave; likely multiple per-domain
  specs at that time.
- **Minimal Environmental Exposure** (Report P1 E: `ExposureSource`,
  `ExposureState`, shelter/fire/clothing mitigation) — deferred per the
  report's own recommendation ("design now, implement after rest substrate
  exists"). Watchlist item in `IMPLEMENTATION-ORDER.md`.

## Folded into S174 scope

- **Survival Failure Forensics extension** (Report P0 D: failed recovery
  records, structured wake reasons, source/facility failure classification) —
  extends `S120-survival-critical-window-forensics` rather than introducing a
  parallel spec. The specific deliverables (`FailedRestOpportunity`,
  `WakeReason::LocalDisturbance { cause }`, `ActionTraceDetail::SleepInterrupted`)
  live in S174's D7 + D8.
- **Rest-Site Memory and Safe-Route Preference** (Report P1 F) — partly covered
  by archived `S38-learned-route-source-preferences` route experience and
  `S151-testimony-reliability-and-route-preferences`; per-place rest-outcome
  learning beyond that is a future trigger only if scenarios prove the existing
  substrate is insufficient. S174 explicitly does not introduce a new memory
  component.

## Dismissed

- **Multi-Agent Contention for All Survival Affordances** (Report P1 G:
  Wash/Latrine collision, sleep-surface contention) — substrate is complete via
  `archive/specs/S44-generalized-contention-substrate.md` +
  `archive/specs/S173-self-care-interruption-occupancy.md`. What's missing is
  *scenario proof*, and S174's golden contract (Scenarios A, B, C) covers the
  sleep-surface axis. Wash/Latrine multi-agent collision proof is an open
  scenario-authoring item but does not require a new spec.
- **Report's "S174 should be one focused spec, S175 follows or pairs"
  recommendation** — partly honored: split into two specs to match per-spec
  per-concern convention (S172/S173 precedent), but treated as a paired wave
  with explicit cross-dependence rather than as a single bundle. S175 reads
  S174's forensic records; both ship together to make the collapse golden
  meaningful.

## Reaffirmed — already addressed elsewhere

- **`specs/S60-persistent-site-occupancy.md`** (Draft, held) — Persistent Camps
  and Sites (Report P2 K).
- **`specs/S61-predator-ecology-dens.md`** (Draft, held) — Predator / Night
  Danger Ecology (Report P3 L).
- **`specs/S62-boundary-processes-remote-shocks.md`** (Draft, held) — Boundary
  Shocks: failed shipments, refugee pressure, drought (Report P3 M).
- **`specs/S63-contested-evidence-warrants.md`** (Draft, held) — adjacent
  justice substrate for survival theft / false accusation (Report adjacent
  cluster seam).
- **`specs/S64-scarcity-response-debt-rationing.md`** (Draft, held) — Scarcity
  Response: Rationing, Debt, Hoarding, Refusal (Report P2 H).
- **`specs/S65-social-aftermath-memory.md`** (Draft, held) — Social Aftermath
  of Survival Failure (Report P2 I).
- **`specs/S66-settlement-decline-reoccupation.md`** (Draft, held) — Facility
  Closure, Flight, Vacancy, Reoccupation (Report P2 J).
- **`archive/specs/S128-sleep-episode-place-quality.md`** (Completed) —
  `SleepEpisode`, `SleepQualityProfile`, `ShelterTag`, `WakeReason` substrate
  that S174 extends. No re-litigation.
- **`archive/specs/S129-place-dirtiness-facility-wear.md`** (Completed) —
  `PlaceDirtiness`, `LatrineFullness`, `WashBasinState` carriers the deferred
  degradation wave will eventually consume.
- **`archive/specs/S44-generalized-contention-substrate.md`** +
  **`archive/specs/S142-contention-event-inspectability.md`** (Completed) —
  contention substrate S174 extends with `PromotableContentionKind::RestSite`.
- **`archive/specs/S173-self-care-interruption-occupancy.md`** (Completed) —
  `SelfCareOccupancy` precedent S174 mirrors with multi-occupant
  `RestOccupancy`; the S173 Non-Goal explicitly deferred sleep-surface scarcity
  to a future spec, which is S174.
- **`archive/specs/S120-survival-critical-window-forensics.md`** (Completed) —
  `SurvivalForensicExtractor` framework S174 extends with
  `FailedRestOpportunity` records.
- **`archive/specs/S17-wound-lifecycle-golden-suites.md`** +
  **`archive/specs/S81-golden-gaps-simulation-remediation.md`** (Completed) —
  wound and death substrate S175 reuses with the new `DeprivationKind::Exhaustion`
  variant.
- **`archive/specs/S38-learned-route-source-preferences.md`** +
  **`archive/specs/S151-testimony-reliability-and-route-preferences.md`**
  (Completed) — route-experience substrate that subsumes the report's
  rest-site memory ask without a new component.

## Follow-ups identified, not actioned

- **Degradation consequence layer** (water quality / food spoilage / latrine
  blocking / basin effectiveness) — re-evaluate after S174 archival. The carriers
  exist; what needs to be designed is the consequence wiring and its
  interaction with rest-site degradation.
- **Environmental exposure carrier** — re-evaluate after S174 archival. The
  report's preferred minimal model (cold/heat/wetness, shelter/fire/clothing
  mitigation) is sound but cannot be designed until rest-site quality
  composition is stable.
- **Wash/Latrine multi-agent collision scenario** — open scenario-authoring
  item, not a spec trigger.
- **Settlement-decline scenario coupling** — once S174 + S175 + the deferred
  degradation wave land, an integrated scenario exercising S66 (settlement
  decline) against actual rest-failure / fatigue-collapse pressure is
  warranted. Out of scope for this wave.
