# Implementation Order

**Status**: ACTIVE

The phase-gate dependency graph and the first two AI-architecture consolidation
waves (S155–S161) are retired at
`archive/specs/IMPLEMENTATION-ORDER-final-2026-05-21.md` (and the dated archives it
references). This file reopens the active order for the **third** AI-architecture
consolidation iteration. Core AI architecture is still being stabilized first;
gameplay specs `S60`–`S66` remain authored but are **intentionally excluded** from
the active order until a future directive reopens them.

## Adjunct Wave: AI Architecture Consolidation — Third Iteration

**Source.** `reports/ai-architecture-consolidation-third-iteration.md` — the third
hostile AI-architecture audit (ChatGPT-Pro). The author did not clone the repo
(GitHub code search + targeted fetches only), so every load-bearing claim was
re-verified against the actual tree before acceptance, using FND-14A as the lens
(co-location-gated physical reads are lawful; `knows_entity`-gated social/legal/
contention reads are not). Verdict: accept the recommended **Option B (moderate
consolidation)** in narrowed form. The two heaviest "Critical" proposals —
per-field `SnapshotFieldSource` typing and the capability-trait split of
`RuntimeBeliefView` — were **rejected** (the planning snapshot has zero direct
`world.` reads, so it is lawful by construction once the view is lawful; same
rejection the second iteration made). Findings that did not survive verification
were dismissed; see
`docs/triage/2026-05-21-ai-architecture-consolidation-third-iteration-triage.md`.

Accepted work is the genuine, FOUNDATIONS-aligned subset: the deferred social/
control belief-view path plus residual contention leaks (S162), the player-POV
FND-19 boundary (S163), and the `EventId(0)` causal-honesty cleanup
(CAUSEVTHON-001 ticket).

```
S162 (belief-view source-gate hardening)  ── extends S158; completes deferred social/control + residual contention path
S163 (CLI player-POV boundary)            ── depends on S162 (player menu inherits the belief view)
CAUSEVTHON-001 (ticket: explicit no-source-event) ── independent of S162/S163
```

### Completed

- **CAUSEVTHON-001 — Explicit "no source event" in blocker/discrepancy memory**
  (ticket) — `archive/tickets/CAUSEVTHON-001-explicit-no-source-event.md` — *Status:
  COMPLETED on 2026-05-21.* Replaced the implicit `EventId(0)` sentinel on
  `Blocker.source_event`/`DiscrepancyEntry.source_event` with `Option<EventId>`
  across producers, persistence stamping, consumers, and tests. Independent.
  **FND-2, FND-29A.**

### Completed / Archived

- **S162 — Belief-View Source-Gate Hardening** —
  `archive/specs/S162-belief-view-source-gate-hardening.md` — *Status:
  COMPLETED.* Closed the
  confirmed FND-14/14A `PerAgentBeliefView` leaks (`has_control`, `record_data`/
  `office_data`, the no-gate contention reads, `loyalty_to`/`stock_storage_policy`,
  `believed_rights`/`can_control`), restored adversarial belief-wall golden proof,
  and locked the snapshot-through-view invariant. Completed the social/control path S158
  deferred. **FND-7, FND-14, FND-14A, FND-14B, FND-19, FND-27, FND-31.**

### Pending

- **S163 — CLI Player-POV Boundary** —
  `specs/S163-cli-player-pov-boundary.md` — *Status: DRAFT.* FND-19: routes the
  player action-menu labels and `handle_cancel` through the lawful belief view,
  marks `display.rs`/`control.rs` observer/debug-only with an enforceable guard,
  and adds a player/AI symmetry test. Sequence after archived S162. **FND-14,
  FND-14A, FND-19.**

## Adjunct Wave: AI Architecture Consolidation — Fourth Iteration

**Source.** `reports/ai-architecture-consolidation-fourth-iteration.md` — the fourth
hostile AI-architecture audit (ChatGPT-Pro). As with prior iterations the author did
not clone the repo (the leak inventory's "Evidence" column is empty), so every
load-bearing claim was re-verified against the actual tree. Verdict: **~85% of the
report is re-litigation of decisions already made and documented in S155/S157/S158/
S162 (the `&World`-holding view / `RuntimeBeliefView` capability-trait split and
per-field `SnapshotFieldSource` typing — rejected across the second and third
iterations; the `believed_rights`/`can_control` self/belief-gated live read —
S162's deliberate design; `direct_container`/`direct_possessor` — S158-verified
lawful; `merchandise_profile`/reward encumbrance — third-triage-verified gated) or
already pending as S163 (the CLI player-menu leak).** Stripped of re-litigation, the
report surfaced **one genuinely new, confirmed leak** — `entity_kind` and the
last-seen belief synthesis read live `world.entity_kind` for remote entities (S164)
— plus two latent footguns closed alongside it. See
`docs/triage/2026-05-22-ai-architecture-consolidation-fourth-iteration-triage.md`.

```
S163 (CLI player-POV boundary)            ── pending from the third iteration; higher priority; land first
S164 (belief-view kind source-gate)       ── sequence after S163; touches the shared belief view, independent of S163's CLI work
```

### Pending

- **S164 — Belief-View Kind Source-Gate + Faction-Policy Footgun Closure** —
  `specs/S164-belief-view-kind-source-gate.md` — *Status: DRAFT.* Closes the residual
  FND-14/14A entity-kind leak S158/S162's accessor sweep missed (`entity_kind` and
  the last-seen `believed_kind` synthesis must come from stored belief / a last-seen
  observed-kind carrier, never live `world.entity_kind`); gates the ungated bandit
  faction-policy accessors to lawfully known factions; adds a `facility_controller_at`
  remote-control-change confirming test; extends the S162 belief-wall goldens with a
  remote-kind-change scenario. Correctness fix; no new authoritative state. Sequence
  after S163. **FND-7, FND-14, FND-14A, FND-15, FND-19, FND-27, FND-31.**

## Excluded from the active order (by directive)

- **S60–S66** (gameplay/world-dynamics specs) — authored, but held until core AI
  architecture is stabilized. Do not schedule against this wave.
