import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

test('live CortexDB harness opts in to the ignored integration test', async () => {
  const script = await readFile(
    new URL('../test-memory-cortexdb-live.sh', import.meta.url),
    'utf8',
  );

  assert.match(
    script,
    /cargo test -p openhuman-cli --test memory_cortexdb_live -- --ignored --nocapture/,
  );
});
