# The Iterative Improvement Pattern: A Domain-Agnostic Extraction

## 1. Executive Summary

The iterative improvement pattern is a tight autonomous loop in which an agent repeatedly proposes a change to a mutable system, executes an experiment, measures results against a fixed evaluation harness, and either keeps or discards the change based on quantitative criteria. The agent runs indefinitely without human approval, advancing the system one improvement at a time while maintaining full rollback capability.

This pattern works because:

- **Fixed evaluation removes ambiguity.** A single immutable metric determines success or failure.
- **Time-budgeted experiments ensure comparability.** Every experiment runs under identical resource constraints, so improvements are real, not artifacts of longer runtimes.
- **Atomic accept/reject preserves stability.** The system never degrades — it either improves or stays the same.
- **Full audit trail enables learning.** Every attempt (success, failure, crash) is logged, giving the agent and human a history to reason about.
- **Autonomy maximizes throughput.** No human-in-the-loop bottleneck means experiments run continuously (e.g., ~100 experiments overnight).
- **Bandit-guided exploration prevents stagnation.** UCB1 scoring balances exploitation of productive categories with exploration of untried ones.
- **Statistical confidence prevents false accepts.** MAD-based noise detection ensures improvements are real, not measurement artifacts.
- **Goodhart defenses prevent metric gaming.** Multi-seed evaluation, suspicion gates, and regression checks guard against the agent exploiting harness quirks.
- **Backtracking escapes local optima.** Named checkpoints allow the agent to return to earlier promising states when stuck.
- **Cross-run learning accelerates future campaigns.** A time-decayed lesson store carries insights across experiments and campaigns.

The pattern is not specific to machine learning. It applies to any domain where you can: (a) define a measurable quality metric, (b) make incremental changes to a system, and (c) evaluate the result of those changes automatically.

---

## 2. The Core Loop (Abstract)

```
LOOP FOREVER:
    1. OBSERVE STATE
       - Inspect the current system state and history of past experiments
       - Identify what has been tried, what worked, what failed
       - Compute UCB1 scores per experiment category
       - Check PROCEED/REFINE/PIVOT trajectory
       - If meta-improvement enabled, check if meta-review is due

    2. GENERATE HYPOTHESIS
       - Select category with highest UCB1 score
       - Propose a specific, testable change to the mutable system
       - Consult lesson store for relevant patterns
       - If stuck, revisit past near-misses, combine ideas, try radical alternatives
       - If partial signals exist, focus on extending successful subsets

    3. IMPLEMENT CHANGE
       - Apply the proposed change to the mutable system
       - Create a checkpoint (snapshot of the changed state)

    4. EXECUTE EXPERIMENT
       - Run the system under fixed resource constraints (time, compute, etc.)
       - Capture all output (metrics, logs, errors, intermediate checkpoints)
       - Apply Goodhart checks (multi-seed, suspicion gate, regression)

    5. MEASURE RESULTS
       - Extract the primary metric from experiment output
       - Compute MAD-based noise floor from multi-run data
       - Parse intermediate metrics for partial signals
       - If no metric produced (crash/timeout), classify as failure

    6. ACCEPT OR REJECT
       - IF primary metric improved AND improvement exceeds noise floor:
           - Keep the change (advance the system state)
           - Record checkpoint for future backtracking
       - ELSE:
           - Rollback to the previous state (discard the change)
       - Apply qualitative modifiers:
           - Simplification with equal results = ACCEPT
           - Tiny improvement with high complexity cost = REJECT
       - Extract lesson from the outcome

    7. LOG AND REPEAT
       - Record: experiment ID, metric value, resource usage, status, description
       - Record learning and partial signals in musings
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
- **Optional intermediate output.** For harnesses that process multiple targets (files, test cases, benchmarks), emitting per-target metrics enables both early abort (negative signal) and partial signal detection (positive signal).

**Pseudocode:**
```
FUNCTION evaluate(system_state) -> MetricValue:
    // Load fixed evaluation data
    // Run system under controlled conditions
    // Emit intermediate metrics per target (optional)
    // Compute and return the primary metric
    // This function is READ-ONLY — never modified
```

**Intermediate metrics:** When a harness processes multiple targets sequentially, each target's metric can serve dual purposes:
- **Negative signal (early abort):** If the running total already exceeds the best known metric by `ABORT_THRESHOLD`, abort early.
- **Positive signal (partial improvement):** Even when the primary metric doesn't improve, improvement in a subset of targets is recorded as a "partial signal" that guides future hypothesis generation.

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
- **Experiment categories.** A taxonomy of change types for tracking success rates.
- **Configuration thresholds.** All tunable parameters (see Section 4 and 5 for defaults).
- **Meta-improvement flag.** Whether the agent may self-modify the instruction spec (Section 5.7).

### 3.4 The Accept/Reject Gate (Decision Logic)

**What it is:** The decision rule applied after each experiment to determine whether the change is kept or discarded.

**Decision tree:**
```
FUNCTION decide(old_metric, new_metric, complexity_delta, noise_floor) -> ACCEPT | REJECT:
    IF new_metric is MISSING (crash/timeout):
        RETURN REJECT

    improvement_pct = (old_metric - new_metric) / old_metric * 100

    IF improvement_pct > 0 AND improvement_pct < noise_floor:
        // Improvement is within measurement noise
        // Require additional confirmation runs (MIN_CONFIDENCE_RUNS)
        confirmed = run_additional_confirmation(MIN_CONFIDENCE_RUNS)
        IF NOT confirmed:
            RETURN REJECT

    IF improvement_pct > MAX_IMPROVEMENT_PCT:
        // Suspiciously large improvement — Goodhart check
        IF NOT plausible_explanation_exists():
            RETURN REJECT
        ELSE:
            RETURN SUSPICIOUS_ACCEPT  // accept but flag for review

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

**Key insight:** The gate has a **quantitative primary criterion** (metric improved?), a **statistical confidence criterion** (improvement exceeds noise floor?), a **Goodhart safety criterion** (improvement isn't suspiciously large?), and a **qualitative secondary criterion** (complexity trade-off). The first three are mechanical; the last requires judgment.

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
    category         // experiment category from taxonomy
    status           // ACCEPT | REJECT | NEAR_MISS | EARLY_ABORT | CRASH | SUSPICIOUS_ACCEPT | BACKTRACK
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

### 3.7 The Checkpoint Store (Backtracking)

**What it is:** A persistent list of named restore points created after each ACCEPT. Unlike the rollback mechanism (which only goes back to the previous state), the checkpoint store enables backtracking to ANY prior accepted state.

**Purpose:** When the agent exhausts all strategies from the current position (normal → combine → ablation → radical), it can backtrack to an earlier checkpoint and explore a different path. This prevents the agent from being permanently stuck in a local optimum.

**Schema:**
```
CHECKPOINT_STORE (checkpoints.jsonl):
    exp_id                   // experiment that created this checkpoint
    metric                   // metric value at this checkpoint
    commit                   // git commit hash
    lines_delta_cumulative   // total lines changed from baseline
    description              // what was the accepted change
    timestamp                // when the checkpoint was created
```

**Backtrack procedure:**
```
FUNCTION backtrack(checkpoint_store, experiment_log, musings):
    // Select checkpoint with best metric
    target = checkpoint_store.sort_by(metric, ascending).first()

    // If metrics are within 1%, prefer lower complexity
    candidates = checkpoint_store.filter(metric < target.metric * 1.01)
    target = candidates.sort_by(lines_delta_cumulative, ascending).first()

    // Execute
    git_reset_hard(target.commit)

    // Avoid repeating experiments already tried from this checkpoint
    already_tried = experiment_log.filter(after=target.timestamp)
    // Agent should read musings for already_tried experiments

    // Reset strategy
    strategy = "normal"
    consecutive_rejects = 0
    best_metric = target.metric
```

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

### 4.6 Noise Reduction via MAD-Based Confidence Scoring

Single-run measurements can be noisy. To improve signal quality, the harness can be executed `HARNESS_RUNS` times per experiment. The **Median Absolute Deviation (MAD)** is used to compute a statistical noise floor, replacing simple spread-based heuristics.

```
CONSTANT HARNESS_RUNS = 1       // default: single run (configurable per campaign)
CONSTANT MIN_CONFIDENCE_RUNS    // default: HARNESS_RUNS * 2

FUNCTION measure_with_confidence(mutable_system, harness, runs):
    results = []
    FOR i IN 1..runs:
        result = run_experiment(mutable_system, harness)
        // Early abort still applies per-run
        IF check_early_abort(result.metric, best_metric) == ABORT:
            RETURN ABORT
        results.append(result.metric)

    metric = median(results)

    // Compute MAD-based noise floor
    deviations = [abs(x - metric) FOR x IN results]
    MAD = median(deviations)
    normalized_MAD = 1.4826 * MAD    // scale factor for normal distribution equivalence
    noise_floor = normalized_MAD / metric * 100  // as percentage

    spread = max(results) - min(results)

    // If spread is large relative to MAD, measurement is unstable
    IF spread > 3 * normalized_MAD AND runs < MIN_CONFIDENCE_RUNS:
        // Add tiebreaker runs for confidence
        FOR i IN 1..(MIN_CONFIDENCE_RUNS - runs):
            extra = run_experiment(mutable_system, harness)
            results.append(extra.metric)
        metric = median(results)
        // Recompute noise floor with expanded data
        deviations = [abs(x - metric) FOR x IN results]
        MAD = median(deviations)
        normalized_MAD = 1.4826 * MAD
        noise_floor = normalized_MAD / metric * 100

    RETURN { metric, noise_floor, spread }
```

**Decision rule:** If a proposed improvement is smaller than `noise_floor`, the improvement is within measurement noise. The accept/reject gate requires `MIN_CONFIDENCE_RUNS` additional harness runs to confirm the improvement is real. Report the `noise_floor` alongside the metric in the experiment log as a confidence indicator.

For single-run campaigns (`HARNESS_RUNS == 1`), MAD cannot be computed. The agent uses `NOISE_TOLERANCE` (default 1%) as the assumed noise floor.

### 4.7 Goodhart's Law Defenses

Autonomous loops are vulnerable to Goodhart's Law: the agent may find ways to improve the metric that don't represent genuine improvement of the underlying system. Three guards defend against this.

#### 4.7.1 Multi-Seed Evaluation

```
CONSTANT HARNESS_SEEDS = 1  // default: disabled (single seed)

FUNCTION multi_seed_check(mutable_system, harness, seeds):
    IF seeds <= 1:
        RETURN PASS  // disabled

    results = []
    FOR seed IN 1..seeds:
        result = run_experiment(mutable_system, harness, env={"HARNESS_SEED": seed})
        results.append(result.metric)

    // Accept only if improvement holds across ALL seeds
    worst_case = max(results)  // for lower-is-better metrics
    IF worst_case > best_metric:
        RETURN FAIL  // improvement doesn't generalize across seeds
    RETURN PASS
```

**Rationale:** If the agent's improvement depends on a specific random seed in the evaluation, it likely exploited a seed-specific artifact rather than finding a genuine optimization.

#### 4.7.2 Suspicion Gate

```
CONSTANT MAX_IMPROVEMENT_PCT = 30  // flag improvements larger than 30%

FUNCTION suspicion_check(improvement_pct, change):
    IF improvement_pct > MAX_IMPROVEMENT_PCT:
        explanation = agent_explain_large_improvement(change, improvement_pct)
        IF explanation is PLAUSIBLE:
            RETURN SUSPICIOUS_ACCEPT  // accept but flag
        ELSE:
            RETURN REJECT  // likely metric gaming
    RETURN PASS
```

**Rationale:** Genuine improvements in iterative optimization rarely exceed 30% in a single step. Unusually large improvements are more likely to be artifacts (changed random seed, exploited timing, accidentally cached results).

#### 4.7.3 Periodic Regression Check

```
CONSTANT REGRESSION_CHECK_INTERVAL = 5  // every 5 accepts

FUNCTION regression_check(total_accepts, mutable_system, harness, best_metric):
    IF total_accepts % REGRESSION_CHECK_INTERVAL != 0:
        RETURN  // not time for a check

    // Re-run harness with no changes
    current_metric = run_experiment(mutable_system, harness)

    drift = abs(current_metric - best_metric) / best_metric * 100
    IF drift > NOISE_TOLERANCE:
        // Metric has drifted — recalibrate
        log("metric drift detected: expected " + best_metric + ", got " + current_metric)
        best_metric = current_metric  // recalibrate to reality
```

**Rationale:** Over many experiments, non-determinism in the harness (system load, thermal throttling, garbage collection) can cause the baseline to drift. Periodic regression checks detect this drift and recalibrate `best_metric` to prevent accepting changes that merely exploit favorable measurement conditions.

---

## 5. Adaptive Strategy

### 5.1 UCB1-Guided Category Selection

Each experiment is tagged with a category from a predefined taxonomy (defined in the campaign's `program.md`). The agent uses the **UCB1 (Upper Confidence Bound)** algorithm to balance exploitation of productive categories with exploration of untried ones.

```
CONSTANT UCB_EXPLORATION_C = 1.414  // exploration constant (sqrt(2) by default)

FUNCTION compute_ucb1_scores(experiment_log):
    total_experiments = experiment_log.length
    categories = {}

    FOR each entry IN experiment_log:
        cat = entry.category
        categories[cat].attempts += 1
        IF entry.status == ACCEPT:
            categories[cat].accepts += 1

    FOR each cat IN categories:
        cat.success_rate = cat.accepts / cat.attempts
        cat.ucb1_score = cat.success_rate + UCB_EXPLORATION_C * sqrt(ln(total_experiments) / cat.attempts)

    // Categories with 0 attempts get score = infinity (always explored first)
    FOR each cat IN all_categories:
        IF cat NOT IN categories:
            cat.ucb1_score = INFINITY

    RETURN categories.sort_by(ucb1_score, descending)

// Before hypothesizing in normal mode:
// Select the category with the highest UCB1 score
// Generate a hypothesis within that category
```

**Why UCB1:** Simple success-rate tracking ("prefer high-yield categories") creates a rich-get-richer dynamic where untried categories are permanently deprioritized. UCB1 adds an exploration bonus that grows with the logarithm of total experiments, guaranteeing that every category is eventually tried while still preferring proven winners. The exploration constant `C` controls the trade-off: higher values explore more, lower values exploit more.

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
    normal   → combine   → ablation   → radical   → backtrack
    (default)  (replay     (remove       (large       (reset to
               2-3 near-   complexity    structural   earlier
               misses      from recent   changes,     checkpoint,
               together)   accepts)      rethink      explore
                                         approach)    different path)

// After any ACCEPT, reset strategy to "normal" and reset reject counter
```

**Combine strategy:** When near-misses exist, select 2-3 near-miss stashes, apply them together, and test as a single experiment. The hypothesis is that individually-insufficient changes may compound into a real improvement.

**Ablation strategy:** Review recent accepted changes and try removing complexity from them — sometimes an earlier accept introduced unnecessary overhead that can now be stripped.

**Radical strategy:** Abandon incremental optimization and try fundamentally different approaches (different algorithms, restructured data flow, etc.).

**Backtrack strategy:** When even radical changes fail, the agent is stuck in a local optimum. Backtrack to an earlier checkpoint with a better metric-to-complexity ratio and explore a different path (see Section 3.7). After backtracking, the agent cross-references the experiment log to avoid repeating approaches already tried from that checkpoint.

### 5.4 Structured Reflection (Musings)

Before implementing an experiment, the agent writes a brief hypothesis about WHY this change should improve the metric. After measuring, the agent records what was learned — whether the hypothesis was confirmed or refuted, and any surprising observations.

```
// Before implementing (after HYPOTHESIZE):
APPEND TO musings.md:
    ## exp-NNN: <description>
    **Category**: <UCB1-selected category> (UCB1 score: X.XX)
    **Hypothesis**: <why this should work>

// After measuring (after LOG):
APPEND TO musings.md:
    **Result**: <ACCEPT|REJECT|NEAR_MISS|CRASH> (<old_ms> → <new_ms> ms, noise_floor: X%)
    **Partial signals**: <intermediate metrics that improved/regressed>
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

### 5.6 Self-Improving Research Strategy (Meta-Loop)

The instruction spec (`program.md`) is normally treated as immutable. With the meta-loop enabled (`meta_improvement: true`), the agent periodically optimizes its own search strategy — a second-order improvement loop.

```
CONSTANT META_REVIEW_INTERVAL = 20   // experiments between meta-reviews
CONSTANT META_TRIAL_WINDOW = 10      // trial period for meta-changes

// Only active if program.md contains: meta_improvement: true

FUNCTION meta_review(experiment_log, musings, program_md):
    // 1. SNAPSHOT
    backup = copy(program_md)

    // 2. ANALYZE
    recent = experiment_log.last(META_REVIEW_INTERVAL)
    accept_rate = recent.count(ACCEPT) / recent.length
    category_rates = compute_ucb1_scores(recent)
    plateau_frequency = count_strategy_shifts(recent)
    near_miss_conversion = count_near_miss_combines(recent) / count_near_misses(recent)

    // 3. HYPOTHESIZE META-CHANGE
    // Propose ONE change to program.md based on analysis
    // Example: "PLATEAU_THRESHOLD=5 triggered 3 strategy shifts but only 1 led to accept.
    //           Raising to 7 would reduce thrashing."

    // ALLOWED changes:
    //   - Threshold values: ABORT_THRESHOLD, PLATEAU_THRESHOLD, NOISE_TOLERANCE, UCB_EXPLORATION_C
    //   - Category weights/priorities and "root causes to seed" list
    //   - Strategy progression timing
    //   - Accept/reject thresholds (complexity vs. improvement boundary)
    //   - HARNESS_RUNS

    // FORBIDDEN changes (hard-wired safety rails):
    //   - Evaluation harness (harness.sh)
    //   - Objective direction (lower-is-better vs higher-is-better)
    //   - Mutable file list
    //   - META_REVIEW_INTERVAL itself (prevents runaway self-modification)
    //   - Safety-critical config: MAX_FIX_ATTEMPTS, HARD_TIMEOUT, MAX_IMPROVEMENT_PCT
    //   - Lesson store and logging format

    // 4. APPLY
    apply_meta_change(program_md)

    // 5. TRIAL
    trial_results = run_next_N_experiments(META_TRIAL_WINDOW)

    // 6. EVALUATE
    trial_accept_rate = trial_results.count(ACCEPT) / trial_results.length
    prior_accept_rate = accept_rate  // from step 2

    IF trial_accept_rate >= prior_accept_rate:
        // KEEP the change
        log_meta_review(decision=KEEP)
        extract_lesson(category="meta", lesson=description_of_change)
    ELSE:
        // REVERT
        restore(program_md, backup)
        log_meta_review(decision=REVERT)
```

**Safety:** The meta-loop cannot modify its own review interval (preventing acceleration of self-modification), cannot change the evaluation harness (preserving metric integrity), and cannot weaken safety constraints. Each meta-change is trialed and reverted if it doesn't help.

### 5.7 Cross-Run Lesson Store

The lesson store captures reusable insights from experiments and makes them available to future experiments and campaigns.

**Schema:**
```
LESSON:
    lesson           // what pattern worked (or failed) and why
    confidence       // 0.0-1.0 — how confident is this lesson
    source_exp       // experiment that generated this lesson
    category         // experiment category
    timestamp        // when the lesson was extracted
    decay_weight     // 1.0 → decreases over time, pruned below 0.3
    polarity         // "positive" (what works) or "negative" (what fails)
```

**Extraction triggers:**
```
FUNCTION extract_lessons(experiment_result, experiment_log):
    IF experiment_result.status == ACCEPT:
        // Extract positive lesson
        append(lessons, {
            lesson: what_pattern_worked(experiment_result),
            confidence: 0.7,
            polarity: "positive",
            decay_weight: 1.0,
            ...
        })

    // Check for repeated category failures
    recent_same_cat = experiment_log.last_N_in_category(experiment_result.category, 3)
    IF all(recent_same_cat.status == REJECT):
        // Extract negative lesson
        append(lessons, {
            lesson: what_consistently_fails(recent_same_cat),
            confidence: 0.6,
            polarity: "negative",
            decay_weight: 1.0,
            ...
        })
```

**Decay and pruning:**
```
// Every 50 experiments:
FOR each lesson IN lessons:
    lesson.decay_weight -= 0.1
    IF lesson.decay_weight < 0.3:
        DELETE lesson  // lesson is stale
```

**Global promotion:**
```
// Every 50 experiments or on campaign completion:
FOR each lesson IN lessons:
    IF lesson.confidence >= 0.8 AND lesson.decay_weight >= 0.5:
        IF lesson NOT IN global_lessons:  // skip duplicates
            append(global_lessons, lesson)
```

**Consumption:** New campaigns read `campaigns/lessons-global.jsonl` at startup. The agent uses relevant global lessons to inform its initial hypothesis queue and category priorities.

### 5.8 PROCEED/REFINE/PIVOT Decision Framework

Beyond the per-experiment accept/reject gate, the agent periodically evaluates its overall trajectory to decide whether to continue, adjust, or fundamentally change its approach.

```
CONSTANT PIVOT_CHECK_INTERVAL = 10  // check every 10 experiments

FUNCTION trajectory_check(experiment_log):
    recent = experiment_log.last(PIVOT_CHECK_INTERVAL)
    accept_rate = recent.count(ACCEPT) / recent.length

    IF accept_rate > 0.20:
        RETURN PROCEED   // approach is productive, continue normally

    IF accept_rate >= 0.10:
        RETURN REFINE    // approach has potential, adjust parameters
        // Actions: tighten/loosen thresholds, shift category priorities,
        // re-read mutable files for missed angles

    IF accept_rate < 0.10:
        RETURN PIVOT     // approach is exhausted
        // Actions: consult lesson store for alternative strategies,
        // trigger radical strategy regardless of consecutive reject count,
        // if no relevant lessons, backtrack to earlier checkpoint
```

**Relationship to plateau detection:** Plateau detection (Section 5.3) triggers after consecutive rejects and cycles through strategies linearly. PROCEED/REFINE/PIVOT operates on a rolling window and can trigger a strategic shift even if rejects are interspersed with occasional near-misses.

## 6. The Autonomy Model

The pattern is designed for **fully autonomous operation**. Once the loop begins, the agent requires no human input, approval, or guidance. This is a deliberate design choice, not an accident.

**Key directives:**

1. **Never ask for permission to continue.** The human may be asleep, away, or deliberately disengaged. The loop runs until externally interrupted.

2. **Generate ideas independently.** When the agent runs out of obvious ideas, it should:
   - Re-read the system code and spec for overlooked angles
   - Combine elements from past near-miss experiments
   - Consult the lesson store for patterns from past campaigns
   - Try more radical or unconventional changes
   - Revisit ideas that failed under different parameter combinations
   - Use partial signals to identify promising subsets to extend

3. **Handle all failures internally.** Crashes, timeouts, and bad results are normal parts of the loop, not reasons to stop.

4. **Maximize throughput.** The faster each experiment cycle completes, the more experiments run per unit of time. With ~5-minute experiments, ~12 experiments/hour, ~100 overnight.

```
DIRECTIVE never_stop:
    // Once the loop begins:
    //   - Do NOT ask the human "should I continue?"
    //   - Do NOT pause at "natural stopping points"
    //   - Do NOT stop when you run out of easy ideas
    //   - DO think harder, read more, combine approaches
    //   - DO consult lessons and partial signals for guidance
    //   - The loop runs until the human kills the process
```

---

## 7. Pseudocode: Complete System

```
// ============================================================
// ITERATIVE IMPROVEMENT SYSTEM — COMPLETE PSEUDOCODE (v3)
// ============================================================

// --- CONSTANTS (set once, never changed) ---
TIME_BUDGET            = <fixed experiment duration>
HARD_TIMEOUT           = TIME_BUDGET * 2
FAILURE_THRESHOLD      = <domain-specific catastrophic value>
MAX_FIX_ATTEMPTS       = 3
ABORT_THRESHOLD        = 0.05   // early abort if 5% worse at any checkpoint
PLATEAU_THRESHOLD      = 5      // consecutive rejects before strategy shift
HARNESS_RUNS           = 1      // runs per experiment (median taken)
NOISE_TOLERANCE        = 0.01   // 1% metric difference = equal
UCB_EXPLORATION_C      = 1.414  // UCB1 exploration constant
MIN_CONFIDENCE_RUNS    = HARNESS_RUNS * 2  // extra runs for noise-floor confirmation
HARNESS_SEEDS          = 1      // multi-seed evaluation (1 = disabled)
MAX_IMPROVEMENT_PCT    = 30     // suspicion gate threshold
REGRESSION_CHECK_INTERVAL = 5   // regression check every N accepts
META_REVIEW_INTERVAL   = 20     // experiments between meta-reviews
META_TRIAL_WINDOW      = 10     // trial period for meta-changes
PIVOT_CHECK_INTERVAL   = 10     // trajectory check interval

// --- IMMUTABLE COMPONENTS ---
evaluation_harness = load_evaluation_harness()  // fixed, read-only
instruction_spec   = load_instruction_spec()    // goals, constraints, scope
categories         = load_categories(instruction_spec)  // experiment taxonomy
global_lessons     = load_or_create("campaigns/lessons-global.jsonl")

// --- MUTABLE STATE ---
mutable_system      = load_current_system()
experiment_log      = load_or_create_log()
musings             = load_or_create("musings.md")
checkpoint_store    = load_or_create("checkpoints.jsonl")
lesson_store        = load_or_create("lessons.jsonl")
intermediates_log   = load_or_create("intermediates.jsonl")
best_metric         = NULL    // set after baseline
strategy            = "normal"
consecutive_rejects = 0
total_accepts       = 0
experiment_count    = 0

// ============================================================
// PHASE 1: INITIALIZATION
// ============================================================

// Establish baseline (with MAD-based confidence scoring if multi-run)
checkpoint(mutable_system)
baseline_result = measure_with_confidence(mutable_system, evaluation_harness, HARNESS_RUNS)
best_metric = baseline_result.metric
log(experiment_log, id=current_id(), metric=best_metric,
    resources=extract_resources(baseline_result),
    category="baseline", status=ACCEPT, description="baseline measurement")
append(checkpoint_store, {exp_id: "baseline", metric: best_metric,
    commit: current_commit(), lines_delta_cumulative: 0})

// ============================================================
// PHASE 2: IMPROVEMENT LOOP
// ============================================================

LOOP FOREVER:
    experiment_count += 1

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
        ELSE IF strategy IN ("normal", "combine", "ablation"):
            strategy = "radical"
        ELSE:
            strategy = "backtrack"

    // --- STEP 1c: COMPUTE UCB1 CATEGORY SCORES ---
    category_scores = compute_ucb1_scores(past_experiments)

    // --- STEP 1d: BACKTRACK CHECK ---
    IF strategy == "backtrack":
        target = select_best_checkpoint(checkpoint_store)
        git_reset_hard(target.commit)
        log(experiment_log, status=BACKTRACK, description="backtracked to " + target.exp_id)
        append(musings, "## backtrack\n**Backtracked to " + target.exp_id + "**")
        strategy = "normal"
        consecutive_rejects = 0
        best_metric = target.metric
        CONTINUE  // restart loop from new position

    // --- STEP 1e: PROCEED/REFINE/PIVOT CHECK ---
    IF experiment_count % PIVOT_CHECK_INTERVAL == 0:
        trajectory = trajectory_check(past_experiments)
        IF trajectory == REFINE:
            // Adjust parameters, shift priorities
        ELSE IF trajectory == PIVOT:
            // Consult lessons, trigger radical, or backtrack
            strategy = "radical"

    // --- STEP 1f: META-REVIEW ---
    IF meta_improvement_enabled AND experiment_count % META_REVIEW_INTERVAL == 0:
        meta_review(experiment_log, musings, instruction_spec)

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
        change = propose_radical_change(current_state, instruction_spec, lesson_store)
    ELSE:
        // Normal mode: use UCB1-selected category
        target_category = category_scores.first()  // highest UCB1 score
        change = generate_hypothesis(
            current_state, past_experiments,
            instruction_spec, target_category,
            lesson_store, global_lessons,
            intermediates_log  // partial signals for guidance
        )

    // --- STEP 2.5: RECORD HYPOTHESIS ---
    append(musings, "## exp-NNN: " + change.description)
    append(musings, "**Category**: " + change.category + " (UCB1: " + change.ucb1_score + ")")
    append(musings, "**Hypothesis**: " + change.reasoning)

    // --- STEP 3: IMPLEMENT ---
    apply(change, mutable_system)
    checkpoint(mutable_system)

    // --- STEP 4: EXECUTE (with MAD + early abort + intermediates) ---
    attempt = 0
    RETRY_LOOP:
        measurement = measure_with_confidence(
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

        // --- STEP 4d: GOODHART CHECKS ---
        // Multi-seed evaluation
        IF HARNESS_SEEDS > 1:
            IF multi_seed_check(mutable_system, evaluation_harness, HARNESS_SEEDS) == FAIL:
                log(experiment_log, status=REJECT,
                    description=change.description + " (failed multi-seed)")
                rollback(mutable_system)
                CONTINUE

    // --- STEP 5: MEASURE ---
    new_metric = measurement.metric
    noise_floor = measurement.noise_floor
    resource_usage = extract_resources(measurement)

    // Parse and record intermediate metrics
    IF measurement.has_intermediates:
        append(intermediates_log, {
            exp_id: current_id(),
            checkpoints: measurement.intermediates,
            primary_metric: new_metric,
            partial_signals: compute_partial_signals(measurement.intermediates, previous_intermediates)
        })

    // --- STEP 6: DECIDE ---
    improvement_pct = (best_metric - new_metric) / best_metric * 100

    // Noise floor check
    IF improvement_pct > 0 AND improvement_pct < noise_floor:
        confirmed = run_additional_confirmation(MIN_CONFIDENCE_RUNS)
        IF NOT confirmed:
            decision = REJECT
            GOTO FINALIZE

    // Suspicion gate
    IF improvement_pct > MAX_IMPROVEMENT_PCT:
        IF NOT plausible_explanation_exists(change, improvement_pct):
            decision = REJECT
            GOTO FINALIZE
        ELSE:
            decision = SUSPICIOUS_ACCEPT
            GOTO FINALIZE

    decision = accept_or_reject(
        old_metric = best_metric,
        new_metric = new_metric,
        complexity_delta = measure_complexity_change(change)
    )

    // Check for near-miss before finalizing REJECT
    IF decision == REJECT:
        IF classify_near_miss(new_metric, best_metric, change.lines_delta) == NEAR_MISS:
            decision = NEAR_MISS

    FINALIZE:
    IF decision IN (ACCEPT, SUSPICIOUS_ACCEPT):
        best_metric = new_metric
        advance(mutable_system)
        total_accepts += 1
        log(experiment_log, metric=new_metric, resources=resource_usage,
            category=change.category, status=decision,
            description=change.description)
        append(checkpoint_store, {exp_id: current_id(), metric: new_metric,
            commit: current_commit(), lines_delta_cumulative: cumulative_lines()})
        strategy = "normal"       // reset on any accept
        consecutive_rejects = 0

        // Periodic regression check
        IF total_accepts % REGRESSION_CHECK_INTERVAL == 0:
            regression_check(total_accepts, mutable_system, evaluation_harness, best_metric)

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
    append(musings, "**Result**: " + decision + " (" + best_metric + " → " + new_metric + " ms, noise_floor: " + noise_floor + "%)")
    append(musings, "**Partial signals**: " + format_partial_signals(intermediates_log.last()))
    append(musings, "**Learning**: " + reflect_on_result(change, decision, measurement))

    // --- STEP 7.6: EXTRACT LESSON ---
    extract_lessons(decision, change, experiment_log, lesson_store)

    // --- LESSON MAINTENANCE ---
    IF experiment_count % 50 == 0:
        decay_lessons(lesson_store)
        promote_lessons(lesson_store, global_lessons)

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

---

## 9. Appendix: Feature Provenance

Features in this document were extracted from Karpathy's autoresearch and its fork ecosystem.

| Feature | Source | Section |
|---------|--------|---------|
| Core loop, evaluation harness, accept/reject gate | Karpathy's autoresearch (original) | 2-4 |
| Near-miss detection and combine strategy | LudoForge adaptation (v1), inspired by fork discussions | 5.2-5.3 |
| Structured musings | LudoForge adaptation (v1) | 5.4 |
| Human steering via next-idea.md | LudoForge adaptation (v1) | 5.5 |
| UCB1 category selection | karpathy/autoresearch Issue #284 | 5.1 |
| MAD confidence scoring | pi-autoresearch (Shopify fork, 1,377 stars) | 4.6 |
| Multi-seed evaluation | karpathy/autoresearch Discussion #285 | 4.7.1 |
| Suspicion gate | Community reports of metric gaming | 4.7.2 |
| Periodic regression check | Community Goodhart's Law discussions | 4.7.3 |
| Lightweight backtracking | AIDE (WecoAI), SWE-Search (ICLR 2025) | 3.7, 5.3 |
| Self-improving research strategy | karpathy/autoresearch Issue #314 | 5.6 |
| Cross-run lesson store | AutoResearchClaw (6,000 stars) | 5.7 |
| PROCEED/REFINE/PIVOT framework | AutoResearchClaw self-healing executor | 5.8 |
| Intermediate metrics / partial signals | Multiple forks extending early-abort | 3.1, 5.4 |
