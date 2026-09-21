# Architecture proposal

## Boundaries

The Rust library owns job state and execution bookkeeping. The CLI is a thin client. An optional agent loop will choose work through a provider adapter. Other agent harnesses should also be able to drive the runtime without using that loop.

SQLite holds jobs, checkpoints and eventually operation records. Workspace files and large artifacts live outside the database with recorded references and hashes. A future job gets its own directory or Git worktree. Database commits cannot atomically commit remote API effects or arbitrary filesystem writes.

## Execution boundary

All capabilities flow through an execution broker: structured operation request, policy decision, recorded intent, adapter dispatch, recorded outcome. CLI process execution and MCP invocation are different adapters. Generic commands remain opaque process operations; parsing shell syntax does not prove their effects.

A JavaScript environment will compose tools and filter results, with explicit durable state APIs. It must have bounded time and memory and no ambient host capabilities. QuickJS is a candidate, not a selected security boundary. Host processes require separate OS-level restrictions. Tool calls from code must pass through the same broker.

The common representation is initially a typed operation envelope, not a compiler IL. Store job ID, operation ID, adapter, capability, arguments or protected references, policy version, timestamps and outcome evidence. Adapter metadata is trusted configuration. Imported MCP annotations are not sufficient to grant access or classify retries.

## Recovery contract

Operation lifecycle: prepared -> dispatched -> succeeded / failed / uncertain. Persist intent before dispatch. After a crash, a dispatched operation with no recorded outcome is uncertain, even if it may never have reached the remote service.

Reconcile using remote identifiers, supported idempotency keys or adapter-specific evidence. Never promise exactly-once remote effects without cooperation from the destination. Read retries can observe changed data; preserve previous results when reproducibility matters. Unknown effects stop that operation pending resolution. Local commands can also create remote effects and must not be assumed retryable.

Use a single writer/worker ownership mechanism before enabling execution. A restarted coordinator must establish whether an old child process is still alive before treating the job as stopped. Cancellation is a recorded request; killing a process cannot undo an external action.

## AsDecided integration

Use supported Core interfaces for retrieval and validation. Capture artifact identities, source revisions/content hashes and retrieval provenance. On resume, compare relevant decisions and present changes before proceeding where needed. Do not silently change the job's original evidence. Pin working code revision separately from decision revision.

Engineering decisions guide planning and review. Only explicit executable policies constrain runtime actions. Core remains optional and read-only through MCP.

## Current implementation

Schema 1 adds operations and append-only transition events to the initial jobs/checkpoints schema. Migration is transactional and newer schema versions are rejected. SQLite uses foreign keys, WAL and FULL synchronous mode. An OS file lock serialises journal owners and releases on process death. Explicit commands execute through bubblewrap with an offline per-job workspace, a wall-clock timeout and bounded captured output. Recovery marks dispatched actions uncertain and manual resolution records evidence. The preview has no model loop, MCP adapter, encryption, quotas on workspace disk/memory/process use, or claim of hostile-workload isolation.
