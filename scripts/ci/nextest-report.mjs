#!/usr/bin/env node
// Consume nextest's JUnit vocabulary; retain counts before checking inventory.
import fs from "node:fs";

const [report, output, expectedFile] = process.argv.slice(2);
const xml = fs.readFileSync(report, "utf8");
const decode = (value) =>
  value
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
const suites = [...xml.matchAll(/<testsuite\s+([^>]+)>/g)].map((match) => {
  const attributes = Object.fromEntries(
    [...match[1].matchAll(/([\w-]+)="([^"]*)"/g)].map((entry) => [
      entry[1],
      decode(entry[2]),
    ]),
  );
  return {
    name: attributes.name,
    tests: Number(attributes.tests ?? 0),
    failures: Number(attributes.failures ?? 0),
    errors: Number(attributes.errors ?? 0),
    skipped: Number(attributes.skipped ?? 0),
  };
});
const counts = {
  elapsedSeconds: Number(process.env.NEXTEST_ELAPSED_SECONDS ?? 0),
  tests: 0,
  failures: 0,
  errors: 0,
  skipped: 0,
  suites,
};
for (const suite of suites)
  for (const key of ["tests", "failures", "errors", "skipped"])
    counts[key] += suite[key];
fs.writeFileSync(output, `${JSON.stringify(counts, null, 2)}\n`);
console.log(
  `[ci][nextest] ${report}: ${counts.tests} tests, ${counts.failures} failures, ${counts.errors} errors, ${counts.skipped} skipped`,
);
const expected = expectedFile
  ? fs.readFileSync(expectedFile, "utf8").split(/\r?\n/).filter(Boolean)
  : [];
const missing = expected.filter(
  (target) =>
    !suites.some(
      (suite) =>
        (suite.name === target || suite.name?.endsWith(`::${target}`)) &&
        suite.tests > 0,
    ),
);
if (counts.tests === 0 || missing.length > 0) {
  console.error(
    `[ci][nextest] expected nonempty suites; missing/empty: ${missing.join(", ") || "entire selection"}`,
  );
  process.exitCode = 1;
}
