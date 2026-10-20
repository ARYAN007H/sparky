import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const script = path.join(repoRoot, "app/scripts/e2e-web-shards.sh");
fs.mkdirSync(path.join(repoRoot, "target"), { recursive: true });
const fixtureRoot = fs.mkdtempSync(path.join(repoRoot, "target", "e2e-web-shards-test-"));

function run(args, env = {}) {
  return spawnSync("bash", [script, ...args], {
    cwd: repoRoot,
    encoding: "utf8",
    env: { ...process.env, ...env },
  });
}

test("four lanes assign each of 64 shards and nine special units once", () => {
  const units = [];
  for (let lane = 1; lane <= 4; lane += 1) {
    const result = run([`--lane=${lane}`, "--lanes=4", "--list"]);
    assert.equal(result.status, 0, result.stderr);
    units.push(...JSON.parse(result.stdout));
  }

  const shards = units.filter((unit) => unit.kind === "shard");
  const specials = units.filter((unit) => unit.kind !== "shard");
  assert.equal(shards.length, 64);
  assert.deepEqual(
    shards.map((unit) => unit.shard).sort((a, b) => Number.parseInt(a) - Number.parseInt(b)),
    Array.from({ length: 64 }, (_, index) => `${index + 1}/64`),
  );
  assert.equal(specials.length, 9);
  assert.deepEqual(
    specials.map((unit) => unit.index).sort((a, b) => a - b),
    Array.from({ length: 9 }, (_, index) => index),
  );
  assert.equal(new Set(units.map((unit) => `${unit.kind}:${unit.index}`)).size, 73);
});

test("invalid lane is rejected", () => {
  const result = run(["--lane=5", "--lanes=4", "--list"]);
  assert.equal(result.status, 2);
  assert.match(result.stderr, /exceeds lane count/);
});

test("failed units retain failure while later units run in distinct workspaces", () => {
  const runner = path.join(fixtureRoot, "runner.sh");
  fs.writeFileSync(
    runner,
    '#!/usr/bin/env bash\nprintf "%s|%s\\n" "$OPENHUMAN_WORKSPACE" "$*" >> "$CALL_LOG"\n[[ "$*" != *"--shard=4/64"* ]]\n',
  );
  fs.chmodSync(runner, 0o755);
  const callLog = path.join(fixtureRoot, "calls.log");
  const workspace = path.join(fixtureRoot, "workspaces");
  const result = run(["--lane=4", "--lanes=4"], {
    E2E_WEB_SESSION_RUNNER: `bash ${runner}`,
    CALL_LOG: callLog,
    OPENHUMAN_WORKSPACE: workspace,
  });
  assert.equal(result.status, 1, result.stderr);
  const calls = fs.readFileSync(callLog, "utf8").trim().split("\n");
  assert.equal(calls.length, 18);
  assert.ok(calls.some((call) => call.includes("--shard=4/64")));
  assert.ok(calls.some((call) => call.includes("--shard=64/64")));
  const paths = calls.map((call) => call.split("|")[0]);
  assert.equal(new Set(paths).size, paths.length);
  assert.equal(fs.readFileSync(path.join(workspace, "lane-4/receipts/shard-4.exit"), "utf8").trim(), "1");
  assert.equal(fs.readFileSync(path.join(workspace, "lane-4/receipts/shard-64.exit"), "utf8").trim(), "0");
});

test.after(() => fs.rmSync(fixtureRoot, { recursive: true, force: true }));
