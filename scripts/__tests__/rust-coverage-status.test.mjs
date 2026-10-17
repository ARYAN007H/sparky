import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repoRoot = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);
const runner = path.join(repoRoot, "scripts", "ci", "rust-coverage.sh");

function extractFunction(name, scriptPath = runner) {
  const source = fs.readFileSync(scriptPath, "utf8");
  const start = source.indexOf(`${name}() {`);
  assert.notEqual(start, -1, `${name}() not found in ${runner}`);
  const end = source.indexOf("\n}\n", start);
  assert.notEqual(end, -1, `could not find the end of ${name}()`);
  return source.slice(start, end + 3);
}

function withRunnerFunctions(functions, preamble, body) {
  const script = [
    "set -euo pipefail",
    "exec 2>&1",
    preamble,
    ...functions.map((name) => extractFunction(name)),
    body,
  ].join("\n");
  try {
    return {
      status: 0,
      output: execFileSync("bash", ["-c", script], { encoding: "utf8" }),
    };
  } catch (error) {
    return {
      status: error.status,
      output: `${error.stdout ?? ""}${error.stderr ?? ""}`,
    };
  }
}

test("a failed raw coverage module is recorded and later modules still run", () => {
  const result = withRunnerFunctions(
    ["suite", "target_features_satisfied", "run_integration_target"],
    [
      'TEST_TARGET_REQS=""',
      'PRODUCT_FEATURES=""',
      "FAILED_SUITES=()",
      "log() { printf '%s\\n' \"$*\"; }",
      "raw_coverage_modules() { printf 'first\\nsecond\\n'; }",
      'llvm_cov() { for arg in "$@"; do case "$arg" in first::) return 7 ;; esac; done; echo RAN-LATER; }',
    ].join("\n"),
    'run_integration_target raw_coverage_all; printf "failed=%s\\n" "${FAILED_SUITES[@]}"',
  );

  assert.equal(result.status, 0, result.output);
  assert.match(result.output, /running raw coverage module: second/);
  assert.match(result.output, /RAN-LATER/);
  assert.match(result.output, /failed=raw_coverage_all::first \(exit 7\)/);
});

test("a cancelled raw coverage module stops the run at once", () => {
  const result = withRunnerFunctions(
    ["suite", "target_features_satisfied", "run_integration_target"],
    [
      'TEST_TARGET_REQS=""',
      'PRODUCT_FEATURES=""',
      "FAILED_SUITES=()",
      "log() { printf '%s\\n' \"$*\"; }",
      "raw_coverage_modules() { printf 'first\\nsecond\\n'; }",
      'llvm_cov() { for arg in "$@"; do case "$arg" in first::) return 143 ;; esac; done; echo RAN-LATER; }',
    ].join("\n"),
    "run_integration_target raw_coverage_all",
  );

  assert.equal(result.status, 143, result.output);
  assert.doesNotMatch(result.output, /running raw coverage module: second/);
  assert.doesNotMatch(result.output, /RAN-LATER/);
});

test("an integration target missing a required product feature is skipped", () => {
  const result = withRunnerFunctions(
    ["suite", "target_features_satisfied", "run_integration_target"],
    [
      "FAILED_SUITES=()",
      "TEST_TARGET_REQS=$'memory_artifacts_e2e\\tmemory-git'",
      'PRODUCT_FEATURES="channels,flows"',
      "log() { printf '%s\\n' \"$*\"; }",
      "llvm_cov() { echo RAN-TARGET; }",
    ].join("\n"),
    "run_integration_target memory_artifacts_e2e",
  );

  assert.equal(result.status, 0, result.output);
  assert.match(result.output, /skipping memory_artifacts_e2e/);
  assert.doesNotMatch(result.output, /RAN-TARGET/);
});

test("an integration target with all required product features runs", () => {
  const result = withRunnerFunctions(
    ["suite", "target_features_satisfied", "run_integration_target"],
    [
      "FAILED_SUITES=()",
      "TEST_TARGET_REQS=$'memory_artifacts_e2e\\tmemory-git'",
      'PRODUCT_FEATURES="memory-github,memory-git,flows"',
      "log() { printf '%s\\n' \"$*\"; }",
      "llvm_cov() { echo RAN-TARGET; }",
    ].join("\n"),
    "run_integration_target memory_artifacts_e2e",
  );

  assert.equal(result.status, 0, result.output);
  assert.match(result.output, /RAN-TARGET/);
});

test("headless Embed coverage forwards core features and preserves caller credentials", () => {
  const result = withRunnerFunctions(
    ["llvm_cov_product_facade", "llvm_cov_embed"],
    [
      'CORE_PRODUCT_FEATURES="channels"',
      "export OPENHUMAN_KEYRING_MASTER_KEY=operator-key",
      "export OPENHUMAN_KEYRING_MASTER_KEY_FILE=operator-file",
      'bash() { [[ "$OPENHUMAN_KEYRING_MASTER_KEY" == operator-key ]] || return 9; [[ "$OPENHUMAN_KEYRING_MASTER_KEY_FILE" == operator-file ]] || return 10; [[ "$*" == *"--features channels"* ]] || return 11; echo CORE-FEATURES; }',
    ].join("\n"),
    'llvm_cov_embed --no-report -p openhuman-embed; [[ "$OPENHUMAN_KEYRING_MASTER_KEY" == operator-key ]]; [[ "$OPENHUMAN_KEYRING_MASTER_KEY_FILE" == operator-file ]]; echo PARENT-UNCHANGED',
  );
  assert.equal(result.status, 0, result.output);
  assert.match(result.output, /CORE-FEATURES/);
  assert.match(result.output, /PARENT-UNCHANGED/);
});

for (const [label, scriptPath, invoke] of [
  [
    "mock runner",
    path.join(repoRoot, "scripts", "test-rust-with-mock.sh"),
    "cargo_test",
  ],
  ["coverage runner", runner, "llvm_cov"],
]) {
  test(`${label} selects the real isolated runtime-reaper regression`, () => {
    const script = [
      "set -euo pipefail",
      'suite() { shift; "$@"; }',
      `${invoke}() { printf '%s\\n' "$@"; }`,
      extractFunction("run_build_only_reaper_test", scriptPath),
      "run_build_only_reaper_test",
    ].join("\n");
    const args = execFileSync("bash", ["-c", script], { encoding: "utf8" })
      .trim()
      .split("\n");
    assert.ok(
      args.includes("--exact"),
      "the runtime singleton requires its own process",
    );
    assert.ok(
      args.includes(
        "agent::tinyagents::reaper::tests::a_build_only_runtime_is_swept_before_it_can_be_invoked",
      ),
      `the libtest selector must reach the declared regression: ${args.join(" ")}`,
    );
  });
}

for (const helper of ["llvm_cov", "llvm_cov_package", "llvm_cov_product_facade"]) {
  test(`${helper} selects nextest for private suites and rejects empty selections`, () => {
    const result = withRunnerFunctions(
      [helper],
      [
        'OH_COV_RUNNER=nextest',
        'CORE_PRODUCT_FEATURES=channels',
        'bash() { printf "%s\\n" "$*"; }',
      ].join("\n"),
      `${helper} --no-report -p openhuman-embed --all-targets`,
    );
    assert.equal(result.status, 0, result.output);
    assert.match(result.output, /cargo llvm-cov nextest --profile ci --no-tests=fail/);
    assert.match(result.output, /--no-report -p openhuman-embed --all-targets/);
  });
}

test("private core coverage runs sequentially without a second target directory", () => {
  const source = fs.readFileSync(runner, "utf8");
  assert.doesNotMatch(source, /UNIT_PID|cov-unit|export CARGO_TARGET_DIR/);
  assert.match(source, /suite "openhuman lib\+bins \(nextest\)" llvm_cov/);
  assert.match(source, /suite "openhuman-embed \(default\)" llvm_cov_package/);
  assert.match(source, /suite "openhuman-tinyhumans \(default\)" llvm_cov_package/);
});

test("coverage finalizer retains interrupted reports and the cancellation status", () => {
  const result = withRunnerFunctions(
    ["preserve_coverage"],
    [
      'OUT=interrupted.info',
      'COVERAGE_REPORTED=0',
      'log() { printf "%s\\n" "$*"; }',
      'llvm_cov() { printf "REPORT %s\\n" "$*"; return 9; }',
    ].join("\n"),
    'trap preserve_coverage EXIT; exit 143',
  );
  assert.equal(result.status, 143, result.output);
  assert.match(result.output, /REPORT report --lcov --output-path interrupted.info/);
});

function runMockedCoverage(exitCode) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "rust-coverage-runner-"));
  try {
    for (const folder of ["bin", "scripts/ci", "crates/openhuman-cli"])
      fs.mkdirSync(path.join(dir, folder), { recursive: true });
    const write = (name, content) => fs.writeFileSync(path.join(dir, name), content, { mode: 0o755 });
    write("scripts/ci/product-features.sh", "#!/bin/bash\necho channels\n");
    write("scripts/ci-cancel-aware.sh", '#!/bin/bash\nexec "$@"\n');
    write("scripts/ci/assert-coverage-presence.sh", "#!/bin/bash\nexit 0\n");
    write("crates/openhuman-cli/Cargo.toml", "");
    write("bin/rustup", "#!/bin/bash\nexit 1\n");
    write("bin/cargo", `#!/bin/bash
printf '%s\\n' "$*" >> "$MOCK_CALLS"
case "$*" in *"-p openhuman --lib --bins"*) exit "${exitCode}" ;; esac
`);
    let status = 0;
    try {
      execFileSync("bash", [runner], {
        cwd: dir,
        env: { ...process.env, PATH: `${dir}/bin:${process.env.PATH}`, MOCK_CALLS: `${dir}/calls`, OH_COV_RUNNER: "nextest", OH_COV_DOCTESTS: "0" },
        stdio: "pipe",
      });
    } catch (error) { status = error.status; }
    return { status, calls: fs.readFileSync(`${dir}/calls`, "utf8").trim().split("\n") };
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
}

test("private coverage executes every package configuration once and reports after a failed suite", () => {
  const { status, calls } = runMockedCoverage(7);
  assert.equal(status, 1);
  const selections = calls.filter((line) => line.includes("llvm-cov nextest"));
  assert.equal(selections.length, 8);
  assert.equal(new Set(selections).size, 8);
  assert.ok(selections.every((line) => line.includes("--no-tests=fail")));
  assert.ok(selections.some((line) => line.includes("-p openhuman-cli --tests -E kind(test)")));
  assert.equal(calls.filter((line) => line.includes("llvm-cov report")).length, 1);
});

test("private coverage cancels remaining suites but still preserves a report", () => {
  const { status, calls } = runMockedCoverage(143);
  assert.equal(status, 143);
  assert.equal(calls.filter((line) => line.includes("llvm-cov nextest")).length, 1);
  assert.equal(calls.filter((line) => line.includes("llvm-cov report")).length, 1);
});
