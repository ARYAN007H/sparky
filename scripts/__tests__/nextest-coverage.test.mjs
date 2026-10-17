import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const root = path.resolve(import.meta.dirname, "../..");
const validator = path.join(root, "scripts/ci/nextest-report.mjs");
function report(xml, expected = [], inventory) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "nextest-report-"));
  try {
    fs.writeFileSync(`${dir}/junit.xml`, xml);
    fs.writeFileSync(`${dir}/expected`, expected.join("\n"));
    if (inventory)
      fs.writeFileSync(`${dir}/inventory.json`, JSON.stringify(inventory));
    let status = 0;
    try {
      execFileSync(
        "node",
        [
          validator,
          `${dir}/junit.xml`,
          `${dir}/counts.json`,
          `${dir}/expected`,
          ...(inventory ? [`${dir}/inventory.json`] : []),
        ],
        { stdio: "pipe" },
      );
    } catch (error) {
      status = error.status;
    }
    return {
      status,
      counts: fs.existsSync(`${dir}/counts.json`)
        ? JSON.parse(fs.readFileSync(`${dir}/counts.json`))
        : undefined,
    };
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

test("nextest reports counts for each binary and rejects a missing expected target", () => {
  const xml =
    '<testsuites><testsuite name="openhuman-cli::first" tests="2" failures="1" errors="0" skipped="0"><testcase name="one"/><testcase name="two"/></testsuite></testsuites>';
  assert.equal(report(xml, ["first"]).status, 0);
  assert.equal(report(xml, ["first", "second"]).status, 1);
  assert.equal(report(xml).counts.tests, 2);
  assert.equal(report(xml).counts.failures, 1);
});

test("nextest rejects zero tests even when a binary report exists", () => {
  assert.equal(
    report(
      '<testsuites><testsuite name="openhuman-cli::first" tests="0"/></testsuites>',
      ["first"],
    ).status,
    1,
  );
});

test("nextest serialization retains JSON-RPC and multi-server suites", () => {
  const config = fs.readFileSync(
    path.join(root, ".config/nextest.toml"),
    "utf8",
  );
  assert.match(config, /binary\(\/\^json_rpc_e2e\$\/\)/);
  assert.match(config, /binary\(\/\^mcp_registry_multi_server\$\/\)/);
});

test("ignored-only binaries remain inventoried without requiring nonexistent JUnit suites", () => {
  const xml =
    '<testsuites><testsuite name="openhuman-cli::active" tests="1"/></testsuites>';
  const inventory = {
    "rust-suites": {
      active: {
        "binary-name": "active",
        testcases: {
          runs: { ignored: false, "filter-match": { status: "matches" } },
        },
      },
      ignored: {
        "binary-name": "ignored",
        testcases: {
          live: {
            ignored: true,
            "filter-match": { status: "mismatch", reason: "ignored" },
          },
        },
      },
    },
  };
  assert.equal(report(xml, ["active", "ignored"], inventory).status, 0);
  assert.equal(
    report(xml, ["active", "ignored", "missing"], inventory).status,
    1,
  );
  inventory["rust-suites"].ignored.testcases.live.ignored = false;
  inventory["rust-suites"].ignored.testcases.live["filter-match"].status =
    "matches";
  assert.equal(report(xml, ["active", "ignored"], inventory).status, 1);
});
