# S10BILTRANEG-003: Faratin concession curve and opening offer derivation

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-systems` (new pure functions in trade_actions.rs)
**Deps**: S10BILTRANEG-001

## Problem

The negotiation protocol requires agents to generate progressively more favorable counter-offers using a time-dependent concession function. Without offer generation, `tick_trade` (ticket 005) cannot compute counter-offers and `enumerate_trade_payloads` (ticket 004) cannot compute opening offers. The spec uses the Faratin concession curve parameterized by two existing but unused `TradeDispositionProfile` fields (`initial_offer_bias`, `concession_rate`) plus the new `rejection_escalation_rate` (added in ticket 001).

## Assumption Reassessment (2026-04-02)

1. `TradeDispositionProfile.initial_offer_bias: Permille` exists at `crates/worldwake-core/src/trade.rs:74` — confirmed never accessed via `.initial_offer_bias` in production code (grep returns zero matches outside struct definition and test construction).
2. `TradeDispositionProfile.concession_rate: Permille` exists at `crates/worldwake-core/src/trade.rs:75` — confirmed never accessed via `.concession_rate` in production code.
3. `TradeDispositionProfile.negotiation_round_ticks: NonZeroU32` exists and is used by `DurationExpr::ActorTradeDisposition` to set action duration.
4. `TradeRole` will be available from ticket 001 in `worldwake-core/src/trade.rs`.
5. `rejection_escalation_rate: Permille` will be available from ticket 001.
6. `Permille::value() -> u16` and `Quantity(pub u32)` support the integer arithmetic needed for the Faratin function. No floats needed — the curve can be computed with integer approximation using `u64` intermediates to avoid overflow.

## Architecture Check

1. Pure functions with no state mutation — they compute offers from reservation prices, profile parameters, and round number. This keeps them independently testable and deterministic.
2. The Faratin curve is a well-studied negotiation function (Faratin et al., 1998). Using a known algorithm rather than inventing one reduces risk and makes behavior predictable.
3. No backward-compatibility shims. These are new functions that activate currently-unused profile fields.

## Verification Layers

1. Boulware curve concedes slowly then rapidly -> focused unit test (curve shape assertion)
2. Conceder curve concedes rapidly then slowly -> focused unit test
3. Linear curve concedes uniformly -> focused unit test
4. Monotonic concession: buyer offers never decrease, seller asks never increase -> focused unit test
5. Opening offer shifts with rejection count proportional to rejection_escalation_rate -> focused unit test
6. Urgency modulation: effective deadline shrinks with higher need -> focused unit test
7. Single-layer ticket (pure function tests, no runtime integration).

## What to Change

### 1. Implement `generate_offer` in `trade_actions.rs`

```rust
fn generate_offer(
    role: TradeRole,
    reservation: Quantity,
    opening: Quantity,
    round: u32,
    deadline: u32,
    concession_rate: Permille,
) -> Quantity
```

Implements the Faratin time-dependent concession function:
- `t = round / deadline` (normalized time, 0.0 to 1.0, computed via integer arithmetic)
- `alpha(t) = t^beta` where `beta` is derived from `concession_rate`:
  - `pm(0)-pm(499)` → Boulware (beta > 1, slow start)
  - `pm(500)` → Linear (beta = 1)
  - `pm(501)-pm(1000)` → Conceder (beta < 1, fast start)
- For Buyer: `offer = opening + alpha * (reservation - opening)` (offers go up toward reservation)
- For Seller: `offer = opening - alpha * (opening - reservation)` (asks go down toward reservation)
- Result is clamped to `[1, reservation]` for buyers, `[reservation, opening]` for sellers.
- All arithmetic uses `u64` intermediates to prevent overflow, then truncates to `u32`.

### 2. Implement `derive_opening_offer` in `trade_actions.rs`

```rust
fn derive_opening_offer(
    role: TradeRole,
    reservation: Quantity,
    initial_offer_bias: Permille,
    rejection_escalation_rate: Permille,
    prior_rejections: u32,
) -> Quantity
```

Logic:
- Base opening from `reservation` adjusted by `initial_offer_bias`. For buyers: `pm(0)` opens at reservation (generous), `pm(1000)` opens at 1 (aggressive). For sellers: inverse.
- Each prior rejection shifts the opening toward the counterparty's likely reservation, at a rate of `rejection_escalation_rate` of reservation per rejection (capped at 4 rejections).
- Buyer openings never exceed reservation. Seller openings never go below 1.

### 3. Implement `urgency_modulated_deadline` in `trade_actions.rs`

```rust
fn urgency_modulated_deadline(
    base_patience: NonZeroU32,
    needs: Option<&HomeostaticNeeds>,
    commodity: CommodityKind,
) -> u32
```

Logic:
- Maps commodity to its relevant need (Apple → hunger, Water → thirst, etc.).
- Higher urgency → shorter deadline: `effective = base * (1000 - urgency) / 1000`.
- Floor at 1 round (even a desperate agent gets one chance).

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (modify — add `generate_offer`, `derive_opening_offer`, `urgency_modulated_deadline`)

## Out of Scope

- Reservation price computation (ticket 002)
- Integration with `enumerate_trade_payloads` (ticket 004)
- Integration with `tick_trade` negotiation rounds (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. `generate_offer` with Boulware rate (`pm(100)`) concedes slowly in early rounds, rapidly near deadline
2. `generate_offer` with Conceder rate (`pm(900)`) concedes rapidly in early rounds, slowly near deadline
3. `generate_offer` with Linear rate (`pm(500)`) concedes uniformly
4. Monotonic constraint: for any sequence of rounds 0..N, buyer offers are non-decreasing and seller asks are non-increasing
5. `derive_opening_offer` with 0 rejections returns bias-derived base
6. `derive_opening_offer` with 3 rejections returns a value shifted toward counterparty's likely reservation
7. `derive_opening_offer` respects `rejection_escalation_rate` parameter — higher rate means larger shift per rejection
8. `urgency_modulated_deadline` returns `base_patience` at zero urgency
9. `urgency_modulated_deadline` returns ≥ 1 at maximum urgency
10. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. All functions are pure and deterministic — same inputs always produce same outputs.
2. `generate_offer` for buyer always returns a value in `[1, reservation]`.
3. `generate_offer` for seller always returns a value in `[reservation, opening]`.
4. No floating-point arithmetic anywhere — all computation uses `u32`/`u64`/`Permille`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade_actions.rs` (new `#[cfg(test)] mod concession_tests`) — unit tests for `generate_offer`, `derive_opening_offer`, `urgency_modulated_deadline`

### Commands

1. `cargo test -p worldwake-systems -- concession` — targeted concession tests
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite

## Outcome

**Completed**: 2026-04-02

Implemented the bilateral-trade pure negotiation helpers in `crates/worldwake-systems/src/trade_actions.rs`:
- `generate_offer`
- `derive_opening_offer`
- `urgency_modulated_deadline`

The final implementation keeps the ticket's deterministic pure-function boundary while using integer-only easing approximations for Boulware, Linear, and Conceder concession behavior. Focused unit coverage now proves curve shape, monotonic buyer/seller concession, opening-offer bias and rejection shifting, escalation-rate sensitivity, and urgency-based deadline shrinkage.

**Deviations from original plan**:
1. The implementation uses deterministic integer easing approximations rather than floating-point Faratin math so the concession surface remains repo-compliant.
2. The new helpers remain staged scaffolding for later S10 integration tickets and are intentionally not wired into runtime trade flow yet.

**Verification results**:
1. `cargo test -p worldwake-systems generate_offer_ -- --nocapture`
2. `cargo test -p worldwake-systems derive_opening_offer -- --nocapture`
3. `cargo test -p worldwake-systems urgency_modulated_deadline -- --nocapture`
4. `cargo test -p worldwake-systems`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`
