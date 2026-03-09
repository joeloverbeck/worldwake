//! Repository policy integration tests.
//!
//! These tests scan source files in worldwake-core/src/ for patterns that
//! violate the deterministic data policy or the no-Player invariant.

use std::fs;
use std::path::Path;

/// Read all `.rs` source files under `crates/worldwake-core/src/`.
fn source_files() -> Vec<(String, String)> {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_dir, &mut files);
    files
}

fn collect_rs_files(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let content = fs::read_to_string(&path).expect("read source file");
            let name = path.display().to_string();
            out.push((name, content));
        }
    }
}

/// Check that a forbidden pattern does not appear in any source line
/// (ignoring comments and the `test_utils` module).
fn assert_pattern_absent(pattern: &str, description: &str) {
    for (path, content) in source_files() {
        // Skip test_utils — it's test infrastructure, not authoritative state.
        if path.contains("test_utils") {
            continue;
        }
        for (line_no, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // Skip comments and doc comments.
            if trimmed.starts_with("//") {
                continue;
            }
            assert!(
                !line.contains(pattern),
                "Forbidden pattern `{pattern}` ({description}) found in {path}:{line_no}:\n  {line}"
            );
        }
    }
}

#[test]
fn no_player_type() {
    // Check for struct/enum/type named Player
    for (path, content) in source_files() {
        for (line_no, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            for keyword in ["struct Player", "enum Player", "type Player"] {
                assert!(
                    !trimmed.contains(keyword),
                    "Forbidden `{keyword}` found in {path}:{line_no}:\n  {line}"
                );
            }
        }
    }
}

#[test]
fn no_is_player() {
    assert_pattern_absent("is_player", "no player-specific branches");
}

#[test]
fn no_hash_map() {
    assert_pattern_absent("HashMap", "use BTreeMap instead");
}

#[test]
fn no_hash_set() {
    assert_pattern_absent("HashSet", "use BTreeSet instead");
}

#[test]
fn no_type_id() {
    assert_pattern_absent(
        "TypeId",
        "no runtime type identification in authoritative state",
    );
}

#[test]
fn no_dyn_any() {
    assert_pattern_absent("dyn Any", "no type-erased storage in authoritative state");
}

#[test]
fn no_box_dyn_any() {
    assert_pattern_absent(
        "Box<dyn Any>",
        "no type-erased storage in authoritative state",
    );
}

// --- Canonical bytes stability ---

#[test]
fn canonical_bytes_stability() {
    use worldwake_core::test_utils::canonical_bytes;
    use worldwake_core::{EntityId, Tick};

    let id = EntityId {
        slot: 42,
        generation: 7,
    };
    let tick = Tick(12345);

    let bytes_1 = canonical_bytes(&id);
    let bytes_2 = canonical_bytes(&id);
    assert_eq!(
        bytes_1, bytes_2,
        "canonical_bytes must be stable for EntityId"
    );

    let tick_bytes_1 = canonical_bytes(&tick);
    let tick_bytes_2 = canonical_bytes(&tick);
    assert_eq!(
        tick_bytes_1, tick_bytes_2,
        "canonical_bytes must be stable for Tick"
    );
}
