**Status**: COMPLETED

# E01: Project Scaffold, Deterministic Foundations & Core Types

## Epic Summary
Set up the Cargo workspace and the determinism-first rules that every later epic must obey.

Phase 1 is gated by replay, conservation, and invariant tests. That means E01 cannot be a generic “project bootstrap” epic; it has to lock in the data-model constraints that make T01, T02, T07, T08, and T09 realistically achievable.

## Phase
Phase 1: World Legality

## Crate
`worldwake-core` (workspace scaffolding touches all crates)

## Dependencies
None

## Why this revision exists
The original version set up IDs and marker traits, but it did not pin down the determinism policy that the later epics depend on. In particular, Phase 1 should not permit:
- unordered authoritative state that cannot be hashed canonically
- dynamic `TypeId` / `Any` component storage that is hard to serialize, diff, or replay
- floating-point values in authoritative state where fixed-point integers are enough
- “player” or `is_player` branches anywhere in simulation logic

Those choices are not implementation details. They decide whether Phase 1 can ever pass its own gate.

## Deliverables

### Cargo Workspace
- Root `Cargo.toml` with workspace members:
  - `crates/worldwake-core`
  - `crates/worldwake-sim`
  - `crates/worldwake-systems`
  - `crates/worldwake-ai`
  - `crates/worldwake-cli`
- Each crate has a minimal `Cargo.toml` and `src/lib.rs` (or `src/main.rs` for CLI)
- Root lint profile enables:
  - `forbid(unsafe_code)`
  - `deny(warnings)` in CI
  - `clippy::all`, `clippy::pedantic` in CI with a small allowlist checked into the repo

### Deterministic Data Policy
The authoritative simulation state for Phase 1 must use only deterministic, serializable data structures.

Allowed in authoritative state:
- `Vec`
- `Option`
- `BTreeMap`
- `BTreeSet`
- fixed-width integers
- enums / structs composed of the above

Disallowed in authoritative or hashed state:
- `HashMap`
- `HashSet`
- `TypeId`
- `Box<dyn Any>`
- raw pointer identity
- wall-clock time
- floating-point values unless there is a written exception and a canonicalization rule

This policy applies to:
- `worldwake-core` authoritative model types
- `worldwake-sim` scheduler / replay / save-load state
- any state included in event hashes or save files

### Core ID Types
Define stable newtypes / structs with `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + Deserialize`.

- `EntityId { slot: u32, generation: u32 }`
  - `slot` identifies the allocator slot
  - `generation` detects stale references after archival and reuse
- `Tick(u64)`
- `EventId(u64)`
- `Seed([u8; 32])`

### Shared Numeric Types
Define small newtypes used later by topology and inventory so Phase 1 does not drift into ad hoc primitives.

- `LoadUnits(u32)` for container capacity accounting
- `Permille(u16)` for fixed-point values in the range `0..=1000`
- `Quantity(u32)` for conserved lot counts where a semantic wrapper is useful

### Core Error Types
`WorldError` must be broad enough for all Phase 1 legality failures.

Required cases:
- `EntityNotFound(EntityId)`
- `ArchivedEntity(EntityId)`
- `ComponentNotFound { entity: EntityId, component_type: &'static str }`
- `DuplicateComponent { entity: EntityId, component_type: &'static str }`
- `InvalidOperation(String)`
- `InvariantViolation(String)`
- `InsufficientQuantity { entity: EntityId, requested: u32, available: u32 }`
- `CapacityExceeded { container: EntityId, requested: u32, remaining: u32 }`
- `ContainmentCycle { entity: EntityId, container: EntityId }`
- `ConflictingReservation { entity: EntityId }`
- `PreconditionFailed(String)`
- `CommitFailed(String)`
- `DeterminismViolation(String)`
- `SerializationError(String)`

`WorldError` must implement `std::error::Error + Send + Sync`.

### Core Traits
Keep the core traits minimal and serialization-friendly.

- `Component`: marker trait for authoritative component types  
  Required bounds:
  - `'static`
  - `Send + Sync`
  - `Clone + Debug`
  - `Serialize + DeserializeOwned`

- `RelationRecord`: marker trait for authoritative relation rows  
  Required bounds:
  - same as `Component`

Do **not** introduce a runtime-registered ECS trait object model here.

### Control Source
- `ControlSource` enum:
  - `Human`
  - `Ai`
  - `None`

Rules:
- there is no `Player` type in simulation code
- there is no `is_player` field or branch in simulation code
- changing control source changes only input capture / presentation, never simulation legality

### Shared Test Utilities
Add a small internal test support module used by later epics:
- deterministic seed helper
- canonical bytes helper for stable hashing tests
- repo policy tests for forbidden symbols in authoritative modules

## Invariants Enforced
- Spec 9.2: determinism is a first-class design constraint from the start
- Spec 9.12: no `Player` type or player-only simulation branch exists
- Spec 9.19: core authoritative types are serializable from day one

## Tests
- [ ] All 5 crates compile with `cargo build --workspace`
- [ ] `EntityId` generation detects stale references after archival + slot reuse
- [ ] `EntityId`, `Tick`, `EventId`, and `Seed` implement the required traits
- [ ] `Tick` supports arithmetic and ordering
- [ ] No `Player` type exists in authoritative crates
- [ ] No `is_player` branch exists in authoritative crates
- [ ] No `HashMap`, `HashSet`, `TypeId`, or `Box<dyn Any>` appear in authoritative model modules
- [ ] `WorldError` is `Send + Sync`
- [ ] Core types round-trip through serialization
- [ ] Canonical bytes for identical values are stable across repeated runs in the same build

## Acceptance Criteria
- `cargo test --workspace` passes
- `cargo clippy --workspace` is clean under the repo allowlist
- all authoritative core types are serializable and deterministically comparable
- no external ECS crate dependency
- no unordered collections in authoritative state

## Spec References
- Section 3.2 (no player type)
- Section 5.1 (tick-based time)
- Section 9.2 (determinism)
- Section 9.12 (player symmetry)
- Section 9.19 (save/load integrity)

## Outcome
- Completion date: 2026-03-09
- What actually changed:
  - Set up the Cargo workspace and crate scaffold for `worldwake-core`, `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, and `worldwake-cli`.
  - Added deterministic core types and policies in `worldwake-core`, including `EntityId`, `Tick`, `EventId`, `Seed`, `LoadUnits`, `Permille`, `Quantity`, `WorldError`, `ControlSource`, and serialization-friendly marker traits.
  - Added unit and policy tests covering stale-reference detection, serialization round-trips, deterministic canonical bytes, and forbidden-symbol checks for `Player`, `is_player`, `HashMap`, `HashSet`, `TypeId`, and `Box<dyn Any>`.
- Deviations from original plan:
  - None identified during archival.
- Verification results:
  - `cargo test -p worldwake-core` passed on 2026-03-09.
  - `cargo test --workspace` passed on 2026-03-09.
  - `cargo clippy --workspace` passed on 2026-03-09.
