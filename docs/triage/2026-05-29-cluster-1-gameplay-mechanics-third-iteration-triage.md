# Triage — Cluster 1 Gameplay Mechanics, Third Iteration (2026-05-29)

**Source:** `reports/cluster-1-gameplay-mechanics-improvements-third-iteration.md`
(ChatGPT-Pro). The report's intended commit
`cef985cf521e5715af4a7784b3b0cfe59cc39a68` matches current `main` HEAD (merge of
PR #138, "Implemented spec S175"). The report flagged that its GitHub connector
misrouted and it could not itself verify live `main`; this triage re-verified all
load-bearing claims directly against the working tree. This is the follow-up to the
2026-05-25 (S172+S173) and 2026-05-26 (S174+S175) Cluster 1 triages, all of which
are now implemented and archived.

## Verdict

Triage turned on **prematurity-now-resolved and scope discipline**, not
correctness. Essentially every load-bearing factual claim verified accurate.
The decisive finding: the report's central thrust — the "Concrete Survival
Degradation Consequence Layer" — is **exactly** what the 2026-05-26 triage
explicitly **deferred** to "a follow-up wave once S174 lands, likely multiple
per-domain specs." S174 has now landed and archived, so the deferred wave is ripe.
The report is, in effect, the second iteration's deferred follow-up arriving on
schedule.

Per the user directive, specs `S60`–`S66` are **excluded** ("old specs that will
likely be absorbed along the way"), which removes the report's scarcity-response
(→ S64) and flight/vacancy (→ S66) candidates from new-spec scope.

Outcome: **3 accepted as new per-domain degradation specs** (S176, S177, S178);
**1 P0 accepted as an action-eligible proof-integrity ticket** (S175CIOWN-001);
the 1440-tick collision proofs **folded** into each spec's FND-31 validation rather
than a separate proof spec; scarcity-response and flight **reaffirmed against held
S64/S66** (excluded per directive); predator (S61) and boundary shocks (S62)
deferred/excluded; minimal exposure kept on the **watchlist** per the report's own
recommendation.

## Claim verification (re-verified against current `main`)

| Claim | Verdict |
|-------|---------|
| Two S175 goldens `#[ignore]`, registered, with "run via golden-survival workflow" messages | CONFIRMED |
| `golden-survival.yml` matrix omits both S175 filters; **no** workflow runs them | CONFIRMED |
| `docs/scenario-roadmap.md` §5.19 (~line 738) repeats the false CI-ownership claim | CONFIRMED |
| `LotOperation::Spoiled` exists but is **never constructed** (schema-only) | CONFIRMED |
| `ResourceSource` has no quality field; `ItemLot` has no freshness/condition field | CONFIRMED |
| `WashBasinState.dirtiness_level` incremented but never read for gating | CONFIRMED |
| `LatrineFullness.fill` raises place dirtiness past threshold but never blocks the Toilet precondition | CONFIRMED |
| Carrier set exists (`ResourceSource`, `WashBasinState`, `LatrineFullness`, `PlaceDirtiness`, `RestCapacity/Occupancy`, `SleepEpisode/QualityProfile`, 5 needs, survival forensics, item-decay basin refill) | CONFIRMED |

## Critical reassessment of the report

The diagnosis is accurate; two report proposals were **narrowed** on FOUNDATIONS
grounds before acceptance:

- **Unsafe-water sickness (report §10, §16)** — the report floats "drinking unsafe
  water causes later consequence." S177 implements only immediate concrete effects
  (reduced thirst relief, raised dirtiness, basin-refill quality preference) and
  **defers any disease/wound path**. An illness carrier is new conceptual surface
  that FND-5 and the report's own MUST-NOT bar until it is a proven concrete carrier
  with source, trace, recovery, and proof. S177 therefore omits the `Unsafe`/
  `Contaminated` tiers and uses only `Clean`/`Stale`/`Muddy` as utility tiers.
- **Food-borne sickness (report §10)** — same discipline. S178 makes spoilage a
  value/lineage axis (reduced relief, spoiled-but-edible, `LotOperation::Spoiled`
  lineage), **not** a health axis. `FoodSickness`/`DigestiveDistress` is deferred.

A third structural choice: the report asks for both focused goldens **and**
standalone 1440-tick collision scenarios, and floats a separate "Rest and Exhaustion
Collision Maturation" proof theme. Standalone proof specs carry no new state and
risk a content-free spec; the 1440 collision scenarios are instead **folded into
each degradation spec's FND-31 validation**, and the rest-scarcity 1440 scenario is
recorded as a watchlist scenario-authoring item.

## Accepted — new specs (held adjunct wave)

- **`specs/S176-sanitation-facility-degradation-consequences.md`** (report §10
  basin/latrine, §12 degradation-as-pressure) — wires inert
  `WashBasinState.dirtiness_level` into wash effectiveness + legality and inert
  `LatrineFullness.fill` into a Toilet precondition gate; adds `clean_wash_basin` /
  `empty_latrine` maintenance actions; extends forensics with
  `DegradedSelfCareOpportunity`. No new ECS component — lowest new surface, the
  report's §17 first-slice pick.
- **`archive/specs/S177-water-source-quality-depletion-reliability.md`** (report §10 water,
  §13 degrading-water scenario) — adds `WaterQuality` to water sources +
  belief-backed source-reliability memory; realizes canonical regression scenario D
  for water. Disease consequence deferred (see reassessment).
- **`archive/specs/S178-perishable-food-spoilage.md`** (report §10 food spoilage) — adds
  per-lot `PerishableState`, first emission of `LotOperation::Spoiled`,
  condition-scaled Eat, profile-gated desperation eating, cache spoilage. Disease
  consequence deferred (see reassessment).

## Accepted — archived ticket (proof-integrity correction)

- **`archive/tickets/S175CIOWN-001-exhaustion-golden-workflow-ownership.md`** (report P0,
  §3, §13) — added the two S175 exhaustion filters to `golden-survival.yml` and
  corrected the false CI-ownership claim in `docs/scenario-roadmap.md` §5.19. CI +
  docs only; exempted from the gameplay hold. A proof-integrity correction, not
  gameplay deepening.

## Reaffirmed — already owned by held pending specs (excluded per user directive)

- **Scarcity response: refusal, rationing, debt, aid, survival theft** (report
  Candidate 2, §12) → `specs/S64-scarcity-response-debt-rationing.md` (held).
- **Flight, facility vacancy, abandonment, settlement decline** (report Candidate 5,
  §12) → `specs/S66-settlement-decline-reoccupation.md` (held).
- **Predator / night-danger ecology** (report P3) →
  `specs/S61-predator-ecology-dens.md` (held).
- **Boundary shocks / external supply failure** (report P3, §12) →
  `specs/S62-boundary-processes-remote-shocks.md` (held). Internal degradation
  (S176–S178) creates scarcity first; S62 does not gate this wave.

## Deferred — watchlist (designed-later, not yet a spec)

- **Minimal environmental exposure carrier** (report P2, §11) — re-evaluate after
  the degradation wave lands. Deferred per the report's own recommendation and the
  second-iteration triage. Recorded on the `specs/IMPLEMENTATION-ORDER.md`
  watchlist.
- **Rest/exhaustion 1440-tick collision proof under adjacent-cluster load** (report
  Candidate 3, §13 `survival-rest-scarcity-collapse-1440`) — open scenario-authoring
  item; not a spec. Becomes warranted once the degradation collision scenarios
  establish the long-run harness pattern. Recorded on the watchlist.

## Dismissed (structural, not benefit)

- **The report's "one broad spec split into 2–3 slices later" framing** (§9
  Candidate 1, §17) — split into three per-domain specs (S176/S177/S178) to match
  the per-spec-per-concern convention both prior Cluster 1 triages invoked, giving
  each a distinct blast radius (sanitation wiring vs. source-quality+belief vs.
  per-lot food state).
- **Standalone 1440 collision-proof specs / "Rest and Exhaustion Collision
  Maturation" proof spec** (report Candidate 3) — folded into per-spec FND-31
  validation + watchlist; a state-free proof spec is not warranted.

## Follow-ups identified, not actioned

- The three specs are **held drafts**; activation requires an explicit directive
  lifting the gameplay hold (S174/S175 were recently implemented, suggesting the
  hold is lifting, but the convention is preserved until stated). Each should pass
  `/reassess-spec` before `/spec-to-tickets`; reassessment must pin: the exact
  `LearnedSourcePreferences` shape (S177 D3), the per-agent-vs-world-table choice for
  water-quality effects (S177 D4), the storage-context source for perishables
  (S178 D3), and behaviorally-neutral defaults for the new threshold fields against
  current goldens.
- The Wash/Latrine multi-agent collision scenario noted by the second-iteration
  triage is now subsumed by S176's `survival-sanitation-breakdown-1440`.
