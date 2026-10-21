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
      collection('flows.runs'),
    ],
  });
  assert.equal(problems.length, 3, problems.join('\n'));
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

test('the rendering is stable and lists the composed collection declarations', () => {
  const dump = {
    collections: [
      collection('b.x'),
      collection('a.y', {
        indexes: [{ name: 'by_created', fields: ['created_ms'], unique: true }],
        search: { fields: ['title'] },
      }),
    ],
  };
  const text = render(dump);
  assert.ok(text.indexOf('`a.y`') < text.indexOf('`b.x`'));
  assert.match(text, /by_created/);
  assert.match(text, /Search fields/);
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
