## Problem Description
STEP 46F spec gap: `GitAuthMaterial::pat` changes from `Option<String>` to `Option<Zeroizing<String>>` (required by Contract & Behavior #1 and POST_GATE), which changes `Option::as_deref()` from `Option<&str>` to `Option<&String>` and breaks `assert_eq!(resolved.pat.as_deref(), Some("..."))` and `assert_eq!(explicit.pat.as_deref(), Some("..."))` in `crates/backend_core/tests/workflow_integration.rs` (not in ALLOWED_FILES). Location: `crates/backend_core/tests/workflow_integration.rs:1428,1450`.
Lapse: L2 file outside ALLOWED_FILES
Kind: scope-lapse
Found during: STEP 46F (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
