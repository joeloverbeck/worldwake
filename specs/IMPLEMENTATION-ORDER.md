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

## Excluded from the active order (by directive)

- **S60–S66** (gameplay/world-dynamics specs) — authored, but held until core AI
  architecture is stabilized. Do not schedule against this wave.
