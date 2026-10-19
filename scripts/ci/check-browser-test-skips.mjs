#!/usr/bin/env node
// Every executable browser-test skip needs its tracking issue on the same line.

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const requireFromApp = createRequire(new URL('../../app/package.json', import.meta.url));
const ts = requireFromApp('typescript');
const issueUrl = /https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/issues\/\d+/;

export function untrackedSkipLines(source) {
  const sourceFile = ts.createSourceFile(
    'browser-spec.tsx',
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX
  );
  const lines = source.split('\n');
  const failures = [];

  function visit(node) {
    if (
      ts.isCallExpression(node) &&
      ts.isPropertyAccessExpression(node.expression) &&
      node.expression.name.text === 'skip'
    ) {
      const line = sourceFile.getLineAndCharacterOfPosition(node.expression.name.getStart(sourceFile))
        .line + 1;
      if (!issueUrl.test(lines[line - 1] ?? '')) {
        failures.push(`${line}: ${lines[line - 1]?.trim() ?? ''}`);
      }
    }
    ts.forEachChild(node, visit);
  }

  visit(sourceFile);
  return failures;
}

if (
  process.argv[1] &&
  import.meta.url === new URL(`file://${process.argv[1]}`).href
) {
  let files;
  try {
    files = execFileSync(
      'git',
      ['ls-files', 'app/test/e2e/specs', 'app/test/playwright/specs'],
      { encoding: 'utf8' }
    )
      .trim()
      .split('\n')
      .filter(Boolean);
  } catch (error) {
    console.error(`Could not list browser specs: ${error.message}`);
    process.exit(2);
  }
  const failures = [];
  for (const file of files) {
    const badLines = untrackedSkipLines(readFileSync(file, 'utf8'));
    failures.push(...badLines.map(line => `${file}:${line}`));
  }
  if (failures.length) {
    console.error(
      'Browser-test skips need a GitHub issue URL on the same line:\n' +
        failures.map(line => `  ${line}`).join('\n')
    );
    process.exit(1);
  }
}
