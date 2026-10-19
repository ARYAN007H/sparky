import assert from "node:assert/strict";
import test from "node:test";

import { untrackedSkipLines } from "../ci/check-browser-test-skips.mjs";

const url = "https://github.com/tinyhumansai/openhuman/issues/123";

test("requires a same-line URL for direct, qualified, and runtime skip calls", () => {
  const source = [
    `it.skip('tracked', () => {}); // ${url}`,
    `test . skip /* reason */ ('tracked', () => {}); // ${url}`,
    `test.describe /* reason */ . skip ('tracked', () => {}); // ${url}`,
    `describe /* reason */ . skip ('tracked suite', () => {}); // ${url}`,
    `describe.\n  skip ('missing');`,
    `this /* runtime */ . skip /* reason */ ( );`,
    `test.step.skip('missing');`,
  ].join("\n");

  assert.deepEqual(untrackedSkipLines(source), [
    "6: skip ('missing');",
    "7: this /* runtime */ . skip /* reason */ ( );",
    "8: test.step.skip('missing');",
  ]);
});

test("ignores inert text but scans executable template interpolations", () => {
  const source = [
    "// test.skip('comment')",
    "/* test.skip('block comment') */",
    "const example = \"it.skip('string')\";",
    "const template = `describe.skip('template')`;",
    "const interpolated = `raw test.skip('ignored') ${test.skip('missing')}`;",
    `const linked = \`raw describe.skip('ignored') \${test.skip('tracked')}\`; // ${url}`,
    "const nested = `outer ${({ value: `inner ${this.skip()}` })}`;",
    "const removed = '- test.skip(\\\"old\\\")';",
    `test.skip('real'); // ${url}`,
  ].join("\n");

  assert.deepEqual(untrackedSkipLines(source), [
    "5: const interpolated = `raw test.skip('ignored') ${test.skip('missing')}`;",
    "7: const nested = `outer ${({ value: `inner ${this.skip()}` })}`;",
  ]);
});
