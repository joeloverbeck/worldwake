# E21CLIHUMCON-005: Display Helpers

## Summary

Implement shared formatting and entity resolution utilities used by all command handlers: entity name lookup, entity resolution from user input, needs bars, quantity formatting, etc.

## Depends On

None.

## Files to Touch

- `crates/worldwake-cli/src/display.rs` — **create**: all display/formatting helper functions

## Out of Scope

- Command handlers (006–012)
- REPL loop (003)
- Command enum (004)
- Changes to any crate other than `worldwake-cli`
- Any world mutation logic

## Deliverables

### Entity Display Name
```rust
pub fn entity_display_name(world: &World, id: EntityId) -> String
```
- If entity has `Name` component → return name string
- Else → return `"<EntityKind>#<slot>"` (e.g., `"Agent#3"`)

### Entity Resolution (User Input → EntityId)
```rust
pub fn resolve_entity(world: &World, input: &str) -> Result<EntityId, ResolveError>
```
Per spec line 60: try numeric ID → exact name match → prefix match → error with suggestions
1. Try parsing `input` as `u64` → look up slot in allocator → return if alive
2. Iterate all live entities, collect those with `Name` component
3. Exact match → return
4. Single prefix match → return
5. Multiple prefix matches → `ResolveError::Ambiguous(Vec<String>)` with matching names
6. No match → `ResolveError::NotFound(String)` with input echoed

### Needs Formatting
```rust
pub fn format_needs_bar(need_name: &str, current: Permille, band: &ThresholdBand) -> String
```
- Display format: `"hunger: ████░░░░░░ 420‰ [medium]"`
- Show filled/empty bar segments proportional to value
- Show urgency band label based on `ThresholdBand` thresholds

### Quantity Formatting
```rust
pub fn format_quantity(kind: CommodityKind, qty: Quantity) -> String
```
- `"5× Grain"`, `"1× Water"`

### Location Formatting
```rust
pub fn format_location(world: &World, entity_id: EntityId) -> String
```
- Look up placement relation → get place name
- Return `"at {place_name}"` or `"(no location)"` if not placed

### Control Source Formatting
```rust
pub fn format_control_source(cs: ControlSource) -> &'static str
```
- `Human` → `"[human]"`, `Ai` → `"[ai]"`, `None` → `"[none]"`

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli`:
  - `test_entity_display_name_with_name`: entity with Name component → returns name
  - `test_entity_display_name_without_name`: entity without Name → returns kind#slot format
  - `test_resolve_entity_by_id`: numeric input resolves to correct entity
  - `test_resolve_entity_exact_name`: exact name match works
  - `test_resolve_entity_prefix`: unique prefix match works
  - `test_resolve_entity_ambiguous`: multiple prefix matches → Ambiguous error with suggestions
  - `test_resolve_entity_not_found`: no match → NotFound error
  - `test_format_needs_bar`: produces readable bar with band label
  - `test_format_quantity`: correct formatting for various quantities
  - `test_format_control_source`: all three variants formatted correctly

### Invariants That Must Remain True
- All functions are pure read-only (no world mutation)
- Entity resolution is deterministic (BTreeMap iteration order)
- `cargo clippy -p worldwake-cli` passes with no warnings
