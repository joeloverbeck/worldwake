# Implementation Order

**Status**: 📌 Held adjunct wave — authored, awaiting explicit user directive to lift the gameplay hold

The prior gameplay adjunct wave (S172 + S173, derived from the first-iteration
ChatGPT-Pro Cluster 1 report at SHA `a83cd87617a48e767c2bd53abd66117367cf4b6f`)
completed and was archived to `archive/specs/IMPLEMENTATION-ORDER-2026-05-26.md`.
This file records the **next gameplay adjunct wave**, derived from the
second-iteration Cluster 1 report at SHA `299d64c25fb45dc9ab69b295162949d8c8442606`.
The same held disposition that applies to `S60`–`S66` applies to the new specs in
this file: implementation requires an explicit user directive lifting the gameplay
hold. The author convention "all specs remain authored-but-deferred until then" is
preserved.

## Adjunct Wave: Cluster 1 Embodied Rest + Failure Cascades

**Source.** `reports/cluster-1-gameplay-mechanics-improvements-second-iteration.md` —
a ChatGPT-Pro Cluster 1 second-iteration improvement analysis at `main` SHA
`299d64c25fb45dc9ab69b295162949d8c8442606`. The author fetched files directly from
the SHA. All 12 load-bearing factual claims verified accurate against current
`main` (`SelfCareOccupancy::Sleep` unwritten, `exhaustion_collapse_ticks` unwired,
Sleep goal `AlwaysLikely`, `WashBasinState.dirtiness_level` inert, `LatrineFullness`
non-blocking, `item_decay` Waste-focused, `WakeReason::LocalDisturbance` coarse,
`SurvivalForensicExtractor` lacking failed-rest records, `SleepEpisode` carrier
present, action-trace `SelfCareInterrupted` present, scenarios named all exist).
The triage turned on benefit, not correctness: 2 of the report's 13 distinct
proposals accepted as new specs (S174 + S175); 2 deferred as future waves
(degradation consequence layer, environmental exposure); 2 folded into S174's
scope (forensics extension, rest-site memory); 1 dismissed as substrate-complete
(general contention proof, owned by S173 scenarios); 6 reaffirmed against existing
pending specs (`S60`–`S66`). Dismissals and rationale:
`docs/triage/2026-05-26-cluster-1-gameplay-mechanics-second-iteration-triage.md`.

```
S174 (Shelter, Sleep Surfaces, and Safe-Rest)         ── completed and archived; depends on archived S173 / S128 / S44 / S142 / S120
S175 (Fatigue Collapse + Failed-Rest Traceability)    ── depends on archived S174 (consumes FailedRestOpportunity records)
```

S175 reads but does not write S174's forensic state. S174 has landed without S175;
S175 now consumes the archived S174 failed-rest causal chain that the collapse
golden depends on.

### Completed

- **S174 — Shelter, Sleep Surfaces, and Safe-Rest Consequence Carrier** —
  `archive/specs/S174-shelter-sleep-surfaces-safe-rest.md` — *Status: Completed.*
  Introduces `RestCapacity` and `RestOccupancy` components on `EntityKind::Place`,
  splits the Sleep goal schema into a two-path enumerator (`KnownRestSite`
  belief-backed + `RoughSleep` always-available fallback), restructures
  `WakeReason::LocalDisturbance` with a typed `SleepFailureCause` payload,
  introduces `ActionTraceDetail::SleepInterrupted`, extends
  `SurvivalForensicExtractor` with `FailedRestOpportunity` records, and adds a
  `rough_sleep_recovery_floor` profile field. Five scenarios cover known-rest-site
  contention, multi-slot contention, structured-cause interruption, player-POV
  symmetry, and the repeated-failed-rest feed that S175 consumes.
  **FND-1/3/4/7/8/9/10/14/14A/14B/19/20/21/26/28/29/29A/31.**

### Authored, Awaiting Activation

- **S175 — Fatigue Collapse and Failed-Rest Traceability** —
  `specs/S175-fatigue-collapse-and-failed-rest-traceability.md` — *Status: Draft.*
  Wires the unimplemented `MetabolismProfile.exhaustion_collapse_ticks` profile
  field into `apply_deprivation_consequences`, adds
  `DeprivationKind::Exhaustion` as the third sibling of `Starvation` and
  `Dehydration`, extends the death-cause attribution match in `combat.rs` to map
  Exhaustion wounds to `DeathCause::NeedDeprivation { need: Fatigue }`, and
  surfaces a `CriticalWindowReport.exhaustion_collapse_observed` forensic flag.
  Three scenarios cover the collapse cascade, recovery-before-collapse dampener
  proof, and profile-field liveness. **FND-1/3/4/8/10/11/19/26/28/29/29A/31.**

## Watchlist — Deferred from Second-Iteration Triage

These proposals from the second-iteration report were verified factually
accurate but **deliberately deferred** to keep this wave focused. They are
revisited after archived S174 plus S175 prove the full rest substrate.

- **Concrete Survival Degradation Consequence Layer (water quality / food
  spoilage / latrine blocking / basin effectiveness)** — Report P0 theme C. The
  carriers exist (`PlaceDirtiness`, `LatrineFullness`, `WashBasinState`,
  `item_decay`) via archived `S129-place-dirtiness-facility-wear` and
  `S130-item-decay`. The missing consequence wiring (blocking, contamination,
  cached-food spoilage producing scarcity) is real but needs S174's rest-site
  substrate first so degradation has rest-relevant consumers (crowded shelters
  wear faster, etc.). Defer to a future adjunct wave; likely multiple
  per-domain specs.

- **Minimal Environmental Exposure** — Report P1 theme E. `ExposureSource` on
  place/edge/boundary, `ExposureState` on agent (cold/heat/wetness),
  shelter/fire/clothing mitigation. The report itself says "design now,
  implement after rest substrate"; we honor that recommendation. Defer until
  S174 ships.

- **Rest-Site Memory beyond existing route experience** — Report P1 theme F.
  Folded into S174's existing `LearnedRoutePreferences` /
  `LearnedSourcePreferences` substrate consumption (archived
  `S38-learned-route-source-preferences`); a typed
  rest-outcome-memory component is a future trigger only if scenarios reveal
  per-place rest learning is insufficient via existing carriers.

## Reaffirmed — Already Owned by Pending Specs

The second-iteration report correctly cites the following as prior art. No new
specs are needed for these themes; the pending specs already own them.

- **S60 Persistent Site Occupancy** — `specs/S60-persistent-site-occupancy.md`
- **S61 Predator Ecology and Dens** — `specs/S61-predator-ecology-dens.md`
- **S62 Boundary Processes and Remote Shocks** — `specs/S62-boundary-processes-remote-shocks.md`
- **S63 Contested Evidence and Warrants** — `specs/S63-contested-evidence-warrants.md`
- **S64 Scarcity Response — Debt, Rationing, Substitution** — `specs/S64-scarcity-response-debt-rationing.md`
- **S65 Social Aftermath Memory** — `specs/S65-social-aftermath-memory.md`
- **S66 Settlement Decline and Reoccupation** — `specs/S66-settlement-decline-reoccupation.md`

## Dismissed — Already Substrate-Complete

- **Multi-Agent Contention Proof for All Survival Affordances** — Report P1
  theme G. The substrate exists via `archive/specs/S44-generalized-contention-substrate.md`
  and `archive/specs/S173-self-care-interruption-occupancy.md`. What was missing
  is *scenario proof*, and S174's golden contract (Scenarios A, B, C) provides
  it for the sleep-surface axis. Wash/Latrine multi-agent collision proof
  remains an open scenario-authoring item but does not require a new spec.

## Dependency Notes

- S174 depends on archived `S128`, `S173`, `S44`, `S142`, `S120` substrate.
- S175 depends on S174's `FailedRestOpportunity` records and consumes the
  existing `S17`/`S81` wound-and-death substrate.
- Neither spec depends on `S60`–`S66`; the held gameplay specs may activate in
  any order relative to this wave.
