#!/usr/bin/env node
// New browser-test skips must carry the tracking issue on the skip line.
// Existing skips are audited separately; this prevents the inventory growing
// without requiring a one-time, potentially misleading mass annotation.

import { execFileSync } from "node:child_process";

export function untrackedSkipLines(patch) {
  const failures = [];
  for (const line of patch.split("\n")) {
    if (!line.startsWith("+") || line.startsWith("+++")) continue;
    const source = line.slice(1);
    if (/^\s*\/\//.test(source)) continue;
    if (!/\b(?:it|test|describe)\.skip\s*\(/.test(source)) continue;
    if (!/https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/issues\/\d+/.test(source)) {
      failures.push(source.trim());
    }
  }
  return failures;
}

if (process.argv[1] && import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  const base = process.env.GITHUB_BASE_REF
    ? `origin/${process.env.GITHUB_BASE_REF}`
    : "origin/main";
  let patch;
  try {
    patch = execFileSync("git", ["diff", "--unified=0", `${base}...HEAD`, "--", "app/test/e2e/specs", "app/test/playwright/specs"], { encoding: "utf8" });
  } catch (error) {
    console.error(`Could not compare browser-test skips with ${base}: ${error.message}`);
    process.exit(2);
  }
  const failures = untrackedSkipLines(patch);
  if (failures.length) {
    console.error("New browser-test skips need a GitHub issue URL on the same line:\n" + failures.map((line) => `  ${line}`).join("\n"));
    process.exit(1);
  }
}
