#!/usr/bin/env bash
# One durable JUnit report and count inventory per instrumented configuration.
set -euo pipefail
suite="$1"
shift
case "$suite" in '' | *[!a-zA-Z0-9._-]*) echo 'invalid nextest suite name' >&2; exit 64 ;; esac
report_dir="${NEXTEST_REPORT_DIR:-${PWD}/ci-out/nextest}"
mkdir -p "$report_dir"
report_dir="$(cd "$report_dir" && pwd)"
config="${report_dir}/${suite}.toml"
report="${report_dir}/${suite}.xml"
# Absolute paths keep reports out of the compiler target tree and survive its
# cleanup. A fresh config retains the repository's timeouts/resource groups.
node - "$config" "$report" <<'JS'
const fs = require('node:fs');
const [config, report] = process.argv.slice(2);
fs.writeFileSync(config, fs.readFileSync('.config/nextest.toml', 'utf8') + '\n[profile.ci.junit]\npath = ' + JSON.stringify(report) + '\n');
JS
rm -f "$report" "${report_dir}/${suite}.json"
args=()
for arg in "$@"; do
  args+=("$arg")
  if [ "$arg" = nextest ]; then args+=(--config-file "$config"); fi
done
rc=0
bash scripts/ci-cancel-aware.sh "${args[@]}" || rc=$?
report_rc=0
node scripts/ci/nextest-report.mjs "$report" "${report_dir}/${suite}.json" ${NEXTEST_EXPECTED_TARGETS:+"${NEXTEST_EXPECTED_TARGETS}"} || report_rc=$?
# A report/inventory failure must not hide the test runner's actual failure or
# cancellation. Successful executions must also produce nonempty reports.
[ "$rc" != 0 ] || rc="$report_rc"
exit "$rc"
