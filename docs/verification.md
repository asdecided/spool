# Candidate verification

Inputs are identified by SHA-256 in [verification-inputs.json](verification-inputs.json). These hashes identify the tested code, not a promise of compatibility.

## Reproduced locally

Environment: x86_64 Ubuntu 24.04 container, Rust 1.98.1, Cargo 1.98.1.

- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked -- -D warnings`: pass.
- `cargo test --locked`: seven ordinary tests pass. The ignored crash-worker helper is invoked and killed by the parent recovery test at four interruption points.
- Recovery test: second process denied ownership; lock released on process kill; interrupted dispatch becomes uncertain; completed work remains completed; repeated recovery changes nothing; uncertain work refuses resume; evidence-backed manual resolution retained.
- Legacy unversioned journal migration retains checkpoints. Newer schema is refused.
- Output reader retains at most 64 KiB per stream and drains excess.
- `bash -n scripts/package.sh scripts/xps-smoke.sh`: pass.

## Blocked locally

`spool doctor` / real bubblewrap execution: container policy prevents creation of the network namespace (`NETLINK_ROUTE socket: Operation not permitted`). No restrictions were relaxed. The opt-in real sandbox test is a required CI step on Ubuntu 22.04, not counted as locally passed.

## Not run locally

Live Omarchy XPS installation, Arch package-manager installation, actual Ctrl+C recovery of a sandbox workload, filesystem power-loss tests, hostile-workload isolation, and memory/disk/process quota tests. This release has no such quotas. Native Arch packaging is not included; install from source or use the CI archive.

The GitHub PR carries current CI results for the exact commit. Check those before publication. Follow [v0.0.1 acceptance](v0.0.1.md) on the XPS.
