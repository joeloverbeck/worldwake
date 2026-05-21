# S162BELVIESOU-007: Restore lawful office/record information flow for the gated survival golden families (ask_consult, justice, offices)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Likely — candidate generation and/or the office/record belief-acquisition (consult) path; to be confirmed by per-family isolation.
**Deps**: S162 (belief-view source-gate hardening, merged on this branch). Sibling fix `project_self_produced_lot_belief` (this branch) already closed the `survival_production` regression.

## Problem

S162 closed the `office_data` / `record_data` / `believed_rights` / `can_control` /
`has_control` / `loyalty_to` / contention belief-view leaks. Its production gating
is correct — the focused unit tests and the *non-gated* `offices.rs` goldens pass,
and `consult_record` now lawfully projects `BelievedOfficeDataSnapshot` /
`BelievedRecordDataSnapshot`. But the four `#[ignore]`d gated survival golden
families were never re-run during S162 development (`cargo test -p worldwake-ai`
and `scripts/verify.sh` both skip `#[ignore]` tests), so the first CI run that
exercised them — `golden-survival.yml` on this branch — surfaced four failures:

- `survival_production_lands_row_eight` — **FIXED on this branch** via
  `WorldTxn::project_self_produced_lot_belief` (production-witness belief). Listed
  here only for context; out of scope.
- `survival_ask_consult_lands_row_six` — searcher never commits `ask_about_person`.
- `survival_justice_*` (4 tests) — merchant never commits `accuse`; searcher never
  commits `report_found`; `ask_witness` appears in `searcher_start_failed_actions`.
- `survival_offices_proves_force_law_uptake` / `survival_offices_replays_deterministically`
  — `ClaimOffice` / `press_force_claim` never selected.

`golden-survival.yml` is **green on `main`** (PRs #118–#122), so these are
S162-caused regressions, not pre-existing flakes. The remaining three families
are **trajectory-divergence regressions**, not simple golden-seeding gaps — see
the diagnosis below.

## Assumption Reassessment (2026-05-21)

1. **The scenario files were not touched by S162.** `git diff --stat main..HEAD --
   crates/worldwake-ai/tests/scenarios/survival_*.rs` is empty. The harness gained
   `seed_believed_office_data_from_world` and `seed_owner_belief`, and `offices.rs`
   adopted a `seed_support_office_belief` helper (seeds office-holder belief **+**
   `BelievedOfficeDataSnapshot`), but the gated `survival_*.rs` families did not.
2. **`accuse` / `fine` / `bounty` / `ClaimOffice` candidate generation reads
   `ctx.view.record_data(...)` / `ctx.view.office_data(...)`** (e.g.
   `candidate_generation.rs:1567,1574,2050,2057,2562`), which now return `None`
   unless a believed snapshot exists. `local_owned_commodity_evidence`
   (`candidate_generation.rs:7378`) additionally requires `believed_owner_of` for
   loose food — relevant to the production case already fixed.
3. **`ask_consult` is NOT a `can_control` regression.** Temporarily restoring
   `main`'s `can_control`/`believed_rights` co-located/owner/possessor branches did
   **not** flip `survival_ask_consult` to pass. The searcher generates
   `SearchForMissing { subject, last_seen: None }` ~703 times but
   `ShareBelief(Testimony)` consistently outranks it
   (`replacement=GoalChanged`, `ranking=ShareBeliefTopicOrder`), so it never reaches
   `ask_about_person`. Ranking code was not changed by S162; the divergence comes
   from S162 read-gating altering belief-derived inputs that cascade through the
   office/record/social dynamics over 1440 ticks. Fixed-tick A/B comparison is
   misleading here (deterministic-but-chaotic divergence) — isolate at the
   whole-run level per `fix-ci-failures` Step 5.
4. **The `ask_about_person` affordance is not directly gated.**
   `enumerate_ask_about_person_payloads` / the authoritative validator
   (`ask_about_person_actions.rs:120,200`) require only a co-located agent target,
   an `EpistemicDispositionProfile`, and an `Overdue` expectation — none S162-gated.
   The `ask_witness` start-failures in `justice` (CLAUDE.md Authoritative-to-AI
   checklist item 6, payload revalidation) still need confirmation as cause vs.
   benign transient.
5. **`consult_record` is the intended runtime carrier** for `office_data`/`record_data`
   (it projects both snapshots on commit — `consult_record_actions.rs:352,365`).
   `survival_offices` asserts `press_force_claim` *after consulting the register*,
   so pre-seeding a tick-0 snapshot would defeat that family's intent; the fix must
   ensure agents reach and commit `consult_record` at runtime, not mask it.
8. **Heuristic/ranking note:** the open question for `ask_consult` is whether
   `ShareBelief` legitimately outranks an overdue missing-person `SearchForMissing`,
   or whether S162 removed the substrate that previously let `SearchForMissing`
   become feasible/win. Name and verify before changing any ranking.

## Architecture Check

1. The fix must preserve S162's belief-only contract (FND-14/14A/14B): a distant
   actor must not regain a candidate from a remote office/record/loyalty change
   with no lawful carrier. The correct restoration is lawful *acquisition* (runtime
   `consult_record` / testimony / co-located perception), not re-broadening the gate.
2. No backward-compat shims; no reverting S162's accessor gating. If a candidate
   generator depends on a now-`None` snapshot, the dependency must be sourced from
   the believed snapshot (post-consult) or the candidate must be correctly absent
   until consult.

## Verification Layers

1. Candidate present/absent → decision trace (`goal_history_for`) per family.
2. `ask_witness` / `ask_about_person` start success vs. failure → action trace
   (`ActionTraceKind`), to settle whether justice's start-failures are causal.
3. Whole-run outcome flip (pass↔fail) when isolating a suspect accessor → not
   fixed-tick state comparison (chaotic divergence).

## What to Change

### 1. Per-family root-cause isolation (do this first)

For each of `ask_consult`, `justice`, `offices`: isolate which S162 gated accessor
drives the divergence by reverting one accessor at a time to `main` and re-running
the *whole* gated scenario, recording pass↔fail. `can_control` is already ruled out
for `ask_consult`. Prime suspects: `record_data`/`office_data` (justice/offices),
and the cascade these produce in the social/relay dynamics (ask_consult).

### 2. Restore lawful acquisition (scope depends on §1)

Likely candidates, to be confirmed: ensure agents lawfully reach `consult_record`
for local registers/offices so `accuse`/`ClaimOffice`/`press_force_claim` candidates
re-appear post-consult; and resolve whatever S162-gated input flipped the
`ask_consult` searcher's `SearchForMissing` vs `ShareBelief` ranking.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (likely modify)
- `crates/worldwake-systems/src/consult_record_actions.rs` and/or office/justice
  action handlers (possible modify)
- `crates/worldwake-ai/tests/scenarios/survival_{ask_consult,justice,offices}.rs`
  (modify only if the lawful-acquisition narrative legitimately needs harness
  belief seeding — never to mask absent runtime acquisition)

## Out of Scope

- `survival_production` — already fixed on this branch via
  `project_self_produced_lot_belief`.
- Any re-broadening or revert of S162's belief-view accessor gating.

## Acceptance Criteria

### Tests That Must Pass

1. `survival_ask_consult_lands_row_six`
2. `survival_justice_*` (all 4)
3. `survival_offices_proves_force_law_uptake`, `survival_offices_replays_deterministically`
4. Existing suite: full `golden-survival.yml` family stays green (no regression to
   the 12 currently-passing scenarios).

### Invariants

1. No distant actor gains an office/record/loyalty/control candidate without a
   lawful carrier (FND-14B); proven by the existing `belief_wall_trap` matrix.
2. Restored candidates trace to a believed snapshot acquired via consult/testimony
   /perception, not a live `world.` read.

## Test Plan

### New/Modified Tests

1. Possibly none beyond the four families above, depending on §1 outcome.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_ai 'scenarios::survival_ask_consult::' -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-ai --test golden_ai 'scenarios::survival_justice::' -- --ignored --test-threads=1` (run each of the 4 tests as separate positional filters)
3. `cargo test --release -p worldwake-ai --test golden_ai 'scenarios::survival_offices::' -- --ignored --test-threads=1`
4. `cargo test --release -p worldwake-ai --test golden_ai 'scenarios::survival_' -- --ignored --test-threads=1` (full family, no regressions)
5. `scripts/verify.sh`
