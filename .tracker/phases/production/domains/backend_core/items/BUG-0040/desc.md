## Problem Description
crates/backend_core/tests/credential_hygiene.rs:285-323 — vault_only_manager_does_not_fall_back_to_global_keyring_store mutates process-global NEBULOUS_CONFIG_DIR without a mutex while vault_ssh_key_is_never_written_to_disk (line 227) concurrently reads nebulous_config_dir() in the same test binary. Guard NEBULOUS_CONFIG_DIR access with a shared test Mutex (related to TASK-0116).

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
