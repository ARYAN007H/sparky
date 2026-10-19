import assert from "node:assert/strict";
import test from "node:test";

import { untrackedSkipLines } from "../ci/check-browser-test-skips.mjs";

test("browser skip guard requires an issue URL on the added skip line", () => {
  const patch = [
    "+  test.skip('tracked', async () => {}); // https://github.com/tinyhumansai/openhuman/issues/123",
    "+  it.skip('missing link', () => {}); // see issue #456",
    "+  describe.skip('also missing', () => {});",
    "+  this.skip(); // no issue link",
    "+  test.skip( // multiline call still needs link on this line",
    "+    'tracked only on the next line' // https://github.com/tinyhumansai/openhuman/issues/789",
    "+// test.skip('comment, not executable');",
    "-  test.skip('removed', () => {});",
  ].join("\n");

  assert.deepEqual(untrackedSkipLines(patch), [
    "it.skip('missing link', () => {}); // see issue #456",
    "describe.skip('also missing', () => {});",
    "this.skip(); // no issue link",
    "test.skip( // multiline call still needs link on this line",
  ]);
});
