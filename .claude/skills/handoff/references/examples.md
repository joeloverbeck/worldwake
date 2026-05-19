# Handoff Examples

Five worked examples covering the acceptance scenarios. Each is abbreviated for illustration; real packets follow the full 16-section template.

---

## Example 1 — Coding session with modified files and failed tests

~~~
# RESUME PACKET — fix planner beam overflow — 2026-04-21

## 1. Objective
Fix beam-overflow regression in `search_plan` when `MAX_BEAM_WIDTH` is 0.

## 2. Latest explicit user request
"Make `cargo test -p worldwake-ai golden_travel_chain` pass without raising `PLAN_BUDGET_CEILING`."

## 3. Hard constraints
- No new `f32`/`f64` — use `Permille`.
- Do not raise `PLAN_BUDGET_CEILING`.

## 4. Current status
Root cause identified (`BeamState::push` unbounded insert). Partial fix in working tree; 1 golden still failing.

## 5. Relevant files and symbols
- `crates/worldwake-ai/src/search.rs:412` — `search_plan`
- `crates/worldwake-ai/src/search.rs:87` — `BeamState::push`
- `crates/worldwake-ai/tests/scenarios/travel_physiology.rs`

## 6. Workspace state
```
CWD: /home/joeloverbeck/projects/worldwake
Repo root: /home/joeloverbeck/projects/worldwake
Branch:    fix/beam-overflow
HEAD:      b9517f92

-- git status --short --
 M crates/worldwake-ai/src/search.rs

-- diff stats --
Staged:   none
Unstaged:  1 file changed, 12 insertions(+), 3 deletions(-)
Untracked files: 0
```

## 7. Decisions already made
- Fix belongs in `BeamState::push`, not in caller (simpler invariant).
- Reuse existing `MAX_BEAM_WIDTH`; no new tuning constant.

## 8. Dead ends / do not retry
- Capping budget in caller → breaks `golden_merchant_stall`.
- Switching beam to `BinaryHeap` → determinism regression.

## 9. Evidence-backed facts
- `golden_travel_chain` fails at tick 14 with `PlanFailure::BudgetExceeded`.
- `BeamState::push` inserts unbounded when `MAX_BEAM_WIDTH == 0`.

## 10. Hypotheses / things to verify
- The overflow may also affect `golden_trade_chain` — not rerun yet.

## 11. Open blockers
None.

## 12. Tests / commands already run
- `cargo test -p worldwake-ai golden_travel_chain` → fail (tick 14)
- `cargo clippy --workspace` → pass

## 13. Things that will NOT survive a fresh session
- Uncommitted working-tree fix to `search.rs:412`.
- Mental model of the beam-overflow trace.

## 14. Reinvoke / reread on next session
- Reread `docs/debugging-traces.md` before modifying planner.

## 15. Ordered next steps
1. `git diff crates/worldwake-ai/src/search.rs` — verify the partial fix.
2. Complete the `BeamState::push` guard.
3. `cargo test -p worldwake-ai` — confirm all goldens.
4. `./scripts/verify.sh` before commit.

## 16. Paste into new session
Continue `.claude/handoffs/latest.md`. Fix the `BeamState::push` guard in `crates/worldwake-ai/src/search.rs`; do not raise `PLAN_BUDGET_CEILING`.
~~~

---

## Example 2 — Debug/research session with many dead ends, no code changes

~~~
## 4. Current status
Investigating intermittent `golden_merchant_stall` failure. No code changes this session.

## 5. Relevant files and symbols
- `crates/worldwake-systems/src/trade.rs`
- `crates/worldwake-ai/tests/scenarios/merchant_selling.rs`

## 6. Workspace state
```
CWD: /home/joeloverbeck/projects/worldwake
Branch: main
HEAD:   b9517f92
-- git status --short --
(clean)
```

## 8. Dead ends / do not retry
- Perception-range explanation → refuted (trace log, tick 42).
- Belief-staleness → refuted; belief timestamps current.
- RNG-seed divergence → refuted; seeds match across runs.

## 10. Hypotheses / things to verify
- Goal-ranking may deprioritize restocking when hunger drive fires simultaneously.
- `can_exercise_control` may short-circuit during force-control handover.

## 12. Tests / commands already run
- `cargo test -p worldwake-ai golden_merchant_stall` → pass 10/10
- Decision trace agent 7, ticks 40-55 → inspected, no smoking gun

## 15. Ordered next steps
1. Instrument `goal_ranking.rs` to log suppressed goals.
2. Rerun scenario with trace enabled.
3. Compare trace against the tick-42 hypothesis.
~~~

---

## Example 3 — Session where nested rules or extra skills matter

~~~
## 14. Reinvoke / reread on next session
- Nested rules: `.claude/worktrees/legact-007/CLAUDE.md` — worktree-root discipline.
- Reinvoke `superpowers:test-driven-development` before touching belief-view code.
- Reread `docs/spec-drafting-rules.md` section 5 (profile completeness).
- Path-scoped: `specs/IMPLEMENTATION-ORDER.md` — phase gate E13 is active.
~~~

---

## Example 4 — Non-git directory

~~~
## 6. Workspace state
```
CWD: /home/joeloverbeck/scratch/handoff-sandbox
(not a git repo)
```
~~~

All other sections fill normally. §5 uses absolute paths since there is no repo root. §7, §8, §12, §15 still apply.

---

## Example 5 — Fresh session restarted from the generated packet

First message of the new session pastes §16 verbatim:

```
Continue `.claude/handoffs/latest.md`. Fix the `BeamState::push` guard in `crates/worldwake-ai/src/search.rs`; do not raise `PLAN_BUDGET_CEILING`.
```

The new session:
1. Reads `.claude/handoffs/latest.md` (one compact file).
2. Reloads any §14 items explicitly (nested CLAUDE.md, skills, docs).
3. Starts at §15 step 1.

No `/compact`, no conversation replay, no re-discovery.
