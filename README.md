# Spool

**Pick up where you left off.**

Spool is being built as a durable execution workspace for AI agents. Its goal is to keep working files, intermediate results and action history together so interrupted work can continue with an honest account of what completed and what remains uncertain.

CLI and MCP are tool interfaces. Spool will provide the execution environment around them. AsDecided Core supplies versioned engineering decisions; Spool will record which decisions informed a job and surface relevant changes when it resumes.

## Current status

Foundation prototype, version 0.0.1 in source. No release has been published.

Implemented: a Rust library and `spool` CLI backed by SQLite, with job creation, listing, inspection and append-only JSON checkpoints. Checkpoints survive closing and reopening the process.

**Not implemented yet:** tool execution, sandboxing, model integration, automatic resume, MCP adapters or AsDecided integration. A saved checkpoint is not a guarantee of safe execution recovery.

## Try the foundation

Requires Rust with edition 2024 support and a C compiler for bundled SQLite.

```bash
cargo build --locked
cargo install --path . --locked
spool init
spool new "Investigate a failing build"
# Use the job ID returned above (1 in a fresh workspace).
spool checkpoint 1 '{"next_step":"inspect logs","logs_saved":true}'
spool inspect 1
spool status
```

State lives in `.spool/journal.sqlite3` under the current directory. Run commands from the same directory. This prototype stores checkpoint data in plaintext; keep credentials out of checkpoints. The journal is local metadata, not an isolated job workspace yet.

## Direction

See [architecture](docs/architecture.md), [milestones](docs/roadmap.md) and [contributor guidance](AGENTS.md).

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```
