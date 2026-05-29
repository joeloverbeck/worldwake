# Implementation Order

**Status**: ACTIVE

The prior gameplay adjunct wave (S174 + S175, derived from the second-iteration
ChatGPT-Pro Cluster 1 report) completed and was archived to
`archive/specs/IMPLEMENTATION-ORDER-2026-05-28.md`. This file records the **next
gameplay adjunct wave**, derived from the third-iteration Cluster 1 report.

The same held disposition that applies to `S60`–`S66` applies to the new **specs**
in this file: implementation requires an explicit user directive lifting the
gameplay hold. The author convention "all specs remain authored-but-deferred until
then" is preserved. The **ticket** in this wave (S175CIOWN-001) was a
proof-integrity correction, **not** gameplay deepening, and was therefore
action-eligible regardless of the gameplay hold.

## Adjunct Wave: Cluster 1 Material Degradation and Source Reliability

**Source.** `reports/cluster-1-gameplay-mechanics-improvements-third-iteration.md` —
a ChatGPT-Pro Cluster 1 third-iteration improvement analysis. The report's intended
commit `cef985cf521e5715af4a7784b3b0cfe59cc39a68` matches current `main` HEAD
(merge of PR #138, "Implemented spec S175"). The report flagged that it could not
itself verify live `main`; this triage re-verified all load-bearing claims directly
against the working tree.

**Verification.** Essentially all of the report's load-bearing factual claims
verified accurate against current `main`:

- Before S175CIOWN-001, the two S175 exhaustion goldens were `#[ignore]` with messages
  claiming they "run via golden-survival workflow," but `golden-survival.yml`'s matrix
  did not include them and no workflow ran them; `docs/scenario-roadmap.md` §5.19
  repeated the false CI-ownership claim (the report's P0).
- `LotOperation::Spoiled` exists in the lineage enum but is **never constructed**
  anywhere (schema-only).
- `ResourceSource` has no quality/contamination field; `ItemLot` has no per-lot
  freshness/condition field.
- `WashBasinState.dirtiness_level` is incremented per wash but never read to gate
  effectiveness/legality; `LatrineFullness.fill` raises place dirtiness past
  `critical_threshold` but never blocks the Toilet precondition.
- The full carrier set exists (`ResourceSource`, `WashBasinState`, `LatrineFullness`,
  `PlaceDirtiness`, `RestCapacity`/`RestOccupancy`, `SleepEpisode`/`SleepQualityProfile`,
  five `HomeostaticNeedId`s, `SurvivalForensicExtractor`, item-decay basin refill from
  a colocated water source).

**Triage decision.** The report's central thrust — the "Concrete Survival
Degradation Consequence Layer" — is **exactly** what the 2026-05-26 second-iteration
triage explicitly **deferred** ("premature… needs S174's rest substrate as the
meaningful consumer. Defer to a follow-up wave; likely multiple per-domain specs at
that time."). **S174 has now landed and archived**, so that deferred wave is ripe.
Per the user directive, specs `S60`–`S66` are **excluded** from this iteration
("old specs that will likely be absorbed along the way"), which removes the report's
scarcity-response (→ S64) and flight/vacancy (→ S66) candidates from scope. Of the
report's proposals: 3 accepted as new per-domain degradation specs; 1 P0 accepted
as an action-eligible proof-integrity ticket; scarcity-response and
flight/abandonment reaffirmed against held `S64`/`S66` (excluded per directive);
predator (S61) and boundary shocks (S62) deferred/excluded; minimal exposure
remains a watchlist item per the report's own recommendation; the requested 1440-tick
collision proofs are folded into each spec's FND-31 validation rather than a separate
proof spec. Dismissals, reaffirmations, and per-item rationale:
`docs/triage/2026-05-29-cluster-1-gameplay-mechanics-third-iteration-triage.md`.

```
S175CIOWN-001 (Exhaustion golden CI ownership)   ── archived ticket; completed; no spec deps
S176 (Sanitation Facility Degradation)           ── ✅ COMPLETED 2026-05-29 (archived); depended on archived S129 / S173 / S174 / S44 / S82; wired dead facility state into consequences
S177 (Water Quality, Depletion, Reliability)     ── depends on archived S79 / S38 / S151 / S129; couples to S176 (basin refill quality) ; realizes canonical scenario D for water
S178 (Perishable Food Spoilage)                  ── depends on archived S82 / S79; first emitter of LotOperation::Spoiled
```

S176 was the recommended first slice (the report's §17 pick) and is now
**completed and archived** (tickets S176SANFACDEG-001..008, 2026-05-29). S177
and S178 are independent of each other; S177 couples to S176 only through
basin-refill water quality (it consumes the now-live `WashBasinState`). Neither
remaining spec depends on the other's completion.

### Completed Proof-Integrity Ticket (not held)

- **S175CIOWN-001 — Exhaustion golden workflow ownership** *(ticket)* —
  `archive/tickets/S175CIOWN-001-exhaustion-golden-workflow-ownership.md` — *Status: COMPLETED.*
  Added the two S175 exhaustion filters to `golden-survival.yml`'s matrix and
  corrected the false CI-ownership claim in `docs/scenario-roadmap.md` §5.19. CI +
  docs only; no engine change. Proof-integrity correction, exempt from the gameplay
  hold.

### Completed (this wave)

- **S176 — Sanitation Facility Degradation Consequences** — *Status: ✅ COMPLETED 2026-05-29.*
  `archive/specs/S176-sanitation-facility-degradation-consequences.md` (tickets
  `archive/tickets/S176SANFACDEG-001..008`). Wired the inert
  `WashBasinState.dirtiness_level` into wash effectiveness + legality and the
  inert `LatrineFullness.fill` into a Toilet precondition gate; added
  `clean_wash_basin` / `empty_latrine` maintenance actions (duration, occupancy,
  Waste aftermath) inserted by the GOAP search as `Wash`/`Relieve` prerequisites;
  extended `SurvivalForensicExtractor` with `DegradedSelfCareOpportunity`. No new
  ECS component, no new `GoalKind`. Landed with focused goldens
  (`survival-basin-dirty-dirty`, `survival-latrine-full`) plus the CI-owned
  1440-tick `survival-sanitation-breakdown-1440` collision scenario.

### Authored, Awaiting Activation

- **S177 — Water Source Quality, Depletion Observation, and Reliability Memory** —
  `specs/S177-water-source-quality-depletion-reliability.md` — *Status: Draft.*
  Adds `WaterQuality` (`Clean`/`Stale`/`Muddy`) to water sources with thirst/dirtiness
  and basin-refill consequences; adds belief-backed source-reliability memory
  (extends the S38 learned-source substrate) so agents discover depletion locally and
  prefer fallbacks — realizing **canonical regression scenario D** for water.
  Explicitly **defers** any unsafe-water sickness/wound (no disease ecology). Focused
  goldens (`survival-water-source-depleted`, `survival-dirty-water-tradeoff`) plus a
  1440-tick `survival-degrading-water-1440` collision scenario.
  **FND-1/3/4/7/14/14A/14B/15/16/17/19/21/22A/26/28/29/29A/31.**

- **S178 — Perishable Food Spoilage and Lot Condition** —
  `specs/S178-perishable-food-spoilage.md` — *Status: Draft.*
  Adds per-lot `PerishableState` (Fresh → Stale → Spoiled) with condition-scaled Eat
  relief, **first emission of `LotOperation::Spoiled`**, spoiled-but-edible
  desperation gating (per-agent profile threshold), and cache spoilage feeding the
  belief/fallback loop. Explicitly **defers** any food-borne sickness/wound. Focused
  goldens (`survival-food-spoilage-lifecycle`, `survival-food-spoilage-cache`) plus a
  1440-tick `survival-food-spoilage-cache-1440` collision scenario.
  **FND-1/3/4/5/7/10/11/14A/14B/16/17/19/22/26/28/29/29A/31.**

### Watchlist (designed-later, not yet a spec)

- **Minimal environmental exposure carrier** (cold/heat/wetness; shelter/fire/clothing
  mitigation; `ExposureWound`) — re-evaluate **after** the degradation wave lands.
  Deferred per the report's own recommendation ("design now, implement after
  degradation and scarcity-response prove themselves") and the second-iteration
  triage. Becomes a spec when rest-site quality composition is stable and a scenario
  proves shelter must matter beyond sleep recovery.
- **Rest/exhaustion 1440-tick collision proof under adjacent-cluster load** —
  a `survival-rest-scarcity-collapse-1440` scenario exercising S174/S175 against
  travel/danger/obligation. Open scenario-authoring item; becomes warranted once the
  degradation collision scenarios above establish the long-run harness pattern.

### Excluded This Iteration (per user directive: S60–S66 absorbed later)

- Scarcity response (refusal, rationing, debt, aid, survival theft) → held
  `specs/S64-scarcity-response-debt-rationing.md`.
- Flight, facility vacancy, abandonment, settlement decline → held
  `specs/S66-settlement-decline-reoccupation.md`.
- Predator / night-danger ecology → held `specs/S61-predator-ecology-dens.md`.
- Boundary shocks / external supply failure → held
  `specs/S62-boundary-processes-remote-shocks.md` (internal degradation creates
  scarcity first; S62 does not gate this wave).
