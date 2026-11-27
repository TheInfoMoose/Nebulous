## Problem Description
Phase 28 review finding (regression, origin: plan, STEP 46D). In crates/backend_core/src/git_sync/mod.rs:1013-1026, when configured_remote_url is Some(ssh_or_non_https_url), auth has both pat and SSH credentials, and allowed_types includes both USER_PASS_PLAINTEXT and SSH_KEY/SSH_MEMORY/SSH_CUSTOM, git_credential_for_remote returns Err(host mismatch) immediately instead of falling through to the SSH authentication branch at line 1037. Only return the host-mismatch error when configured_remote_url is https:// (or when no other allowed credential type can satisfy the request).

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
