#!/usr/bin/env bash
# Fail closed on stale guests. Never install or silently select another compiler.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
image="$(cat "$root/scripts/ci/self-hosted/guest-image.txt")"
metadata="${CI_IMAGE_BUILD_INFO:-/etc/ohci/build-info}"
if [[ ! "$image" =~ ^ghcr.io/tinyhumansai/rust_ci@sha256:[0-9a-f]{64}$ ]] ||
  [[ ! -r "$metadata" ]] || ! grep -Fxq "digest=${image##*@}" "$metadata"; then
  echo '::error::Private guest image metadata is missing or does not match guest-image.txt' >&2
  exit 1
fi
echo "[ci][guest] verified $image"
[[ "${1:-}" != image ]] || exit 0

for tool in rustup cargo rustc clang readelf tinymold python3 node pnpm jq; do
  command -v "$tool" >/dev/null || { echo "::error::Missing preinstalled tool: $tool" >&2; exit 1; }
done
linker=/usr/local/bin/rust-mold-linker
[[ -x "$linker" && -x /opt/mold/bin/mold ]] || { echo '::error::Missing pinned mold linker' >&2; exit 1; }
channel="$(python3 -c 'import tomllib,sys; print(tomllib.load(open(sys.argv[1], "rb"))["toolchain"]["channel"])' "$root/rust-toolchain.toml")"
# rustup run --install is deliberately absent: an image without this toolchain fails.
rustup run "$channel" rustc --version
rustup run "$channel" cargo clippy --version
rustup run "$channel" cargo fmt --version
rustup run "$channel" cargo nextest --version
rustup run "$channel" cargo llvm-cov --version
python3 -c 'import sys,tomllib; assert sys.version_info >= (3,11)'
tinymold --version
/opt/mold/bin/mold --version
for key in TINYMOLD_REMOTE_URL TINYMOLD_REMOTE_AUTH TINYMOLD_REMOTE_SIGNING_KEY TINYMOLD_REMOTE_TRUSTED_KEYS; do
  [[ -n "${!key:-}" ]] || { echo "::error::Guest lacks signed cache configuration: $key" >&2; exit 1; }
  [[ "${!key}" != *$'\n'* && "${!key}" != *$'\r'* ]] || exit 1
  # The supervisor supplies credentials through MMDS. Publish only into the
  # runner's masked environment, so the action needs no repository secrets.
  printf '::add-mask::%s\n' "${!key}"
  printf '%s=%s\n' "$key" "${!key}" >> "$GITHUB_ENV"
done
{
  echo "RUSTUP_TOOLCHAIN=$channel"
  echo "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=$linker"
  echo "OPENHUMAN_CI_PYTHON=$(command -v python3)"
} >> "$GITHUB_ENV"
