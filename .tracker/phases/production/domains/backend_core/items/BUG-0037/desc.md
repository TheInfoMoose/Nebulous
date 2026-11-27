## Problem Description
Phase 28 review finding (test defect, origin: plan/coder, STEP 46C). In crates/backend_core/tests/credential_hygiene.rs:281-305, the test asserts resolved.pat.is_none() without seeding a PAT under the same account in the global config/vault dir (or isolated NEBULOUS_CONFIG_DIR). The test would still pass if the fallback to get_pat_from_keyring were re-introduced.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
