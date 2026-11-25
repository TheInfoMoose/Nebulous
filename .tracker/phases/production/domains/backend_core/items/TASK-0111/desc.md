## Problem Description
Source: docs/codebase-audit-report.md (AUD-15) — Validation: CONFIRMED.

Citations:
- crates/backend_core/Cargo.toml:10, 18, 21-22
- Cargo.lock:2872-2887 (confirmed via `cargo deny` duplicate warnings)

Details:
`backend_core` depends on `ssh-key = "0.7.0-rc.11"`, which pulls `sha2 0.11.1`, `aes-gcm 0.11.1`, `aead 0.6.1`, `aes 0.9.3`, and `rand_core 0.10.1` alongside direct `aes-gcm 0.10`, `sha2 0.10`, and `rand 0.8` dependencies, doubling the compiled crypto stack and introducing `-rc` churn risk.

Suggested Direction:
Pin stable `ssh-key 0.6` or align direct RustCrypto dependencies once `ssh-key 0.7.0` ships stably.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
