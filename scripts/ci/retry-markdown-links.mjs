import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

/** Extract URLs only when every reported link failure is an HTTP 503. */
export function serviceUnavailableUrls(report) {
  const failures = [...report.matchAll(/^\*\s+\[([^\]]+)\]\s+<([^>]+)>/gm)];
  if (!failures.length || failures.some(([, status]) => status !== '503')) return null;
  return [...new Set(failures.map(([, , url]) => url))];
}

/** Recheck a failed sweep without accepting unsuccessful HTTP responses. */
export async function retryServiceUnavailable({ initialReport, check, wait = delay }) {
  const urls = serviceUnavailableUrls(initialReport);
  if (!urls) return 1;
  await wait(5000);
  return (await check(urls)).status;
}

async function main(reportPath, binary) {
  const initialReport = readFileSync(reportPath, 'utf8');
  let attempt = 0;
  const status = await retryServiceUnavailable({
    initialReport,
    check(urls) {
      const input = join(dirname(reportPath), `retry-${++attempt}.md`);
      writeFileSync(input, urls.map((url) => `- <${url}>`).join('\n') + '\n');
      // These URLs passed the full sweep's inclusion/exclusion rules already.
      // Keep fragment checking on the retry, and require actual successful HTTP.
      const run = spawnSync(binary, [
        '--no-progress', '--max-concurrency', '4', '--include-fragments',
        '--format', 'markdown', '--output', reportPath, input,
      ], { stdio: 'inherit', timeout: 90000 });
      return { status: run.status ?? 1, report: readFileSync(reportPath, 'utf8') };
    },
  });
  process.exitCode = status;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv[2], process.argv[3]);
}
