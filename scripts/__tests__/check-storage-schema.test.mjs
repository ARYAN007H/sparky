import assert from 'node:assert/strict';
import test from 'node:test';

import { lint, render } from '../ci/check-storage-schema.mjs';

const collection = (name, extra = {}) => ({
  name,
  indexes: [],
  ...extra,
});

test('a well-formed schema is clean', () => {
  const dump = {
    collections: [
      collection('cron.jobs', {
        indexes: [{ name: 'by_time', fields: ['next_run_ms'], unique: false }],
        ttl_field: 'expires_ms',
        search: { fields: ['name', 'description'] },
      }),
    ],
  };
  assert.deepEqual(lint(dump), []);
});

test('collection names must be <owner>.<entity> in snake_case', () => {
  const problems = lint({
    collections: [
      collection('jobs'),
      collection('Cron.Jobs'),
      collection('flows.runs-v2'),
    ],
  });
  assert.equal(problems.length, 3, problems.join('\n'));
});

test('known legacy migration collections remain inventoried', () => {
  assert.deepEqual(lint({
    collections: [collection('remix_markers'), collection('tinymemes_documents')],
  }), []);
});

test('duplicate collection and index names are refused', () => {
  const problems = lint({
    collections: [
      collection('cron.jobs', {
        indexes: [
          { name: 'by_time', fields: ['created_ms'] },
          { name: 'by_time', fields: ['updated_ms'] },
        ],
      }),
      collection('cron.jobs'),
    ],
  });
  assert.ok(problems.some(p => p.includes('collection "cron.jobs" is declared more than once')));
  assert.ok(problems.some(p => p.includes('index name "by_time" is empty or duplicated')));
});

test('indexed time fields must be epoch milliseconds', () => {
  const problems = lint({
    collections: [collection('cron.jobs', {
      indexes: [{ name: 'by_time', fields: ['created_at', 'next_run_ms'] }],
      ttl_field: 'expires_at',
    })],
  });
  assert.equal(problems.length, 2);
  assert.match(problems[0], /created_at/);
  assert.match(problems[1], /expires_at/);
});

test('the rendering is stable and inventories CollectionSpecs by owner namespace', () => {
  const dump = {
    collections: [
      collection('a.y', {
        indexes: [{ name: 'by_created', fields: ['created_ms'], unique: true }],
        search: { fields: ['title'] },
      }),
      collection('b.x'),
      collection('a.z', { ttl_field: 'expires_ms' }),
      collection('remix_markers'),
    ],
  };
  const text = render(dump);
  assert.ok(text.indexOf('`a.y`') < text.indexOf('`b.x`'));
  assert.ok(text.indexOf('`a.y`') < text.indexOf('`a.z`'));
  assert.match(text, /## a\n/);
  assert.match(text, /## b\n/);
  assert.match(text, /## legacy \(unqualified names\)\n/);
  assert.doesNotMatch(text, /composed registry/i);
  assert.match(text, /by_created/);
  assert.match(text, /`expires_ms`/);
  assert.match(text, /`title`/);
  assert.equal(render(dump), text);
});

test('the dump example builds with web3 by default and with an override that keeps it', async () => {
  const { dumpFeatures, dumpSchema } = await import('../ci/check-storage-schema.mjs');
  assert.equal(dumpFeatures({}), 'web3');
  assert.equal(dumpFeatures({ STORAGE_SCHEMA_FEATURES: 'web3,flows' }), 'web3,flows');
  assert.throws(() => dumpFeatures({ STORAGE_SCHEMA_FEATURES: 'flows' }), /web3/);
  const product = dumpFeatures({ STORAGE_SCHEMA_FEATURES: 'core-product' });
  assert.ok(product.split(',').includes('web3'));
  let args;
  const dump = dumpSchema(undefined, {
    env: { STORAGE_SCHEMA_FEATURES: 'core-product' },
    run: (_cmd, argv) => {
      args = argv;
      return { status: 0, stdout: '{"collections":[]}', stderr: '' };
    },
  });
  assert.deepEqual(dump, { collections: [] });
  assert.equal(args[args.indexOf('--features') + 1], product);
});
