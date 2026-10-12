#!/bin/bash
cd /home/enamakel/work/openhuman/worktrees/storage-7636-host-1
export RUST_MIN_STACK=16777216
cargo test --manifest-path Cargo.toml -p openhuman --lib storage::import 2>&1 | tail -80
