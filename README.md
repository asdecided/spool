# Spool

**Pick up where you left off.**

Spool gives local command workflows a persistent workspace and an honest record of what completed. Queue commands, keep their files and output, and inspect interrupted work before continuing.

**v0.0.1 preview:** Linux CLI and Rust library. Explicit commands run offline through bubblewrap. This is an execution foundation, not yet a model-driven agent or MCP host. AsDecided Core integration is planned and optional.

<img src="https://raw.githubusercontent.com/tcballard/omarchy-badges/75975e5b5bf75e7ede3764bcd2950046f7abfe2c/badges/v1/omarchy-app.svg" alt="Built for Omarchy — App" height="20">

Omarchy is a target platform; this badge is a community identity label. No live Omarchy version has been tested yet.

## Install on your Omarchy XPS

From a checkout of this candidate (Rust 1.89+):

```bash
sudo pacman -S --needed rust base-devel bubblewrap git
cargo install --path . --locked
export PATH="$HOME/.cargo/bin:$PATH"
spool --version
spool doctor
```

`doctor` must pass before trying execution. Spool does not fall back to unsandboxed commands. Run it as your normal user, never with sudo.

For a fresh checkout before publication, fetch the candidate branch:

```bash
git clone --branch feat/v0.0.1-recovery https://github.com/asdecided/spool.git
cd spool
```

Once v0.0.1 is published, use `--branch v0.0.1` instead. Source installation avoids prebuilt glibc compatibility assumptions. CI also produces an x86_64 Linux archive and checksum for release preparation.

## Try a persistent job

In a new empty directory:

```bash
spool init
spool new "XPS smoke test"
# Use the returned job ID; these examples assume 1.
spool queue 1 -- sh -c 'printf "hello from Spool\n" > hello.txt'
spool queue 1 -- cat hello.txt
spool resume 1
spool inspect 1
# Completed operations are skipped; this does not run them again.
spool resume 1
```

Files remain in `.spool/workspaces/1/`, mounted as `/work` in the sandbox. Your repository, home directory, credentials and journal are not mounted. System tools under `/usr` are read-only. Each command has a fresh temporary directory and environment; files in `/work` persist. Host networking is unavailable.

## Recovery and limits

- `spool exec 1 -- command args...` queues a command and runs pending work in order.
- `spool checkpoint 1 '{"next":"review"}'` retains explicit JSON state.
- `spool recover` marks interrupted dispatched operations **uncertain**. It never replays them.
- `spool resolve OP_ID succeeded 'evidence from inspecting the workspace'` records a manual conclusion. Use `failed` if the operation did not complete successfully. Do not guess.
- `spool events OP_ID` shows the operation history.
- Failed or uncertain operations block that job. After a failure, use a new job with a corrected command; no automatic retries are performed.
- `spool status` lists jobs; `spool inspect JOB_ID` shows checkpoints, operations and output.

Only one process can own a journal, including readers. While a command runs, other commands against that journal report busy. Ctrl+C or terminating Spool leaves dispatched work for recovery. Bubblewrap is configured to die with its parent; inspect the workspace and ensure work has stopped before resolving uncertainty.

Default timeout is 60 seconds. Set `SPOOL_TIMEOUT_SECONDS` to 1–3600. A timeout is uncertain because a command may have partially written files. Captured stdout and stderr are each capped at 64 KiB and excess bytes drained; truncation is reported. Workspace disk use, memory use and process count are **not quota-limited** in this preview. Use trusted commands and disposable workspaces, not hostile workloads.

Journal/checkpoint/output data is plaintext. Do not place secrets in command arguments or checkpoints. Persisted files and external effects are not rolled back. This preview does not claim exactly-once execution or power-loss recovery of arbitrary files. Do not delete the journal lock file while Spool is running.

## Build and test

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
# Requires working Linux user/network namespaces and bubblewrap:
cargo test --locked --test sandbox -- --ignored
# Run after installing the candidate:
bash scripts/xps-smoke.sh
```

[Architecture](docs/architecture.md) · [Roadmap](docs/roadmap.md) · [XPS acceptance and release checklist](docs/v0.0.1.md) · [Verification evidence](docs/verification.md)

## Uninstall and data

`cargo uninstall asdecided-spool` removes a Cargo-installed executable. A manually installed archive binary can be removed from `~/.local/bin/spool`. Neither removes job data. Keep `.spool/` to preserve your work; copy it only while no Spool command is running. Schema 1 reads and migrates the initial unversioned journal. Older binaries must not be used after migration.

Apache-2.0; see [LICENSE](LICENSE).
