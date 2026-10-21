#!/usr/bin/env node

/**
 * Keep module implementation tests in their owning vendor repository. Host
 * tests may still exercise public facades and the module bus contracts.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

// These namespaces exercise implementation behavior rather than the host's
// composition surface. Public module types and *_bus contracts remain allowed.
const DENIED_PATHS = [
  {
    path: 'tinychannels::providers::',
    reason: 'TinyChannels provider behavior belongs in TinyChannels.',
  },
  {
    path: 'tinyinference_local::service::',
    reason: 'TinyInference local service behavior belongs in TinyInference.',
  },
  {
    path: 'tinymemory_tools::lifecycle::turn::',
    reason: 'TinyMemory turn lifecycle behavior belongs in TinyMemory.',
  },
  {
    path: 'tinymemory_tools::lifecycle::window::',
    reason: 'TinyMemory window lifecycle behavior belongs in TinyMemory.',
  },
  {
    path: 'tinymemory_tools::queue::',
    reason: 'TinyMemory queue behavior belongs in TinyMemory.',
  },
  {
    path: 'tinymemory_tools::recall::',
    reason: 'TinyMemory recall behavior belongs in TinyMemory.',
  },
  {
    path: 'tinymemory_tools::backfill::',
    reason: 'TinyMemory backfill behavior belongs in TinyMemory.',
  },
  {
    path: 'tinyflows::gates::',
    reason: 'TinyFlows gate behavior belongs in TinyFlows.',
  },
];

// These adapter tests need a vendor-owned value to prove OpenHuman forwards
// the module contract correctly. Keep exceptions tied to both file and path.
const ALLOWLIST = new Map([
  [
    'crates/openhuman-core/src/agent/tinyagents/host/run_context_tests.rs',
    new Map([
      ['tinymemory_tools::lifecycle::window::', 'Asserts OpenHuman maps memory-window state into the TinyAgents run context.'],
    ]),
  ],
  [
    'crates/openhuman-core/src/agent/tinyagents/memory_summarizer_tests.rs',
    new Map([
      ['tinymemory_tools::lifecycle::window::', 'Exercises OpenHuman’s summarizer adapter with the TinyMemory window type.'],
    ]),
  ],
  [
    'crates/openhuman-core/src/agent/tinyagents/middleware/memory_pack_tests.rs',
    new Map([
      ['tinymemory_tools::lifecycle::turn::', 'Checks the OpenHuman middleware adapter using TinyMemory’s public turn-pack type.'],
      ['tinymemory_tools::lifecycle::window::', 'Checks middleware conversion of TinyMemory window metadata.'],
    ]),
  ],
  [
    'crates/openhuman-core/src/flows/ops_run_status_and_prompt_binding_tests.rs',
    new Map([
      ['tinyflows::gates::', 'Uses gate analysis as a fixture assertion for the host run-status response.'],
    ]),
  ],
  [
    'crates/openhuman-core/src/memory/lifecycle/adapter_tests.rs',
    new Map([
      ['tinymemory_tools::lifecycle::turn::', 'Tests the OpenHuman lifecycle adapter against TinyMemory’s public turn-pack contract.'],
      ['tinymemory_tools::lifecycle::window::', 'Tests the OpenHuman lifecycle adapter against TinyMemory’s public window contract.'],
    ]),
  ],
  [
    'crates/openhuman-core/src/memory/lifecycle/jobs_tests.rs',
    new Map([
      ['tinymemory_tools::queue::', 'Tests OpenHuman’s queue host adapter using TinyMemory’s public queue state types.'],
    ]),
  ],
  [
    'crates/openhuman-core/src/memory/lifecycle/prefetch_tests.rs',
    new Map([
      ['tinymemory_tools::lifecycle::turn::', 'Tests OpenHuman prefetch integration using the TinyMemory turn-pack contract.'],
      ['tinymemory_tools::recall::', 'Uses TinyMemory’s token estimator to verify host prefetch budgeting.'],
    ]),
  ],
]);

function isTestFile(filePath) {
  const normalized = filePath.split(path.sep).join('/');
  return (
    (normalized.startsWith('tests/') && normalized.endsWith('.rs')) ||
    (normalized.startsWith('crates/') && normalized.endsWith('_tests.rs'))
  );
}

/** Return forbidden module paths found in an eligible host test source. */
export function findOwnershipViolations(source, filePath) {
  if (!isTestFile(filePath)) return [];

  source = withoutComments(source);
  const normalizedPath = filePath.split(path.sep).join('/');
  const allowed = ALLOWLIST.get(normalizedPath);
  const violations = [];
  for (const denied of DENIED_PATHS) {
    if (allowed?.has(denied.path)) continue;
    const escapedPath = denied.path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const matcher = new RegExp(`\\b${escapedPath}`, 'g');
    for (const match of source.matchAll(matcher)) {
      const line = source.slice(0, match.index).split('\n').length;
      violations.push({ line, path: denied.path, reason: denied.reason });
    }
  }

  return violations.sort((left, right) => left.line - right.line || left.path.localeCompare(right.path));
}

function listRustFiles(directory, predicate, files = []) {
  if (!fs.existsSync(directory)) return files;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const absolutePath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      listRustFiles(absolutePath, predicate, files);
    } else if (entry.isFile() && entry.name.endsWith('.rs') && predicate(absolutePath)) {
      files.push(absolutePath);
    }
  }
  return files;
}

function withoutComments(source) {
  return source.replace(/\/\*[\s\S]*?\*\/|\/\/[^\n]*/g, (comment) =>
    comment.replace(/[^\n]/g, ' '),
  );
}

function main() {
  const testFiles = [
    ...listRustFiles(path.join(ROOT, 'tests'), (file) => true),
    ...listRustFiles(path.join(ROOT, 'crates'), (file) => file.endsWith('_tests.rs')),
  ].sort();

  const failures = [];
  for (const absolutePath of testFiles) {
    const relativePath = path.relative(ROOT, absolutePath);
    const source = fs.readFileSync(absolutePath, 'utf8');
    for (const violation of findOwnershipViolations(source, relativePath)) {
      failures.push(`${relativePath}:${violation.line}: ${violation.path} ${violation.reason}`);
    }
  }

  if (failures.length > 0) {
    console.error('Host tests must not call module implementation internals:');
    for (const failure of failures) console.error(`  ${failure}`);
    process.exitCode = 1;
    return;
  }

  console.log(`Test ownership check passed (${testFiles.length} Rust test files scanned).`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
