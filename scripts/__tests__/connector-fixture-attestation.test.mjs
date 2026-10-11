import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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
