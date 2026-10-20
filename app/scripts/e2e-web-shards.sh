#!/usr/bin/env bash
# Run the isolated Playwright logical units assigned to one hosted lane.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

lane=""
lanes="4"
list=0
while (($#)); do
  case "$1" in
    --lane=*) lane="${1#*=}" ;;
    --lanes=*) lanes="${1#*=}" ;;
    --list) list=1 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

[[ "$lane" =~ ^[1-9][0-9]*$ && "$lanes" =~ ^[1-9][0-9]*$ ]] || { echo "--lane and --lanes must be positive integers" >&2; exit 2; }
(( lane <= lanes )) || { echo "lane $lane exceeds lane count $lanes" >&2; exit 2; }

special_files=(
  "test/playwright/specs/connections-tab-aliases.spec.ts"
  "test/playwright/specs/connections-tab-deeplinks.spec.ts"
)
special_titles=(
  "a burst of typing issues ONE catalog search"
  "a pause between words issues a second search"
  "a query with no matches leaves no catalog rows"
  "the typed text is preserved in the box while results load"
  "offers Install, and the button is enabled before it is pressed"
  "disables the button and shows Installing while the RPC is in flight"
  "a failed install re-enables the button rather than stranding it"
)

if (( list )); then
  first=1
  printf '['
  for i in "${!special_files[@]}"; do
    if (( i % lanes + 1 == lane )); then
      (( first )) || printf ','
      printf '{"kind":"special-file","index":%d,"file":"%s"}' "$i" "${special_files[$i]}"
      first=0
    fi
  done
  for i in "${!special_titles[@]}"; do
    unit=$((i + ${#special_files[@]}))
    if (( unit % lanes + 1 == lane )); then
      escaped="${special_titles[$i]//\\/\\\\}"
      escaped="${escaped//\"/\\\"}"
      (( first )) || printf ','
      printf '{"kind":"special-title","index":%d,"file":"test/playwright/specs/skills-search-install.spec.ts","title":"%s"}' "$unit" "$escaped"
      first=0
    fi
  done
  for ((shard=1; shard<=64; shard++)); do
    if (( (shard - 1) % lanes + 1 == lane )); then
      (( first )) || printf ','
      printf '{"kind":"shard","index":%d,"shard":"%d/64"}' "$shard" "$shard"
      first=0
    fi
  done
  printf ']\n'
  exit 0
fi

workspace_root="${OPENHUMAN_WORKSPACE:-${RUNNER_TEMP:-$REPO_ROOT/target}/openhuman-playwright}"
export OPENHUMAN_E2E_ALLOW_EPHEMERAL_KEY="${OPENHUMAN_E2E_ALLOW_EPHEMERAL_KEY:-1}"
if [[ -n "${E2E_WEB_SESSION_RUNNER:-}" ]]; then
  read -r -a runner_cmd <<< "$E2E_WEB_SESSION_RUNNER"
else
  runner_cmd=(bash app/scripts/e2e-web-session.sh)
fi

escape_regex() {
  printf '%s' "$1" | sed 's/[][(){}.*+?^$|\\]/\\&/g'
}

mkdir -p "$workspace_root/lane-$lane/receipts"
receipt_dir="$workspace_root/lane-$lane/receipts"
overall=0
run_unit() {
  local unit="$1"; shift
  local workspace="$workspace_root/lane-$lane/$unit"
  local receipt="$receipt_dir/$unit.exit"
  mkdir -p "$workspace"
  workspace="$(cd "$workspace" && pwd)"
  local output="$workspace/test-results"
  export OPENHUMAN_WORKSPACE="$workspace"
  local status=0
  bash scripts/ci-cancel-aware.sh "${runner_cmd[@]}" "$@" --output="$output" || status=$?
  printf '%s\n' "$status" > "$receipt"
  echo "unit=$unit exit=$status receipt=$receipt"
  (( status == 0 )) || overall=1
}

for i in "${!special_files[@]}"; do
  if (( i % lanes + 1 == lane )); then
    run_unit "special-$i" "${special_files[$i]}"
  fi
done
for i in "${!special_titles[@]}"; do
  unit=$((i + ${#special_files[@]}))
  if (( unit % lanes + 1 == lane )); then
    run_unit "special-$unit" test/playwright/specs/skills-search-install.spec.ts -g "$(escape_regex "${special_titles[$i]}")"
  fi
done
for ((shard=1; shard<=64; shard++)); do
  if (( (shard - 1) % lanes + 1 == lane )); then
    run_unit "shard-$shard" --shard="$shard/64" --grep-invert='Connections (aliases|deep links)|Skills explorer'
  fi
done
exit "$overall"
