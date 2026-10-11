import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync, existsSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const installer = fileURLToPath(new URL("../ci/self-hosted/install-test-modules.sh", import.meta.url));

for (const [description, resolverExit] of [["failed", 7], ["empty", 0]]) {
  test(`${description} module asset resolution refuses to publish fixture paths`, () => {
    const root = mkdtempSync(join(tmpdir(), "module-install-"));
    try {
      const bin = join(root, "bin");
      mkdirSync(bin);
      writeFileSync(join(bin, "node"), `#!/bin/sh\nexit ${resolverExit}\n`, { mode: 0o755 });
      // No builds or downloads are needed to verify failure admission.
      for (const command of ["git", "cargo", "rustc", "curl"]) {
        writeFileSync(join(bin, command), `#!/bin/sh\ntouch '${root}/attempted'\nexit 0\n`, { mode: 0o755 });
      }
      const envFile = join(root, "modules.env");
      const result = spawnSync("bash", [installer, join(root, "modules"), envFile], {
        encoding: "utf8",
        env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, CI_CACHE_DIR: "" },
      });
      assert.notEqual(result.status, 0, result.stdout + result.stderr);
      assert.equal(existsSync(join(root, "attempted")), false, "resolve pins before building or downloading");
      assert.equal(existsSync(envFile), false, "unresolved modules must not be exported");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
}
