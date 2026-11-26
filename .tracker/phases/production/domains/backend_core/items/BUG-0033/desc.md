## Problem Description
Phase 27 review (Findings 1 & 2): In crates/backend_core/src/models/registry.rs, with_registry_lock() opens projects.lock with create(true).write(true) and takes an exclusive try_write() flock even for read-only load_global_registry_from / credential_accounts_for_root_in, failing with PermissionDenied on read-only config dirs, creating projects.lock on empty writable dirs, and holding the process-global REGISTRY_LOCK mutex across the 30s poll loop. Additionally, load_registry_unlocked() now propagates save_registry_unlocked()? on unsafe-project pruning, failing reads of a read-only projects.toml.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
