#!/usr/bin/env node
// Every executable browser-test skip needs its tracking issue on the same line.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const issueUrl = /https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/issues\/\d+/;

function tokenize(source) {
  const tokens = [];
  let index = 0;
  let line = 1;
  const advance = (end) => {
    while (index < end) if (source[index++] === "\n") line++;
  };
  const scan = (templateExpression = false) => {
    let braces = 0;
    while (index < source.length) {
    const char = source[index];
    if (templateExpression && char === "}") {
      if (braces === 0) {
        index++;
        return;
      }
      braces--;
    }
    if (templateExpression && char === "{") braces++;
    if (/\s/.test(char)) {
      advance(index + 1);
      continue;
    }
    if (char === "/" && source[index + 1] === "/") {
      const end = source.indexOf("\n", index);
      advance(end < 0 ? source.length : end);
      continue;
    }
    if (char === "/" && source[index + 1] === "*") {
      const close = source.indexOf("*/", index + 2);
      advance(close < 0 ? source.length : close + 2);
      continue;
    }
    if (char === "'") {
      const quote = char;
      const start = index;
      const tokenLine = line;
      index++;
      while (index < source.length) {
        if (source[index] === "\\") {
          advance(Math.min(index + 2, source.length));
          continue;
        }
        if (source[index] === quote) {
          index++;
          break;
        }
        advance(index + 1);
      }
      tokens.push({ value: "<string>", start, end: index, line: tokenLine });
      continue;
    }
    if (char === '"') {
      const start = index;
      const tokenLine = line;
      index++;
      while (index < source.length) {
        if (source[index] === "\\") {
          advance(Math.min(index + 2, source.length));
          continue;
        }
        if (source[index++] === '"') break;
        if (source[index - 1] === "\n") line++;
      }
      tokens.push({ value: "<string>", start, end: index, line: tokenLine });
      continue;
    }
    if (char === "`") {
      const start = index;
      const tokenLine = line;
      index++;
      while (index < source.length) {
        if (source[index] === "\\") {
          advance(Math.min(index + 2, source.length));
          continue;
        }
        if (source[index] === "`") {
          index++;
          break;
        }
        if (source[index] === "$" && source[index + 1] === "{") {
          advance(index + 2);
          scan(true);
          continue;
        }
        advance(index + 1);
      }
      tokens.push({ value: "<template>", start, end: index, line: tokenLine });
      continue;
    }
    const start = index;
    const tokenLine = line;
    if (/[A-Za-z_$]/.test(char)) {
      index++;
      while (index < source.length && /[\w$]/.test(source[index])) index++;
    } else {
      index++;
    }
    tokens.push({
      value: source.slice(start, index),
      start,
      end: index,
      line: tokenLine,
    });
    }
  };
  scan();
  return tokens;
}

export function untrackedSkipLines(source) {
  const lines = source.split("\n");
  const tokens = tokenize(source);
  const failures = [];
  for (let index = 0; index + 2 < tokens.length; index++) {
    if (
      tokens[index].value !== "." ||
      tokens[index + 1].value !== "skip" ||
      tokens[index + 2].value !== "("
    )
      continue;
    const line = tokens[index + 1].line;
    if (!issueUrl.test(lines[line - 1] ?? ""))
      failures.push(`${line}: ${lines[line - 1]?.trim() ?? ""}`);
  }
  return failures;
}

if (
  process.argv[1] &&
  import.meta.url === new URL(`file://${process.argv[1]}`).href
) {
  let files;
  try {
    files = execFileSync(
      "git",
      ["ls-files", "app/test/e2e/specs", "app/test/playwright/specs"],
      { encoding: "utf8" },
    )
      .trim()
      .split("\n")
      .filter(Boolean);
  } catch (error) {
    console.error(`Could not list browser specs: ${error.message}`);
    process.exit(2);
  }
  const failures = [];
  for (const file of files) {
    const badLines = untrackedSkipLines(readFileSync(file, "utf8"));
    failures.push(...badLines.map((line) => `${file}:${line}`));
  }
  if (failures.length) {
    console.error(
      "Browser-test skips need a GitHub issue URL on the same line:\n" +
        failures.map((line) => `  ${line}`).join("\n"),
    );
    process.exit(1);
  }
}
