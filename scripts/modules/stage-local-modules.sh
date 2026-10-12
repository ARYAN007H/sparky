#!/usr/bin/env bash
# Helpers the local Rust test runners (scripts/test-rust-with-mock.sh,
# scripts/test-rust-e2e.sh) source to stage native modules.
#
# Released modules (the default, same as CI): `stage_release_module <id>` downloads
# the registry-pinned archive, verifies its sha256 and puts it on
# OPENHUMAN_MODULE_PATH. Source-built modules: with OPENHUMAN_MODULE_SOURCE=source,
# `stage_source_modules <id>...` builds them from the vendored submodules with
# scripts/modules/build.mjs and exports OPENHUMAN_MODULE_SOURCE_DIR plus the
# search path. tinyconnectors has no test release, so it is always built.
#
# Needs REPO_ROOT, module_ext and wallet_platform (or module_platform) set by the caller.

local_module_source_is_source() {
  [ "${OPENHUMAN_MODULE_SOURCE:-}" = "source" ]
}

# stage_release_module <id> <host-platform> <ext>; prints the staged directory.
stage_release_module() {
  local id="$1" platform="$2" ext="$3"
  local asset url filename sha dir archive
  asset="$(node "$REPO_ROOT/scripts/ci/self-hosted/test-module-assets.mjs" "$platform" "$id")"
  IFS=$'\t' read -r _id url filename sha <<< "$asset"
  dir="$REPO_ROOT/target/test-modules/$id/$sha"
  archive="$dir/$filename"
  if ! compgen -G "$dir/*.$ext" >/dev/null; then
    echo "Downloading the checksum-pinned ${id} test module ..." >&2
    mkdir -p "$dir"
    curl --fail --location --silent --show-error "$url" --output "$archive"
    if command -v shasum >/dev/null 2>&1; then
      echo "${sha}  $archive" | shasum -a 256 -c >&2
    else
      echo "${sha}  $archive" | sha256sum --check >&2
    fi
    tar --no-same-owner -xzf "$archive" -C "$dir"
  fi
  printf '%s\n' "$dir"
}

# stage_source_modules <id>... : build from the pinned submodules and eval the
# exports. OPENHUMAN_MODULE_PATH then carries every built directory.
stage_source_modules() {
  local exports
  exports="$(node "$REPO_ROOT/scripts/modules/build.mjs" "$@" --print-env)" || return 1
  eval "$exports"
}
