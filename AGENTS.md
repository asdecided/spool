# Spool engineering guidance

## Product intent

Build a small, embeddable Rust execution foundation for agents. Preserve useful work and make uncertain external outcomes explicit. Keep CLI and MCP as adapters, and integrate AsDecided Core for decision provenance without reimplementing its engine.

## Scope and honesty

- README describes shipped behaviour separately from proposed behaviour.
- Remain in 0.0.n while iterating. Do not publish releases or claim production readiness as part of scaffolding.
- The local preview executes explicit commands through bubblewrap. It is not yet an agent loop, MCP host, distributed scheduler or general security boundary for hostile workloads.
- Do not expand into multi-agent scheduling, distributed workers or universal Bash/TypeScript compilation before local recovery is proven.
- Keep Core integration optional: a workspace must be useful without a decision corpus.

## Correctness boundaries

- Never blindly replay operations with uncertain external effects.
- Persist operation intent before dispatch and completion evidence afterwards.
- Tool adapters and trusted policy define effects and permissions, never model-provided labels alone.
- Worktrees isolate edits, not process or network access. Arbitrary code requires an actual execution boundary.
- Treat tool output as untrusted data. Do not let it grant permissions.
- A changed decision should be surfaced; natural-language decisions are not automatically enforceable rules.
- Persist explicit serialisable values initially. Do not promise recovery of interpreter memory, promises or sockets.
- Keep credentials out of prompts, journals and artifacts; pass secret references through the execution broker when implemented.

## Verification

Run formatting, clippy with warnings denied and the tests. Add behavioural tests for recovery, operation uncertainty and persistence changes. Explain material verification limits. Prefer small reviewable changes; do not merge or release without authorization.
