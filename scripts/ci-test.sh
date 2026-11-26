#!/usr/bin/env bash
# CI's Rust tests. When ci-failed.txt lists tests (restored on a re-run of a failed job), only
# those run; otherwise everything does. Afterwards ci-failed.txt holds this run's failures, so
# a re-run resumes at what failed. An empty list (all passed, or it failed before any test,
# such as a compile error) means the next run is a full one.
set -o pipefail
if [ -s ci-failed.txt ]; then
  mapfile -t names < ci-failed.txt
  echo "Re-running only the tests that failed last time: ${names[*]}"
  cargo test --locked --no-fail-fast -- --exact "${names[@]}" 2>&1 | tee test.log
else
  cargo test --locked --no-fail-fast 2>&1 | tee test.log
fi
status=$?
# For each test program, libtest prints "failures:" twice; the second lists the failed tests, indented four spaces.
awk '/^running [0-9]+ tests?$/ { n = 0 } /^failures:$/ { n++ } n == 2 && /^    [A-Za-z0-9_:]+$/ { print $1 }' test.log > ci-failed.txt
exit $status
