# Milestones

## 0. Foundation (implemented)

Rust library and CLI; SQLite jobs; append-only JSON checkpoints; reopening and validation tests; CI. This milestone proves basic persistence, not crash-safe tool execution.

## 1. Operation journal and recovery (implemented in v0.0.1 candidate)

Versioned schema migrations, operation state machine, atomic worker ownership, fake executor and explicit reconciliation. Test crashes before dispatch, during dispatch and after an external effect but before local completion. Verify uncertain effects are never automatically repeated. Prove recovery in separate processes.

## 2. Restricted local execution (initial candidate implementation; live acceptance pending)

Per-job workspaces, artifact storage, bounded outputs, timeouts and cancellation, actual platform sandboxing, secret references and permission checks. Start with one job/worker. Test surviving child processes and paths escaping the workspace. Unsupported platforms must fail closed for operations requiring sandboxing.

## 3. MCP and code composition

MCP adapter, capability discovery, sandboxed JavaScript, explicit state APIs and bounded result summaries. Every nested tool call uses the broker. Demonstrate data processing without loading entire results into model context.

## 4. Agent and decision integration

One model provider, budgets, explicit completion criteria, optional AsDecided retrieval and decision snapshots. Verify relevant decision changes are surfaced on resume. Other harnesses can use the execution library without this agent loop.

## 5. End-to-end acceptance

Run a repository task, interrupt the coordinator and resume it. Show persisted files/results, accurate pending work and reconciled external effects. Change a relevant decision while stopped and demonstrate the resulting review. Document remaining failure modes before considering v0.1.0.

Deferred: distributed execution, multi-agent scheduling, GUI, universal language compilation, arbitrary interpreter snapshots and claims of exactly-once external effects.
