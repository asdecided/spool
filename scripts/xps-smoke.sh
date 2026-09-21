#!/usr/bin/env bash
set -euo pipefail
spool --version
spool doctor
test_dir=$(mktemp -d "${TMPDIR:-/tmp}/spool-smoke.XXXXXX")
printf 'Test workspace retained at %s\n' "$test_dir"
cd "$test_dir"
spool init
job=$(spool new 'XPS smoke')
spool queue "$job" -- sh -c 'printf hello >> proof.txt'
spool queue "$job" -- cat proof.txt
spool resume "$job"
spool resume "$job"
test "$(cat ".spool/workspaces/$job/proof.txt")" = hello
spool checkpoint "$job" '{"smoke":"passed"}'
spool inspect "$job"
slow=$(spool new 'Timeout should stop recovery')
spool queue "$slow" -- sh -c 'printf started > partial.txt; sleep 10'
if SPOOL_TIMEOUT_SECONDS=1 spool resume "$slow"; then
  printf 'ERROR: slow operation unexpectedly succeeded\n' >&2
  exit 1
fi
if spool resume "$slow"; then
  printf 'ERROR: uncertain operation was replayed\n' >&2
  exit 1
fi
spool inspect "$slow"
printf 'PASS: persistent files, no replay of completed work, and blocked uncertain work.\n'
