# E21CLIHUMCON-012: Persistence Commands (save, load)

## Summary

Implement `save <path>` and `load <path>` commands, delegating to existing `worldwake_sim::save()` and `worldwake_sim::load()`.

## Depends On

- E21CLIHUMCON-003 (REPL loop)
- E21CLIHUMCON-004 (command enum)

## Files to Touch

- `crates/worldwake-cli/src/handlers/persistence.rs` — **create**: `handle_save()`, `handle_load()`
- `crates/worldwake-cli/src/handlers/mod.rs` — **modify**: wire `Save`, `Load` variants

## Out of Scope

- Other command handlers (006–011)
- Modifying the save/load implementation in `worldwake-sim`
- Save file format changes
- Auto-save functionality
- Changes to any crate other than `worldwake-cli`

## Deliverables

### `handle_save(sim: &SimulationState, path: &str)`
1. Call `worldwake_sim::save(sim, path)` (or equivalent API)
2. On success: print `"Saved to {path}"`
3. On error: print error message (I/O error, serialization error)

### `handle_load(sim: &mut SimulationState, path: &str)`
1. Call `worldwake_sim::load(path)` (or equivalent API)
2. On success: replace `*sim` with loaded state
3. Print `"Loaded from {path} — tick {t}"`
4. On error: print error message, leave current state unchanged

### Error Handling
- File not found → clear error message
- Corrupt/invalid save file → clear error message, current state preserved
- Permission denied → clear error message

### Notes
- The `AgentTickDriver` (AI state) is NOT part of the save file — it reconstructs from world state
- After load, the driver should be reset or its caches invalidated (coordinate with REPL state)
- `ReplState.last_affordances` should be cleared after load (stale references)

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli`:
  - `test_save_creates_file`: save to temp path → file exists
  - `test_save_load_roundtrip`: save → load → tick matches, entity count matches
  - `test_load_nonexistent_file`: load from nonexistent path → error, state unchanged
  - `test_load_invalid_file`: load from invalid file → error, state unchanged
  - `test_load_clears_repl_state`: after load, last_affordances is empty

### Invariants That Must Remain True
- Save format is bincode (existing `worldwake-sim` format)
- Load failure never corrupts current state (atomic replacement)
- Current tick is correct after load
- `cargo clippy -p worldwake-cli` passes with no warnings
