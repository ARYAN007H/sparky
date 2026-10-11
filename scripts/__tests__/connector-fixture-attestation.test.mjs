import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const stage = fileURLToPath(
  new URL("../ci/stage-connector-test-module.mjs", import.meta.url),
);

test("locally built connector fixtures carry the exact artifact digest for confidential calls", () => {
  const root = mkdtempSync(join(tmpdir(), "connector-fixture-"));
  try {
    const source = join(root, "libtinyconnectors.so");
    const destination = join(root, "fixture");
    for (const bytes of ["first pinned build", "replacement pinned build"]) {
      writeFileSync(source, bytes);
      const result = spawnSync(process.execPath, [stage, source, destination], {
        encoding: "utf8",
      });
      assert.equal(result.status, 0, result.stdout + result.stderr);
      const fixture = result.stdout.trim();
      assert.equal(readFileSync(fixture, "utf8"), bytes);
      const digest = createHash("sha256").update(bytes).digest("hex");
      assert.equal(
        readFileSync(join(destination, "modules.toml"), "utf8"),
        `"libtinyconnectors.so" = "${digest}"\n`,
      );
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("native E2E installation attests the connector module before launching the app", () => {
  const root = mkdtempSync(join(tmpdir(), "native-connector-fixture-"));
  try {
    const workflow = readFileSync(
      new URL("../../.github/workflows/e2e-reusable.yml", import.meta.url),
      "utf8",
    );
    const block = workflow.match(
      /- name: Install native E2E test modules\n        run: \|\n([\s\S]*?)(?=\n      - name:)/,
    );
    assert.ok(block, "native module installation step exists");
    const command = block[1]
      .replace(/^          /gm, "")
      .replace("/opt/openhuman-test-modules", join(root, "modules"));
    const source = join(root, "vendor/tinyconnectors/target/release");
    mkdirSync(source, { recursive: true });
    const bytes = "pinned native connector module";
    writeFileSync(join(source, "libtinyconnectors.so"), bytes);
    mkdirSync(join(root, "scripts/ci"), { recursive: true });
    writeFileSync(
      join(root, "scripts/ci/stage-connector-test-module.mjs"),
      readFileSync(stage),
    );
    writeFileSync(
      join(root, "scripts/ci-cancel-aware.sh"),
      'test "$1" = cargo && test "$2" = build && test "$3" = --release\n',
    );
    const envFile = join(root, "env");
    const result = spawnSync("bash", ["-c", command], {
      cwd: root,
      encoding: "utf8",
      env: {
        ...process.env,
        GITHUB_RUN_ID: "fixture",
        GITHUB_RUN_ATTEMPT: "1",
        GITHUB_ENV: envFile,
      },
    });
    assert.equal(result.status, 0, result.stdout + result.stderr);
    const destination = join(root, "modules/fixture-1/tinyconnectors");
    assert.equal(
      readFileSync(join(destination, "libtinyconnectors.so"), "utf8"),
      bytes,
    );
    assert.equal(
      readFileSync(join(destination, "modules.toml"), "utf8"),
      `"libtinyconnectors.so" = "${createHash("sha256").update(bytes).digest("hex")}"\n`,
    );
    assert.equal(
      readFileSync(envFile, "utf8").trim(),
      `TINYCONNECTORS_TEST_MODULE=${destination}/libtinyconnectors.so`,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
