import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { classifyElf } from "../ci/self-hosted/verify-linkage.mjs";

const helper = new URL("../ci/self-hosted/verify-runner.sh", import.meta.url);
const pin = readFileSync(
  new URL("../ci/self-hosted/guest-image.txt", import.meta.url),
  "utf8",
).trim();

test("private runner rejects missing, stale and unqualified guest image metadata", () => {
  const dir = mkdtempSync(join(tmpdir(), "guest-contract-"));
  try {
    const metadata = join(dir, "build-info");
    const run = () =>
      spawnSync("bash", [helper.pathname, "image"], {
        encoding: "utf8",
        env: { ...process.env, CI_IMAGE_BUILD_INFO: metadata },
      });
    assert.notEqual(run().status, 0);
    writeFileSync(metadata, "digest=sha256:stale\n");
    assert.notEqual(run().status, 0);
    writeFileSync(metadata, `image=${pin}\n`);
    assert.notEqual(run().status, 0);
    writeFileSync(metadata, `digest=${pin.split("@")[1]}\n`);
    assert.equal(run().status, 0);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("private workflow pins checkout, image and signed Tinymold without artifact caches", () => {
  const workflow = readFileSync(
    new URL("../../.github/workflows/ci-lanes.yml", import.meta.url),
    "utf8",
  );
  const privateJob = workflow.split("  # ── hosted:")[0];
  assert.match(privateJob, /group: openhuman-ex63/);
  assert.match(privateJob, /labels: \[self-hosted, openhuman-ex63\]/);
  assert.match(
    privateJob,
    /tinyhumansai\/\.github\/actions\/checkout@[a-f0-9]{40}/,
  );
  assert.match(privateJob, /submodules: recursive/);
  assert.match(privateJob, /exclude-submodules: wiki/);
  assert.match(privateJob, /fetch-depth: 0/);
  assert.match(privateJob, /submodule-depth: 0/);
  assert.match(privateJob, /allow-unsafe-pr-checkout: true/);
  assert.match(privateJob, /tinyhumansai\/tinymold@[a-f0-9]{40}/);
  assert.match(privateJob, /install: none/);
  assert.match(privateJob, /backend: remote/);
  assert.match(privateJob, /remote-signing-key:/);
  assert.match(privateJob, /remote-trusted-keys:/);
  assert.match(privateJob, /tinyhumansai\/tinymold\/save@[a-f0-9]{40}/);
  assert.doesNotMatch(
    privateJob,
    /Swatinem\/rust-cache|sccache|actions\/cache/,
  );
  assert.ok(
    privateJob.indexOf("verify-runner.sh") <
      privateJob.indexOf("- name: Start lanes"),
  );
  assert.match(
    privateJob,
    /name: Save Tinymold cache\n\s+if:.*always\(\).*\n\s+continue-on-error: true/,
  );
});

test("private routing requires both trusted identities and retains outsider hosted fallback", () => {
  const root = new URL("../../.github/workflows/", import.meta.url);
  const route = readFileSync(new URL("ci-fast.yml", root), "utf8");
  const hosted = readFileSync(new URL("ci-fast-hosted.yml", root), "utf8");
  assert.match(route, /pull_request_target:/);
  assert.match(route, /is_member "\$\{AUTHOR\}" && is_member "\$\{ACTOR\}"/);
  assert.match(route, /contents: read/);
  assert.match(route, /needs.route.outputs.member == 'true'/);
  assert.match(hosted, /profile: hosted/);
  assert.doesNotMatch(route, /contents: write|secrets: inherit/);
});

test("mold proof distinguishes instrumented binaries from ordinary executables", () => {
  assert.deepEqual(
    classifyElf(
      " [1] .interp PROGBITS\n [2] __llvm_prf_cnts PROGBITS",
      "mold 2.40.4",
    ),
    {
      executable: true,
      instrumented: true,
      mold: true,
    },
  );
  assert.deepEqual(classifyElf(" [1] .interp PROGBITS", "GCC 15"), {
    executable: true,
    instrumented: false,
    mold: false,
  });
  assert.equal(
    classifyElf(" [1] .text PROGBITS", "mold 2.40.4").executable,
    false,
  );
});

test("membership gate routes mixed identities, outsiders and bots away from EX63", () => {
  const route = readFileSync(
    new URL("../../.github/workflows/ci-fast.yml", import.meta.url),
    "utf8",
  );
  const body = route
    .split("        run: |\n")[1]
    .split("\n  lanes:")[0]
    .split("\n")
    .map((line) => line.slice(10))
    .join("\n");
  const dir = mkdtempSync(join(tmpdir(), "ci-membership-"));
  try {
    writeFileSync(
      join(dir, "curl"),
      '#!/usr/bin/env bash\nurl="${!#}"\ncase "$url" in */members/alice|*/members/bob) echo 204;; *) echo 404;; esac\n',
      { mode: 0o755 },
    );
    for (const [author, actor, association, token, expected] of [
      ["alice", "bob", "MEMBER", "fixture", true],
      ["alice", "outsider", "MEMBER", "fixture", false],
      ["outsider", "alice", "NONE", "fixture", false],
      ["dependabot[bot]", "dependabot[bot]", "NONE", "fixture", false],
      ["alice", "alice", "MEMBER", "", true],
      ["alice", "bob", "MEMBER", "", false],
    ]) {
      const output = join(dir, "output");
      writeFileSync(output, "");
      const result = spawnSync("bash", ["-c", body], {
        encoding: "utf8",
        env: {
          ...process.env,
          PATH: `${dir}:${process.env.PATH}`,
          ORG: "fixture",
          AUTHOR: author,
          ACTOR: actor,
          ASSOCIATION: association,
          MEMBERSHIP_TOKEN: token,
          GITHUB_OUTPUT: output,
        },
      });
      assert.equal(result.status, 0, result.stderr);
      assert.equal(
        readFileSync(output, "utf8").trim(),
        `member=${expected}`,
        `${author}/${actor}`,
      );
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("private validation and cache save have measured budgets without suppressing artifacts", () => {
  const workflow = readFileSync(
    new URL("../../.github/workflows/ci-lanes.yml", import.meta.url),
    "utf8",
  ).split("  # ── hosted:")[0];
  const saveStart = workflow.indexOf("      - name: Save Tinymold cache");
  const saveEnd = workflow.indexOf("\n      - name:", saveStart + 1);
  const save = workflow.slice(saveStart, saveEnd);
  assert.match(save, /timeout-minutes: 15/);
  const job = workflow.slice(
    workflow.indexOf("  ex63:"),
    workflow.indexOf("    steps:"),
  );
  assert.match(job, /timeout-minutes: 180/);
  assert.match(save, /continue-on-error: true/);
  const upload = workflow.slice(
    workflow.indexOf("      - name: Upload lane logs, timings and coverage"),
  );
  assert.match(upload, /if: always\(\)/);
  assert.ok(
    workflow.indexOf("      - name: Upload lane logs, timings and coverage") >
      saveEnd,
  );
  assert.doesNotMatch(upload, /tinymold\.outcome|success\(\)/);
});
