# Musings: golden-top3-perf

## Learnings from Previous Campaign (golden-ai-perf)

The previous `golden-ai-perf` campaign targeted a different subset of golden tests and achieved -49% wall time reduction through affordance pre-filtering. Key takeaways:

### What Worked

- **Affordance pre-filtering** was the single biggest win: filtering out irrelevant action candidates before expensive plan search dramatically reduced search iterations.
- **Profile-guided approach**: Measuring first, then targeting the actual hot paths (not assumed ones) prevented wasted experiments.
- **Small, isolated changes**: Each experiment touched 1-2 files, making it easy to attribute gains and roll back failures.

### What Didn't Work / Diminishing Returns

- Micro-optimizations to already-fast paths yielded <1% gains.
- Over-aggressive pruning risked changing agent behavior (caught by golden test assertions).

### Key Observations

- `golden_determinism` dominates at ~81% of combined time and runs the full simulation **twice** (normal + replay verification). Any per-tick optimization gets doubled impact there.
- Affordance filtering low-hanging fruit is now captured on main. Remaining gains require **structural changes**: allocation reduction, caching, algorithm improvements.
- The prototype world is small (few places, few agents), so algorithmic complexity improvements may show modest absolute gains but establish patterns that scale.

## Hypotheses Priority

Starting with H1 (route cloning) and H7 (Floyd-Warshall caching) since topology operations are called deep in hot paths and the fixes are well-contained. H2 (reservation reverse index) is a close third due to clear O(k) → O(1) improvement.

H5 (PlanningState clone) and H10 (step list cloning) are higher-effort structural changes — save for later iterations after capturing easier wins.

## Running Notes

*(Updated during campaign execution)*
