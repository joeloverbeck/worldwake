# The Iterative Improvement Pattern: A Domain-Agnostic Extraction

## 1. Executive Summary

The iterative improvement pattern is a tight autonomous loop in which an agent repeatedly proposes a change to a mutable system, executes an experiment, measures results against a fixed evaluation harness, and either keeps or discards the change based on quantitative criteria. The agent runs indefinitely without human approval, advancing the system one improvement at a time while maintaining full rollback capability.

This pattern works because:

- **Fixed evaluation removes ambiguity.** A single immutable metric determines success or failure.
- **Time-budgeted experiments ensure comparability.** Every experiment runs under identical resource constraints, so improvements are real, not artifacts of longer runtimes.
- **Atomic accept/reject preserves stability.** The system never degrades — it either improves or stays the same.
- **Full audit trail enables learning.** Every attempt (success, failure, crash) is logged, giving the agent and human a history to reason about.
- **Autonomy maximizes throughput.** No human-in-the-loop bottleneck means experiments run continuously (e.g., ~100 experiments overnight).

The pattern is not specific to machine learning. It applies to any domain where you can: (a) define a measurable quality metric, (b) make incremental changes to a system, and (c) evaluate the result of those changes automatically.

---

## 2. The Core Loop (Abstract)

```
LOOP FOREVER:
    1. OBSERVE STATE
       - Inspect the current system state and history of past experiments
       - Identify what has been tried, what worked, what failed

    2. GENERATE HYPOTHESIS
       - Propose a specific, testable change to the mutable system
       - If stuck, revisit past near-misses, combine ideas, try radical alternatives

    3. IMPLEMENT CHANGE
       - Apply the proposed change to the mutable system
       - Create a checkpoint (snapshot of the changed state)

    4. EXECUTE EXPERIMENT
       - Run the system under fixed resource constraints (time, compute, etc.)
       - Capture all output (metrics, logs, errors)

    5. MEASURE RESULTS
       - Extract the primary metric from experiment output
       - If no metric produced (crash/timeout), classify as failure

    6. ACCEPT OR REJECT
       - IF primary metric improved:
           - Keep the change (advance the system state)
       - ELSE:
           - Rollback to the previous state (discard the change)
       - Apply qualitative modifiers:
           - Simplification with equal results = ACCEPT
           - Tiny improvement with high complexity cost = REJECT

    7. LOG AND REPEAT
       - Record: experiment ID, metric value, resource usage, status, description
       - Return to step 1
```

The loop has no termination condition. It runs until externally interrupted.

---

## 3. Component Architecture

### 3.1 The Evaluation Harness (Fixed, Immutable)

**What it is:** A function or process that measures the quality of the mutable system. It is never modified during the experiment cycle. Its immutability is the foundation of the entire pattern — without a stable metric, you cannot distinguish improvement from noise.

**Properties:**
- **Deterministic or low-variance.** Given the same system state, the harness should produce the same (or very similar) metric value.
- **Comparable across experiments.** The metric must be computed under identical conditions (same data, same resource budget, same evaluation parameters).
- **Isolated from the mutable system.** The agent cannot modify the evaluation logic. This prevents the agent from "gaming" the metric by changing how it's measured.
- **Single primary metric.** While secondary metrics (resource usage, complexity) inform the decision, one metric is the primary gate for accept/reject.

**Pseudocode:**
```
FUNCTION evaluate(system_state) -> MetricValue:
    // Load fixed evaluation data
    // Run system under controlled conditions
    // Compute and return the primary metric
    // This function is READ-ONLY — never modified
```

### 3.2 The Mutable System (What the Agent Can Change)

**What it is:** The artifact being iteratively improved. This is the ONLY thing the agent is allowed to modify.

**Properties:**
- **Clearly bounded.** The agent knows exactly what it can and cannot change.
- **Self-contained.** Changes to the mutable system should not require changes to the evaluation harness, external dependencies, or infrastructure.
- **Version-controlled.** Every state of the mutable system can be captured, restored, and diffed against other states.

**Pseudocode:**
```
MUTABLE_SYSTEM:
    // The single artifact the agent modifies
    // All changes are atomic: one change per experiment cycle
    // Must be serializable (can be saved/restored)
```

### 3.3 The Instruction Spec (Research Program / Goals)

**What it is:** A human-authored document that tells the agent what to optimize for, what constraints to respect, and what the boundaries of acceptable behavior are. It is the "constitution" of the improvement loop.

**Contents:**
- **Objective.** What metric to optimize (and in which direction).
- **Scope.** What the agent can modify and what it cannot.
- **Constraints.** Resource budgets, dependency restrictions, complexity preferences.
- **Behavioral directives.** Autonomy level, crash handling policy, when to give up on an idea.
- **Logging format.** How to record experiments for the audit trail.

### 3.4 The Accept/Reject Gate (Decision Logic)

**What it is:** The decision rule applied after each experiment to determine whether the change is kept or discarded.

**Decision tree:**
```
FUNCTION decide(old_metric, new_metric, complexity_delta) -> ACCEPT | REJECT:
    IF new_metric is MISSING (crash/timeout):
        RETURN REJECT

    IF new_metric is STRICTLY BETTER than old_metric:
        IF complexity_delta is LARGE and improvement is TINY:
            RETURN REJECT  // not worth the complexity
        ELSE:
            RETURN ACCEPT

    IF new_metric is EQUAL to old_metric:
        IF complexity_delta is NEGATIVE (simplification):
            RETURN ACCEPT  // simpler code, same result = win
        ELSE:
            RETURN REJECT

    IF new_metric is WORSE than old_metric:
        RETURN REJECT
```

**Key insight:** The gate has a **quantitative primary criterion** (metric improved?) and a **qualitative secondary criterion** (complexity trade-off). The primary criterion is mechanical; the secondary requires judgment.

### 3.5 The Experiment Log (Audit Trail)

**What it is:** A persistent record of every experiment attempted, including successes, failures, and crashes. Separate from the version-controlled system state.

**Purpose:**
- **History for the agent.** The agent reads the log to understand what's been tried and avoid repeating failed approaches.
- **History for the human.** The human reviews the log to understand the agent's research trajectory.
- **Post-hoc analysis.** Patterns in the log reveal which categories of changes tend to succeed or fail.

**Schema:**
```
EXPERIMENT_LOG:
    experiment_id    // unique identifier (e.g., git commit hash)
    metric_value     // primary metric achieved (0 if crash)
    resource_usage   // secondary metric (memory, time, etc.)
    status           // ACCEPT | REJECT | CRASH
    description      // human-readable summary of what was tried
```

### 3.6 The Rollback Mechanism (State Management)

**What it is:** The ability to restore the mutable system to its pre-experiment state when a change is rejected.

**Properties:**
- **Atomic.** Rollback restores the exact previous state — no partial undos.
- **Fast.** Rollback must be cheap enough that rejecting an experiment has negligible overhead.
- **Reliable.** The rollback mechanism itself must never fail or corrupt state.

**Pseudocode:**
```
// Before experiment:
checkpoint = snapshot(mutable_system)

// After experiment, if rejected:
restore(mutable_system, checkpoint)
```

In practice, version control (e.g., `git commit` / `git reset --hard`) provides all three properties naturally.

---

## 4. Safety and Constraint Mechanisms

### 4.1 Time/Resource Budget

Every experiment runs under a fixed resource budget. This serves two purposes: (a) experiments are comparable because they all had the same resources, and (b) the agent cannot accidentally consume unbounded resources.

```
CONSTANT TIME_BUDGET = <fixed duration>

FUNCTION run_experiment(mutable_system):
    start_time = now()
    // ... execute system ...
    IF elapsed(start_time) >= TIME_BUDGET:
        STOP execution
        RETURN partial_results

    // Additional hard timeout for total wall clock:
    IF elapsed(start_time) >= TIME_BUDGET * 2:
        KILL execution
        RETURN CRASH
```

### 4.2 Fast-Fail Detection

The system monitors for catastrophic failures during execution and aborts early rather than wasting the full time budget.

```
FUNCTION check_fast_fail(current_metric):
    IF current_metric is NaN:
        ABORT("metric became NaN — catastrophic failure")
    IF current_metric > FAILURE_THRESHOLD:
        ABORT("metric exceeded failure threshold")
```

### 4.3 Simplicity Criterion

A qualitative constraint that prevents complexity from growing unboundedly even when metrics improve slightly.

```
RULE simplicity_criterion:
    // A SMALL improvement that adds LARGE complexity → REJECT
    // An EQUAL result with LESS complexity → ACCEPT
    // Complexity is measured by: lines of code added/removed,
    //   number of new concepts introduced, coupling increase
```

This criterion is deliberately left to judgment rather than automated, because complexity is context-dependent.

### 4.4 Crash Handling and Recovery

```
FUNCTION handle_crash(experiment_result, attempt_count):
    IF experiment_result is CRASH:
        error = read_error_log()
        IF error is TRIVIAL (typo, missing import, off-by-one):
            IF attempt_count < MAX_FIX_ATTEMPTS:
                fix_error()
                RETURN RETRY
        // Error is fundamental or too many fix attempts
        log_experiment(status=CRASH)
        rollback()
        RETURN CONTINUE_TO_NEXT_IDEA
```

### 4.5 Early Abort on Losing Experiments

When the evaluation harness supports intermediate metric checkpoints (e.g., one output line per target file), the agent can abort an experiment early if the running total already exceeds the best known metric by more than `ABORT_THRESHOLD`. This avoids wasting the full time budget on clearly losing experiments.

```
CONSTANT ABORT_THRESHOLD = 0.05  // abort if 5% worse after any checkpoint

FUNCTION check_early_abort(running_metric, best_metric):
    IF running_metric > best_metric * (1 + ABORT_THRESHOLD):
        log_experiment(status=EARLY_ABORT)
        rollback()
        RETURN ABORT  // skip remaining checkpoints, move to next experiment
    RETURN CONTINUE
```

For multi-file harnesses: after each target file completes, parse its output and update the running total. If the running total already exceeds the abort threshold, kill the harness process immediately and classify the experiment as `EARLY_ABORT` (a subtype of REJECT).

### 4.6 Noise Reduction via Repeated Measurement

Single-run measurements can be noisy. To improve signal quality, the harness can be executed `HARNESS_RUNS` times per experiment. The median result is used for the accept/reject decision, as median is robust to outliers.

```
CONSTANT HARNESS_RUNS = 1  // default: single run (configurable per campaign)

FUNCTION measure_with_averaging(mutable_system, harness, runs):
    results = []
    FOR i IN 1..runs:
        result = run_experiment(mutable_system, harness)
        // Early abort still applies per-run
        IF check_early_abort(result.metric, best_metric) == ABORT:
            RETURN ABORT
        results.append(result.metric)

    metric = median(results)
    spread = max(results) - min(results)

    // If spread is large relative to noise tolerance, add a tiebreaker run
    IF spread > 2 * NOISE_TOLERANCE AND runs < MAX_RUNS:
        extra = run_experiment(mutable_system, harness)
        results.append(extra.metric)
        metric = median(results)

    RETURN { metric, spread }
```

Report the spread alongside the metric in the experiment log as a confidence indicator. Apply median-based measurement to the baseline as well.

---

## 5. Adaptive Strategy

### 5.1 Experiment Categories and Success Rate Tracking

Each experiment is tagged with a category from a predefined taxonomy (defined in the campaign's `program.md`). The agent tracks `attempts` and `accepts` per category across the experiment log.

```
FUNCTION compute_category_success_rates(experiment_log):
    categories = {}
    FOR each entry IN experiment_log:
        cat = entry.category
        categories[cat].attempts += 1
        IF entry.status == ACCEPT:
            categories[cat].accepts += 1
    FOR each cat IN categories:
        cat.success_rate = cat.accepts / cat.attempts
    RETURN categories

// Before hypothesizing, prefer high-yield categories
// but don't completely ignore low-rate ones (exploration matters)
```

### 5.2 Near-Miss Detection

A near-miss is an experiment where the metric is within noise tolerance of the best (no worse than ~1%) but not an improvement, AND the change did not simplify the code (lines_delta >= 0). Near-misses represent changes that were "almost good enough" — individually insufficient but potentially powerful in combination.

```
FUNCTION classify_near_miss(new_metric, best_metric, lines_delta):
    pct_diff = abs(new_metric - best_metric) / best_metric * 100
    IF pct_diff <= NOISE_TOLERANCE AND lines_delta >= 0:
        RETURN NEAR_MISS
    RETURN REJECT
```

Near-misses are stored with their diffs (e.g., via `git stash push -m "near-miss-exp-NNN"`) so they can be replayed later.

### 5.3 Plateau Detection and Strategy Shift

A plateau occurs when the agent accumulates `PLATEAU_THRESHOLD` consecutive rejects without any accepts. This signals that the current approach has been exhausted and a strategy shift is needed.

```
CONSTANT PLATEAU_THRESHOLD = 5  // consecutive rejects before strategy shift

STRATEGY_PROGRESSION:
    normal   → combine   → ablation   → radical
    (default)  (replay     (remove       (large structural
               2-3 near-   complexity    changes, rethink
               misses      from recent   approach entirely)
               together)   accepts)

// After any ACCEPT, reset strategy to "normal" and reset reject counter
```

**Combine strategy:** When near-misses exist, select 2-3 near-miss stashes, apply them together, and test as a single experiment. The hypothesis is that individually-insufficient changes may compound into a real improvement.

**Ablation strategy:** Review recent accepted changes and try removing complexity from them — sometimes an earlier accept introduced unnecessary overhead that can now be stripped.

**Radical strategy:** Abandon incremental optimization and try fundamentally different approaches (different algorithms, restructured data flow, etc.).

### 5.4 Structured Reflection (Musings)

Before implementing an experiment, the agent writes a brief hypothesis about WHY this change should improve the metric. After measuring, the agent records what was learned — whether the hypothesis was confirmed or refuted, and any surprising observations.

```
// Before implementing (after HYPOTHESIZE):
APPEND TO musings.md:
    ## exp-NNN: <description>
    **Hypothesis**: <why this should work>

// After measuring (after LOG):
APPEND TO musings.md:
    **Result**: <ACCEPT|REJECT|NEAR_MISS|CRASH> (<old_ms> → <new_ms> ms)
    **Learning**: <what was learned, what to try differently>
```

Musings are stored in `musings.md` in the campaign folder, keyed by experiment ID. This creates a searchable record of the agent's reasoning that improves future hypothesis quality.

### 5.5 Human Steering via Proposal Override

The agent checks for a `next-idea.md` file in the campaign folder at the start of each loop iteration. If the file exists, its contents are used as the hypothesis (skipping normal hypothesis generation). After using the proposal, the file is renamed so it's not reused.

```
// At the start of HYPOTHESIZE step:
IF exists(campaign_folder / "next-idea.md"):
    hypothesis = read("next-idea.md")
    rename("next-idea.md" → "next-idea.used-exp-NNN.md")
    RETURN hypothesis
ELSE:
    // proceed with normal hypothesis generation
```

This allows a human to steer the loop without interrupting it — they simply drop a file to inject their next idea.

## 6. The Autonomy Model

The pattern is designed for **fully autonomous operation**. Once the loop begins, the agent requires no human input, approval, or guidance. This is a deliberate design choice, not an accident.

**Key directives:**

1. **Never ask for permission to continue.** The human may be asleep, away, or deliberately disengaged. The loop runs until externally interrupted.

2. **Generate ideas independently.** When the agent runs out of obvious ideas, it should:
   - Re-read the system code and spec for overlooked angles
   - Combine elements from past near-miss experiments
   - Try more radical or unconventional changes
   - Revisit ideas that failed under different parameter combinations

3. **Handle all failures internally.** Crashes, timeouts, and bad results are normal parts of the loop, not reasons to stop.

4. **Maximize throughput.** The faster each experiment cycle completes, the more experiments run per unit of time. With ~5-minute experiments, ~12 experiments/hour, ~100 overnight.

```
DIRECTIVE never_stop:
    // Once the loop begins:
    //   - Do NOT ask the human "should I continue?"
    //   - Do NOT pause at "natural stopping points"
    //   - Do NOT stop when you run out of easy ideas
    //   - DO think harder, read more, combine approaches
    //   - The loop runs until the human kills the process
```

---

## 7. Pseudocode: Complete System

```
// ============================================================
// ITERATIVE IMPROVEMENT SYSTEM — COMPLETE PSEUDOCODE (v2)
// ============================================================

// --- CONSTANTS (set once, never changed) ---
TIME_BUDGET        = <fixed experiment duration>
HARD_TIMEOUT       = TIME_BUDGET * 2
FAILURE_THRESHOLD  = <domain-specific catastrophic value>
MAX_FIX_ATTEMPTS   = 3
ABORT_THRESHOLD    = 0.05   // early abort if 5% worse at any checkpoint
PLATEAU_THRESHOLD  = 5      // consecutive rejects before strategy shift
HARNESS_RUNS       = 1      // runs per experiment (median taken)
NOISE_TOLERANCE    = 0.01   // 1% metric difference = equal

// --- IMMUTABLE COMPONENTS ---
evaluation_harness = load_evaluation_harness()  // fixed, read-only
instruction_spec   = load_instruction_spec()    // goals, constraints, scope
categories         = load_categories(instruction_spec)  // experiment taxonomy

// --- MUTABLE STATE ---
mutable_system     = load_current_system()
experiment_log     = load_or_create_log()
musings            = load_or_create("musings.md")
best_metric        = NULL    // set after baseline
strategy           = "normal"
consecutive_rejects = 0

// ============================================================
// PHASE 1: INITIALIZATION
// ============================================================

// Establish baseline (with multi-run averaging if configured)
checkpoint(mutable_system)
baseline_metric = measure_with_averaging(mutable_system, evaluation_harness, HARNESS_RUNS)
best_metric = baseline_metric.metric
log(experiment_log, id=current_id(), metric=best_metric,
    resources=extract_resources(baseline_result),
    category="baseline", status=ACCEPT, description="baseline measurement")

// ============================================================
// PHASE 2: IMPROVEMENT LOOP
// ============================================================

LOOP FOREVER:

    // --- STEP 1: OBSERVE ---
    current_state = inspect(mutable_system)
    past_experiments = read(experiment_log)

    // --- STEP 1b: CHECK STRATEGY ---
    consecutive_rejects = count_consecutive_rejects(past_experiments)
    IF consecutive_rejects >= PLATEAU_THRESHOLD:
        near_misses = find_near_miss_stashes()
        IF strategy == "normal" AND near_misses.length > 0:
            strategy = "combine"
        ELSE IF strategy IN ("normal", "combine"):
            strategy = "ablation"
        ELSE:
            strategy = "radical"

    // --- STEP 1c: COMPUTE CATEGORY SUCCESS RATES ---
    category_rates = compute_category_success_rates(past_experiments)

    // --- STEP 2: HYPOTHESIZE ---
    // Check for human steering first
    IF exists(campaign_folder / "next-idea.md"):
        change = read("next-idea.md")
        rename("next-idea.md" → "next-idea.used-exp-NNN.md")
    ELSE IF strategy == "combine":
        change = combine_near_misses(select(near_misses, 2..3))
    ELSE IF strategy == "ablation":
        change = propose_ablation(recent_accepts)
    ELSE IF strategy == "radical":
        change = propose_radical_change(current_state, instruction_spec)
    ELSE:
        change = generate_hypothesis(
            current_state, past_experiments,
            instruction_spec, category_rates  // prefer high-yield categories
        )

    // --- STEP 2.5: RECORD HYPOTHESIS ---
    append(musings, "## exp-NNN: " + change.description)
    append(musings, "**Hypothesis**: " + change.reasoning)

    // --- STEP 3: IMPLEMENT ---
    apply(change, mutable_system)
    checkpoint(mutable_system)

    // --- STEP 4: EXECUTE (with multi-run + early abort) ---
    attempt = 0
    RETRY_LOOP:
        measurement = measure_with_averaging(
            mutable_system, evaluation_harness, HARNESS_RUNS
        )

        // --- STEP 4a: EARLY ABORT CHECK ---
        IF measurement == ABORT:
            log(experiment_log, status=EARLY_ABORT,
                category=change.category, description=change.description)
            rollback(mutable_system)
            CONTINUE

        // --- STEP 4b: FAST-FAIL CHECK ---
        IF measurement.status == CATASTROPHIC_FAILURE:
            IF is_trivial_fix(measurement.error) AND attempt < MAX_FIX_ATTEMPTS:
                apply_fix(measurement.error, mutable_system)
                checkpoint(mutable_system)
                attempt += 1
                GOTO RETRY_LOOP
            ELSE:
                log(experiment_log, status=CRASH,
                    category=change.category, description=change.description)
                rollback(mutable_system)
                CONTINUE

        // --- STEP 4c: TIMEOUT CHECK ---
        IF measurement.wall_time > HARD_TIMEOUT:
            log(experiment_log, status=CRASH,
                category=change.category,
                description=change.description + " (timeout)")
            rollback(mutable_system)
            CONTINUE

    // --- STEP 5: MEASURE ---
    new_metric = measurement.metric
    spread = measurement.spread
    resource_usage = extract_resources(measurement)

    // --- STEP 6: DECIDE ---
    decision = accept_or_reject(
        old_metric = best_metric,
        new_metric = new_metric,
        complexity_delta = measure_complexity_change(change)
    )

    // Check for near-miss before finalizing REJECT
    IF decision == REJECT:
        IF classify_near_miss(new_metric, best_metric, change.lines_delta) == NEAR_MISS:
            decision = NEAR_MISS

    IF decision == ACCEPT:
        best_metric = new_metric
        advance(mutable_system)
        log(experiment_log, metric=new_metric, resources=resource_usage,
            category=change.category, status=ACCEPT,
            description=change.description)
        strategy = "normal"       // reset on any accept
        consecutive_rejects = 0
    ELSE IF decision == NEAR_MISS:
        stash(mutable_system, "near-miss-exp-NNN: " + change.description)
        rollback(mutable_system)
        log(experiment_log, metric=new_metric, resources=resource_usage,
            category=change.category, status=NEAR_MISS,
            description=change.description)
    ELSE:
        rollback(mutable_system)
        log(experiment_log, metric=new_metric, resources=resource_usage,
            category=change.category, status=REJECT,
            description=change.description)

    // --- STEP 7.5: RECORD LEARNING ---
    append(musings, "**Result**: " + decision + " (" + best_metric + " → " + new_metric + " ms)")
    append(musings, "**Learning**: " + reflect_on_result(change, decision, measurement))

// ============================================================
// TERMINATION: External interruption only (human kills process)
// ============================================================
```

---

## 8. Appendix: Karpathy's Concrete Implementation

This section maps every abstract concept above to the specific implementation in the autoresearch codebase.

### 8.1 File Overview

| File | Role | Modifiable? |
|------|------|-------------|
| `program.md` | Instruction spec (agent's "constitution") | No (by agent) |
| `prepare.py` | Evaluation harness, constants, data loading | No |
| `train.py` | Mutable system (model, optimizer, training loop) | Yes |
| `results.tsv` | Experiment log (untracked by git) | Yes |
| `run.log` | Experiment output (overwritten each run) | Yes |

### 8.2 Concept-to-Code Mapping

| Abstract Concept | Karpathy Implementation | Location |
|-----------------|------------------------|----------|
| **Primary metric** | `val_bpb` (bits per byte, lower is better) | `prepare.py:343-365` (`evaluate_bpb()`) |
| **Evaluation harness** | `evaluate_bpb()` function with fixed `EVAL_TOKENS`, `MAX_SEQ_LEN` | `prepare.py:343-365` |
| **Fixed eval data** | Pinned validation shard (`shard_06542`) | `prepare.py:43` |
| **Evaluation constants** | `MAX_SEQ_LEN=2048`, `EVAL_TOKENS=40*524288` | `prepare.py:30-32` |
| **Mutable system** | `train.py` (architecture, optimizer, hyperparams, batch size) | `train.py` (entire file) |
| **Instruction spec** | `program.md` (goals, constraints, scope, behavioral directives) | `program.md:1-114` |
| **Objective** | "Get the lowest val_bpb" | `program.md:33` |
| **Scope: CAN modify** | `train.py` — everything is fair game | `program.md:25-26` |
| **Scope: CANNOT modify** | `prepare.py`, external dependencies | `program.md:28-31` |
| **Time budget** | `TIME_BUDGET = 300` seconds (5 minutes) | `prepare.py:31` |
| **Time enforcement** | `if step > 10 and total_training_time >= TIME_BUDGET: break` | `train.py:602-604` |
| **Hard timeout** | 10 minutes — kill and treat as failure | `program.md:108` |
| **Fast-fail: NaN** | `if math.isnan(train_loss_f) or train_loss_f > 100: exit(1)` | `train.py:569-572` |
| **Warmup exclusion** | Steps 0-10 excluded from time tracking (compilation overhead) | `train.py:578-579` |
| **Checkpoint (accept)** | `git commit` | `program.md:98` |
| **Rollback (reject)** | `git reset --hard HEAD~1` | `program.md:104` |
| **Accept criterion** | `val_bpb` improved (lower) | `program.md:103` |
| **Reject criterion** | `val_bpb` equal or worse | `program.md:104` |
| **Simplicity criterion** | Qualitative: weigh complexity cost vs. improvement magnitude | `program.md:37` |
| **Crash detection** | `grep` output empty → no `val_bpb` in log → crash | `program.md:101` |
| **Crash handling** | Trivial fix → retry; fundamental → skip, log crash | `program.md:110` |
| **Experiment log** | `results.tsv` (TSV: commit, val_bpb, memory_gb, status, description) | `program.md:66-88` |
| **Secondary metric** | `peak_vram_mb` (soft constraint) | `program.md:35` |
| **Run experiment** | `uv run train.py > run.log 2>&1` | `program.md:99` |
| **Read results** | `grep "^val_bpb:\|^peak_vram_mb:" run.log` | `program.md:100` |
| **Never-stop directive** | "Do NOT pause to ask... continue indefinitely until manually stopped" | `program.md:112` |
| **Idea generation when stuck** | "Read papers, re-read files, combine near-misses, try radical changes" | `program.md:112` |
| **Throughput estimate** | ~12 experiments/hour, ~100 overnight | `program.md:114` |
| **Dual tracking** | Git log (accepted changes only) + results.tsv (all experiments) | `program.md:98-104, 66-88` |
| **Output format** | Structured summary printed after training (val_bpb, timing, VRAM, etc.) | `train.py:610-619`, `program.md:41-56` |
