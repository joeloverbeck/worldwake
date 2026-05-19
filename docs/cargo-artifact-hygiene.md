# Cargo Artifact Hygiene

Worldwake's Rust test surface produces large debug artifacts. Golden and integration test binaries can be hundreds of megabytes each because debug builds statically link workspace crates and carry debug information. Cargo also keeps hash-suffixed older binaries and incremental compilation state across repeated source changes.

This is expected Cargo behavior, but it can exhaust small WSL2 or VM disks after repeated `cargo test --workspace`, all-target clippy, golden inventory, or visualizer builds. Broad verification can still create large artifacts because integration test binaries statically link substantial workspace code, even after the AI golden suites were consolidated into entry binaries.

## What Grows

- `target/debug/deps/` stores compiled library, binary, and test artifacts. In this repo, the largest files are usually executable test binaries such as `golden_ai`, `integration_ai`, `observer`, `worldwake_cli`, or visualizer targets.
- `target/debug/incremental/` stores rustc incremental compilation caches. These speed up local rebuilds but can grow very large across many crate/target variants.
- `target/release/` is usually much smaller here, but release/profiling runs can still accumulate artifacts.

## Space-Conscious Verification

`./scripts/verify.sh` is the canonical local equivalent of CI and is now
space-conscious by default. It exports `CARGO_INCREMENTAL=0` for the broad
verification run, and the workspace `[profile.dev]` / `[profile.test]`
defaults in `Cargo.toml` set `debug = "line-tables-only"` — so every dev and
test binary keeps backtrace line numbers and panic locations while skipping
full DWARF (local-variable debug info).

```bash
./scripts/verify.sh
```

These defaults trade some rebuild speed and debugger detail for much smaller
local artifacts. They do not change source code, test selection, clippy
lints, or runtime assertions. Interactive iterative dev still gets
incremental compilation via Cargo's normal defaults — only `verify.sh`
disables it for the duration of the broad run.

If you need full DWARF for a debugger session, locally override with
`CARGO_PROFILE_DEV_DEBUG=full cargo build` (or `CARGO_PROFILE_TEST_DEBUG=full`
for test debugging). This will produce much larger artifacts in `target/` for
the duration of that session.

### Remaining bloat after defaults

Even with these defaults, `target/` after a clean `./scripts/verify.sh` can
still exceed safe thresholds on small WSL2 / VM disks. S154 consolidated the
AI golden and integration suites into `golden_ai` and `integration_ai`, which
removes the old per-scenario binary fan-out, but the remaining broad gate
still builds large statically linked test binaries across the workspace. Use
the "Cleanup Options" below after broad verification runs when disk space is
tight.

## Check Disk Use

Run this before or after broad verification when disk space is a concern:

```bash
du -sh target target/debug/deps target/debug/incremental target/release 2>/dev/null
```

To see the largest debug test binaries:

```bash
find target/debug/deps -maxdepth 1 -type f -perm -111 -printf '%s %p\n' | sort -nr | head -40
```

To see duplicate hash-suffixed test binaries by base name:

```bash
find target/debug/deps -maxdepth 1 -type f -perm -111 -printf '%f\n' \
  | sed -E 's/-[0-9a-f]{16}$//' \
  | sort \
  | uniq -c \
  | sort -nr \
  | head -40
```

## Cleanup Options

Use the least disruptive cleanup that solves the space problem:

1. Remove only incremental caches:

   ```bash
   rm -rf target/debug/incremental
   ```

   This often frees many gigabytes while preserving most compiled dependencies and test binaries. The next build will recreate incremental state.

2. Remove all Cargo artifacts:

   ```bash
   cargo clean
   ```

   This is the safest complete cleanup, but the next broad build will be slow.

3. Put especially heavy temporary verification in a disposable target directory:

   ```bash
   CARGO_TARGET_DIR=/tmp/worldwake-target cargo test -p worldwake-ai --test golden_ai scenario_diagnostics_fixture -- --ignored --test-threads=1
   rm -rf /tmp/worldwake-target
   ```

   Use this for one-off heavy proof runs when preserving the main `target/` cache is not important.

4. Disable incremental compilation for ad-hoc broad runs outside `verify.sh`:

   ```bash
   CARGO_INCREMENTAL=0 cargo test --workspace
   ```

   This can reduce new incremental cache growth, usually at the cost of slower rebuilds. `./scripts/verify.sh` already does this for you.

## Routine Discipline

- Prefer narrow focused commands while iterating; avoid broad workspace gates until the source diff is stable.
- `./scripts/verify.sh` is the canonical pre-PR gate and is space-conscious by default.
- After heavy golden/workspace verification, check `target/` size if working in WSL2, a VM, or a small disk.
- If `target/` grows beyond the local disk budget, clean `target/debug/incremental/` first; use `cargo clean` when duplicate debug binaries dominate.
- Do not commit generated target artifacts. `target/` remains local build output.

## WSL2 Note

Deleting files inside WSL2 frees space inside the Linux filesystem, but the Windows-side VHDX may not shrink immediately. Compacting the WSL2 virtual disk is a separate Windows maintenance step.
