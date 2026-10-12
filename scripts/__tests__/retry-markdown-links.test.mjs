import assert from "node:assert/strict";
import test from "node:test";
import {
  retryServiceUnavailable,
  serviceUnavailableUrls,
} from "../ci/retry-markdown-links.mjs";

const unavailable =
  "* [503] <https://example.test/docs#anchor> | Service Unavailable\n";
const missing = "* [404] <https://example.test/missing> | Not Found\n";

function checker(results) {
  const calls = [];
  return {
    calls,
    check: async (urls) => {
      calls.push(urls);
      return results[calls.length - 1];
    },
  };
}

test("transient 503 recovers after more than one retry and preserves fragments", async () => {
  const { calls, check } = checker([
    { status: 2, report: unavailable },
    { status: 0, report: "" },
  ]);
  const waits = [];
  assert.equal(
    await retryServiceUnavailable({
      initialReport: unavailable,
      check,
      wait: async (ms) => waits.push(ms),
    }),
    0,
  );
  assert.deepEqual(calls, [
    ["https://example.test/docs#anchor"],
    ["https://example.test/docs#anchor"],
  ]);
  assert.deepEqual(waits, [5000, 10000]);
});

test("persistent 503 remains a failure after bounded retries", async () => {
  const { calls, check } = checker(
    Array.from({ length: 3 }, () => ({ status: 2, report: unavailable })),
  );
  assert.equal(
    await retryServiceUnavailable({
      initialReport: unavailable,
      check,
      wait: async () => {},
    }),
    2,
  );
  assert.equal(calls.length, 3);
});

test("404 or mixed failures are not retried", async () => {
  for (const initialReport of [
    missing,
    unavailable + missing,
    "",
    "* [TIMEOUT] <https://example.test/> | Timeout\n",
  ]) {
    const { calls, check } = checker([]);
    assert.equal(
      await retryServiceUnavailable({
        initialReport,
        check,
        wait: async () => {},
      }),
      1,
    );
    assert.equal(calls.length, 0);
  }
});

test("a permanent error discovered on retry remains a failure", async () => {
  const { calls, check } = checker([{ status: 2, report: missing }]);
  assert.equal(
    await retryServiceUnavailable({
      initialReport: unavailable,
      check,
      wait: async () => {},
    }),
    2,
  );
  assert.equal(calls.length, 1);
});

test("retry inputs deduplicate repeated failures without dropping URL fragments", () => {
  assert.deepEqual(serviceUnavailableUrls(unavailable.repeat(3)), [
    "https://example.test/docs#anchor",
  ]);
});
