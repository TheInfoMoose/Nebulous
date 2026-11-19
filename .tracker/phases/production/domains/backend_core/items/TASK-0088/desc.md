## Problem Description
`neb init` writes `.tracker/.gitignore` with only `.lock`; `.timer.toml` / `.timer.toml.bak` are ignored only by Nebulous's own root `.gitignore:6-7`, so new repos (seen in Orrery, ST_Rust_Age on 2026-11-18) would commit timer state — add both to the generated `.tracker/.gitignore`. Found during neb rollout.
Migrated from the plan file's Parked list (2026-11-18).

## Reproduction Steps
n/a (parked finding; see Problem Description)

## Fix Log & Agent Notes
