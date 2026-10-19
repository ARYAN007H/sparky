import assert from "node:assert/strict";
import net from "node:net";
import test from "node:test";

import {
  coreLaunchSpec,
  devConnectUrl,
  mergeChildEnv,
  freePort,
  isLoopback,
  parseArgs,
  runStamp,
  startCoreChild,
  waitFor,
} from "../debug/web-ui-lib.mjs";

test("coreLaunchSpec isolates the core from operator credentials, storage, and dotenv", () => {
  const spec = coreLaunchSpec(
    {
      OPENHUMAN_BACKEND_API_KEY: "inherited",
      OPENHUMAN_BACKEND_SESSION_TOKEN: "inherited",
      OPENHUMAN_STORAGE_URL: "inherited",
      OPENHUMAN_DOTENV_PATH: "/operator/.env",
      RUST_LOG: "trace",
    },
    {
      repo: "/checkout/openhuman",
      workspace: "/private/run/workspace",
      mockUrl: "http://127.0.0.1:18500",
      token: "test-core-token",
    },
  );
  assert.equal(spec.cwd, "/private/run/workspace");
  assert.equal(spec.env.OPENHUMAN_WORKSPACE, spec.cwd);
  assert.equal(spec.env.OPENHUMAN_DOTENV_PATH, "/private/run/workspace/empty.env");
  assert.equal(spec.env.BACKEND_URL, "http://127.0.0.1:18500");
  assert.equal(spec.env.VITE_BACKEND_URL, "http://127.0.0.1:18500");
  assert.equal(spec.env.OPENHUMAN_CORE_TOKEN, "test-core-token");
  assert.equal(spec.env.OPENHUMAN_BACKEND_API_KEY, undefined);
  assert.equal(spec.env.OPENHUMAN_BACKEND_SESSION_TOKEN, undefined);
  assert.equal(spec.env.OPENHUMAN_STORAGE_URL, undefined);
  assert.equal(spec.env.RUST_LOG, "info");

  let launch;
  startCoreChild((...args) => (launch = args), "/private/openhuman-core", 18473, spec);
  assert.equal(launch[0], "core");
  assert.equal(launch[1], "/private/openhuman-core");
  assert.deepEqual(launch[2], ["serve", "--port", "18473"]);
  assert.equal(launch[3], spec.env);
  assert.equal(launch[4], spec.cwd);
});

test("mergeChildEnv overrides and removes inherited variables", () => {
  assert.deepEqual(
    mergeChildEnv(
      { KEEP: "parent", REMOVE: "credential", OVERRIDE: "parent" },
      { REMOVE: null, OVERRIDE: "private", ADD: "value" },
    ),
    { KEEP: "parent", OVERRIDE: "private", ADD: "value" },
  );
});

test("parseArgs defaults to an interactive, signed-in run that builds the core", () => {
  assert.deepEqual(parseArgs([]), {
    script: null,
    headed: false,
    keep: false,
    core: null,
    build: true,
    signIn: true,
    help: false,
  });
});

test("parseArgs reads every flag", () => {
  const opts = parseArgs([
    "--script",
    "s.mjs",
    "--core",
    "bin/core",
    "--headed",
    "--keep",
    "--no-build",
    "--no-sign-in",
  ]);
  assert.equal(opts.script, "s.mjs");
  assert.equal(opts.core, "bin/core");
  assert.equal(opts.headed, true);
  assert.equal(opts.keep, true);
  assert.equal(opts.build, false);
  assert.equal(opts.signIn, false);
  assert.equal(parseArgs(["-h"]).help, true);
});

test("parseArgs rejects unknown flags and missing values", () => {
  assert.throws(() => parseArgs(["--bogus"]), /unknown option: --bogus/);
  assert.throws(() => parseArgs(["--script"]), /--script requires a value/);
  assert.throws(() => parseArgs(["--core", "--keep"]), /--core requires a value/);
});

test("devConnectUrl carries the RPC URL and bearer in the fragment", () => {
  const url = new URL(devConnectUrl("http://localhost:1420", "http://127.0.0.1:7788/rpc", "tok"));
  assert.equal(url.pathname, "/__dev-connect");
  assert.equal(url.search, "");
  const fragment = new URLSearchParams(url.hash.slice(1));
  assert.equal(fragment.get("rpcUrl"), "http://127.0.0.1:7788/rpc");
  assert.equal(fragment.get("token"), "tok");
});

test("runStamp is a sortable local timestamp", () => {
  assert.equal(runStamp(new Date(2026, 9, 7, 9, 5, 3)), "20261007-090503");
});

test("freePort returns a port that can be bound", async () => {
  const port = await freePort();
  await new Promise((resolve, reject) => {
    const srv = net.createServer().listen(port, "127.0.0.1", () => srv.close(resolve));
    srv.on("error", reject);
  });
});

test("waitFor resolves once the probe passes and times out otherwise", async () => {
  let calls = 0;
  await waitFor(async () => ++calls >= 3, { timeoutMs: 1000, intervalMs: 1, what: "x" });
  assert.equal(calls, 3);
  await assert.rejects(
    waitFor(async () => false, { timeoutMs: 20, intervalMs: 5, what: "the thing" }),
    /timed out waiting for the thing/,
  );
});

test("isLoopback admits only local hosts and inline URLs", () => {
  assert.equal(isLoopback("http://localhost:1420/src/main.tsx"), true);
  assert.equal(isLoopback("http://127.0.0.1:7788/rpc"), true);
  assert.equal(isLoopback("ws://127.0.0.1:7788/socket.io/"), true);
  assert.equal(isLoopback("data:image/png;base64,AAAA"), true);
  assert.equal(isLoopback("https://panel.tinyhumans.ai/api/track"), false);
  assert.equal(isLoopback("https://raw.githubusercontent.com/x"), false);
});
