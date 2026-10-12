import assert from "node:assert/strict";
import test from "node:test";
import { mkdtempSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import {
  validateCoverageDebt,
  validateAllowances,
  historicalFindings,
} from "../ci/storage-bypass-debt.mjs";
import {
  compare,
  scan,
  CURATED_VENDOR_ROOTS,
  collectFindings,
} from "../ci/check-storage-bypass.mjs";
const row = {
  rule: "json-write",
  path: "crates/openhuman-embed/src/state.rs",
  text: "tokio::fs::write(path, bytes);",
  occurrence: 1,
  classification: "unmigrated",
  reason: "Existing native backend migration debt at this exact source site.",
  issue: "https://github.com/tinyhumansai/openhuman/issues/7428",
};
const history = { baseline: [], extension: null, findings: [row] };
test("initial extension admits only exact existing canonical coverage", () => {
  assert.deepEqual(validateCoverageDebt([row], [], [row], history), []);
  const added = { ...row, path: "crates/openhuman-embed/src/new.rs" };
  assert.match(
    validateCoverageDebt([added], [], [added], history).join("\n"),
    /branch-added writer/,
  );
});
test("moving or copying a recorded writer cannot reuse its identity", () => {
  const moved = { ...row, path: "crates/openhuman-rpc/src/state.rs" };
  assert.equal(compare([moved], [row]).added.length, 1);
  const copied = { ...row, occurrence: 2 };
  assert.equal(compare([row, copied], [row]).added.length, 1);
});
test("extension records may shrink but never grow after establishment", () => {
  const old = { ...history, extension: [row] };
  assert.deepEqual(validateCoverageDebt([], [], [], old), []);
  const added = { ...row, text: "tokio::fs::write(other, bytes);" };
  assert.match(
    validateCoverageDebt([row, added], [], [row, added], old).join("\n"),
    /Extension growth forbidden/,
  );
});
test("removing a debt record does not grandfather a surviving writer", () => {
  assert.equal(compare([row], []).added.length, 1);
});
test("original core debt cannot move into the extension or grow", () => {
  const core = { ...row, path: "crates/openhuman-core/src/state.rs" };
  assert.match(
    validateCoverageDebt([core], [], [core], history).join("\n"),
    /cannot conceal original core debt/,
  );
  assert.match(
    validateCoverageDebt([core], [core], [], history).join("\n"),
    /Core baseline growth forbidden/,
  );
});
test("reasons, exact occurrence and issue links are mandatory", () => {
  for (const bad of [
    { reason: "" },
    { issue: "" },
    { classification: "approved" },
    { occurrence: 2 },
  ]) {
    assert.ok(
      validateCoverageDebt([row], [], [{ ...row, ...bad }], history).length,
    );
  }
});
test("an initial allowance cannot exempt a newly introduced branch writer", () => {
  const record = { ...row, site: row.text };
  assert.match(
    validateAllowances([row], [record], {
      rawFindings: [],
      allowances: null,
    }).join("\n"),
    /branch-added writer/,
  );
  assert.deepEqual(
    validateAllowances([row], [record], {
      rawFindings: [row],
      allowances: null,
    }),
    [],
  );
});
test("later allowances cannot grow or excuse a copied site", () => {
  const record = { ...row, site: row.text };
  const copy = { ...record, path: "crates/openhuman-embed/src/copy.rs" };
  assert.match(
    validateAllowances([{ ...row, path: copy.path }], [copy], {
      rawFindings: [row],
      allowances: [record],
    }).join("\n"),
    /growth forbidden/,
  );
});
test("curated vendor coverage excludes native drivers but not host adapter writes", () => {
  assert.ok(
    CURATED_VENDOR_ROOTS.every((p) => !p.includes("tinystoragedrivers")),
  );
  assert.ok(CURATED_VENDOR_ROOTS.some((p) => p.includes("tinymcp")));
  assert.ok(CURATED_VENDOR_ROOTS.some((p) => p.includes("run_ledger")));
  assert.equal(
    scan(
      "crates/openhuman-embed/src/adapter.rs",
      "tokio::fs::write(path, bytes);",
    ).length,
    1,
  );
});
test("new serializer and helper rules ignore quoted and commented fixture text", () => {
  assert.deepEqual(
    scan(
      "x.rs",
      'let fixture = "serde_json::to_writer(file, value);";\n/* fn write_atomic(path) {} */',
    ),
    [],
  );
});

test("missing canonical base history fails closed", () => {
  const root = mkdtempSync(join(tmpdir(), "storage-history-"));
  try {
    execFileSync("git", ["init"], { cwd: root, stdio: "ignore" });
    assert.throws(() =>
      historicalFindings(root, {
        scan,
        withoutAllowed: (_, x) => x,
        isTestFile: () => false,
        CURATED_VENDOR_ROOTS: [],
      }),
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
test("quoted database opener examples do not count as host persistence", () => {
  assert.deepEqual(
    scan(
      "crates/openhuman-embed/src/example.rs",
      'let example = r#"OtherStore::open(root.join("cache.db"))"#;',
    ),
    [],
  );
});


test("duplicate original core identities cannot inflate the retained baseline", () => {
  const core = { ...row, path: "crates/openhuman-core/src/state.rs" };
  assert.match(validateCoverageDebt([core], [core, core], [], {
    baseline: [core], extension: [], findings: [core],
  }).join("\n"), /Duplicate core baseline record/);
});

test("real checkouts refuse missing curated vendor source instead of shrinking debt", async () => {
  const root = mkdtempSync(join(tmpdir(), "storage-curated-source-"));
  try {
    execFileSync("git", ["init"], { cwd: root, stdio: "ignore" });
    mkdirSync(join(root, "crates"));
    await assert.rejects(collectFindings(root), /Missing curated source/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
