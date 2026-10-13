#!/usr/bin/env bash
#
# Headless core e2e on a configured storage backend (#7522 acceptance).
#
# Runs the headless Rust e2e suites against the mock backend with
# OPENHUMAN_STORAGE_URL set, so the core under test keeps everything in the
# chosen backend instead of the per-workspace default:
#
#   scripts/e2e-core.sh sqlite     # sqlite:<scratch dir>
#   scripts/e2e-core.sh mongodb    # needs TSD_MONGO_URL (a local replica set)
#
# MongoDB: start one with
#   vendor/tinyagents/vendor/tinystoragedrivers/.github/scripts/start-mongo-replset.sh 27017
# and export TSD_MONGO_URL=mongodb://127.0.0.1:27017/tsd_test?directConnection=true
# (the storage-mongodb workflow does exactly this). The URL must be loopback:
# the run creates a database of its own and writes to it.
#
# Extra arguments after the driver name are suites for scripts/test-rust-e2e.sh
# (`--suite <name>`); the default is the headless set below.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

driver="${1:-sqlite}"
[ $# -gt 0 ] && shift

# Suites that boot the core headless (JSON-RPC and the CLI) and so honour
# OPENHUMAN_STORAGE_URL. Suites that pin their own storage setup (the
# storage_*_e2e matrix, which opens each driver itself) are run by their own CI
# lanes.
DEFAULT_SUITES=(json_rpc_e2e cli_storage_url_e2e)

scratch="$(mktemp -d "$REPO_ROOT/target/e2e-core.XXXXXX" 2>/dev/null || { mkdir -p "$REPO_ROOT/target" && mktemp -d "$REPO_ROOT/target/e2e-core.XXXXXX"; })"
trap 'rm -rf "$scratch"' EXIT

case "$driver" in
  sqlite)
    export OPENHUMAN_STORAGE_URL="sqlite:$scratch/storage"
    ;;
  mongodb)
    base="${TSD_MONGO_URL:-}"
    if [ -z "$base" ]; then
      echo "[e2e-core] ERROR: TSD_MONGO_URL is not set (a loopback replica set is required)." >&2
      exit 2
    fi
    case "$base" in
      mongodb://localhost*|mongodb://127.0.0.1*|mongodb://\[::1\]*) ;;
      *) echo "[e2e-core] ERROR: TSD_MONGO_URL must name a local throwaway server." >&2; exit 2 ;;
    esac
    db="oh_e2e_core_$(date +%s)_$$"
    head="${base%%\?*}"
    query=""
    [ "$head" != "$base" ] && query="?${base#*\?}"
    authority="${head#mongodb://}"
    authority="${authority%%/*}"
    export OPENHUMAN_STORAGE_URL="mongodb://${authority}/${db}${query}"
    # The product build carries the sqlite driver only; add mongodb.
    export OPENHUMAN_E2E_EXTRA_FEATURES="storage-mongodb"
    ;;
  *)
    echo "usage: $0 [sqlite|mongodb] [--suite <name> ...]" >&2
    exit 2
    ;;
esac

# Shared backends keep secrets under the keyring master key and fail closed
# without one; CI has no OS keychain.
export OPENHUMAN_KEYRING_MASTER_KEY="${OPENHUMAN_KEYRING_MASTER_KEY:-$(printf '55%.0s' $(seq 1 32))}"

echo "[e2e-core] driver=$driver (OPENHUMAN_STORAGE_URL set)"
if [ $# -eq 0 ]; then
  args=()
  for suite in "${DEFAULT_SUITES[@]}"; do args+=(--suite "$suite"); done
  set -- "${args[@]}"
fi
exec bash "$SCRIPT_DIR/test-rust-e2e.sh" "$@"
