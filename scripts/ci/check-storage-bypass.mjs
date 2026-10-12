#!/usr/bin/env node
// Storage-bypass ratchet.
//
// Persistent state goes through the storage port (`crates/openhuman-core/src/
// storage/`, `tinystoragedrivers`), so one backend setting moves all of it:
// SQLite files, a file tree, or a shared MongoDB, with every record under the
// acting agent's scope. Code that opens its own database or writes its own
// file store behind the port makes state a cloud deployment cannot share or
// isolate. Each existing site is baselined exactly; a new one fails CI, and a
// fixed one must be removed from the baseline.
//
// Rules (non-test Rust under crates/openhuman-core/src, outside storage/):
//   sqlite-open  rusqlite `Connection::open(` - use the document port
//   json-write   fs::write / File::create / OpenOptions::new() / NamedTempFile - a raw file
//                store; use the document, stream or secret port
//
// A file may keep a rule when it is the legacy fallback of a store that already
// runs on the port (it serves the host that configured no backend), or when its
// state is per-machine and must not be shared. Both are listed in ALLOW with the
// reason; the reason is the review.
//
//   node scripts/ci/check-storage-bypass.mjs
//   node scripts/ci/check-storage-bypass.mjs --write-baseline
//   node scripts/ci/check-storage-bypass.mjs --root <dir>   (tests)

import { execFileSync } from "node:child_process";
import { stripRustComments } from "./check-crate-chain.mjs";
import { validateCoverageDebt, historicalFindings } from "./storage-bypass-debt.mjs";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const rootFlag = process.argv.indexOf("--root");
const repoRoot =
  rootFlag === -1
    ? resolve(dirname(fileURLToPath(import.meta.url)), "../..")
    : resolve(process.argv[rootFlag + 1]);
const scanRoot = resolve(repoRoot, "crates");
const baselinePath = resolve(
  repoRoot,
  "scripts/ci/storage-bypass-baseline.json",
);
const writeBaseline = process.argv.includes("--write-baseline");

export const CURATED_VENDOR_ROOTS = [
  "vendor/tinyagents/crates/tinyagents-session/src/run_ledger",
  "vendor/tinyagents/crates/tinyagents-session/src/threads/store",
  "vendor/tinyagents/crates/tinyagents-session/src/transcript",
  "vendor/tinyagents/crates/tinyagents-session/src/turn_state/store.rs",
  "vendor/tinymcp/crates/tinymcp/src/registry/store",
  "vendor/tinymcp/crates/tinymcp/src/audit/store",
  "vendor/tinymcp/crates/tinymcp/src/tinybus_module/service.rs",
];

export const RULES = [
  {
    rule: "sqlite-open",
    pattern: /\bConnection::open(?:_with_flags)?\s*\(/,
    hint: "persist through the storage port (crate::storage::documents::Repo)",
  },
  {
    rule: "json-write",
    pattern:
      /\b(?:fs::write|File::create|OpenOptions::new|NamedTempFile::new_in)\s*\(/,
    hint: "persist through the storage port, or allowlist it with a reason in ALLOW",
  },
  {
    rule: "file-mutation",
    pattern: /\b(?:fs::(?:rename|copy|hard_link)|tempfile::Builder::new|serde_json::to_writer(?:_pretty)?)\s*\(/,
    hint: "review the filesystem publication or serializer destination through the owning storage port",
  },
  {
    rule: "writer-helper",
    pattern: /\bfn\s+(?:write_private|create_private|write_atomic)\s*\(/,
    hint: "review this helper definition; private permissions do not supply shared storage",
  },
  {
    rule: "vendored-open",
    pattern: /\b(?:[A-Za-z_][\w]*::)*(?:[A-Za-z_][\w]*Store|[A-Za-z_][\w]*Ledger)::open(?:_file)?\s*\([^;]*"[^"\n]*\.(?:db|jsonl)"/,
    hint: "a persistent path passed to an owning store opener needs captured native routing",
  },
];

const SRC = "crates/openhuman-core/src/";
const FALLBACK =
  "legacy fallback of a store that already runs on the storage port";

/**
 * path -> { rule, site, reason }: the one site (its trimmed source line) a rule
 * does not apply to. Pinning the line means a new write in the same file is
 * still a finding, and removing the allowed line does not free a quota for
 * another.
 */
export const ALLOW = new Map(
  [
    [
      "security/approval/store.rs",
      "sqlite-open",
      "let conn = Connection::open(&db_path).with_context(|| {",
      FALLBACK,
    ],
    [
      "security/devices/store.rs",
      "sqlite-open",
      "let conn = Connection::open(&db_path)",
      FALLBACK,
    ],
    [
      "desktop/notifications/store.rs",
      "sqlite-open",
      "let conn = Connection::open(&db_path).with_context(|| {",
      FALLBACK,
    ],
    [
      "integrations/task_sources/store.rs",
      "sqlite-open",
      "let mut conn = Connection::open(&db_path)",
      FALLBACK,
    ],
    [
      "cron/policy.rs",
      "sqlite-open",
      "let conn = Connection::open(&path)",
      FALLBACK,
    ],
    [
      "config/workspace/state.rs",
      "sqlite-open",
      "let conn = Connection::open(db_path)?;",
      "vault watcher state is absolute local paths and their mtimes: per-machine filesystem state that is meaningless on, and must not be shared through, another host's database",
    ],
    [
      "platform/cost/tracker.rs",
      "json-write",
      "let mut file = OpenOptions::new()",
      FALLBACK,
    ],
    [
      "agent/orchestration/subagent_sessions/store.rs",
      "json-write",
      "fs::write(&tmp_path, raw)",
      FALLBACK,
    ],
    [
      "integrations/composio/file_store.rs",
      "json-write",
      "tokio::fs::write(&tmp, &bytes)",
      FALLBACK,
    ],
    [
      "desktop/control/ops.rs",
      "json-write",
      "let mut temporary = tempfile::NamedTempFile::new_in(parent)",
      "this machine's own consent to drive its desktop: per-machine, and it must fail closed rather than follow a shared backend to another host",
    ],
    [
      "desktop/app_state/ops/state_file.rs",
      "json-write",
      "let mut temp_file = NamedTempFile::new_in(parent)",
      "holds the local encryption key and keyring consent, which are needed before any storage backend is open (bootstrap state is never read from storage)",
    ],
    [
      "web3/wallet/ops/state.rs",
      "json-write",
      "let mut temp_file = NamedTempFile::new_in(parent)",
      "paired with the OS keychain mnemonic (and holds it when there is no keychain): moves with the secrets work, not as a plain document",
    ],
    [
      "inference/tokenjuice/savings.rs",
      "json-write",
      "if let Err(e) = std::fs::write(path, json) {",
      "one process-global savings counter snapshot, cheap to lose; not per-user data a scope would mean anything for",
    ],
    [
      "agent/artifacts/store.rs",
      "json-write",
      "tokio::fs::write(&tmp_path, json).await.map_err(|e| {",
      FALLBACK,
    ],
    [
      "agent/artifacts/store.rs",
      "json-write",
      "tokio::fs::write(&args_path, json)",
      FALLBACK,
    ],
    [
      "agent/artifacts/files.rs",
      "json-write",
      "match tokio::fs::OpenOptions::new()",
      "reserves the artifact's content file in the user-visible files folder; the bytes stay a file on every backend (the port has no blob store), only the records moved",
    ],
  ].reduce((map, [path, rule, site, reason]) => {
    const key = `${SRC}${path}`;
    map.set(key, [...(map.get(key) ?? []), { rule, site, reason }]);
    return map;
  }, new Map()),
);

/** Whether `finding` is the allowlisted site of its file (its first occurrence). */
export function allowed(rel, finding) {
  return (ALLOW.get(rel) ?? []).some(
    (entry) =>
      entry.rule === finding.rule &&
      entry.site === finding.text &&
      finding.occurrence === 1,
  );
}

/** Drops the allowlisted site from `rel`'s findings; anything else stays. */
export function withoutAllowed(rel, found) {
  return found.filter((f) => !allowed(rel, f));
}

export function isTestFile(rel) {
  return (
    rel.startsWith("crates/openhuman-core/src/storage/") ||
    rel.includes("/tests/") ||
    rel.endsWith("_tests.rs") ||
    rel.includes(`${sep}test_support${sep}`) ||
    rel.includes("/test_support/")
  );
}

/** Code part of a line: drops `//` comments outside string literals. */
export function codeOf(line) {
  let quoted = false;
  let escaped = false;
  for (let i = 0; i < line.length; i += 1) {
    const c = line[i];
    if (c === '"' && !escaped) quoted = !quoted;
    if (!quoted && c === "/" && line[i + 1] === "/") return line.slice(0, i);
    escaped = c === "\\" && !escaped;
  }
  return line;
}

/** Findings in one file's source. */
export function scan(rel, source) {
  const found = [];
  const occurrences = new Map();
  const lines = source.split(/\r?\n/);
  const masked = stripRustComments(source).split(/\r?\n/);
  for (const [index, raw] of lines.entries()) {
    const code = codeOf(raw);
    if (!code.trim()) continue;
    // A call may break between the path and its `(`: look one line ahead, and
    // count a match only when it starts on this line.
    const searchable = `${code}\n${codeOf(lines[index + 1] ?? "")}`;
    for (const { rule, pattern } of RULES) {
      const input = ["sqlite-open", "json-write", "vendored-open"].includes(rule) ? searchable : `${masked[index]}\n${masked[index + 1] ?? ""}`;
      const hit = pattern.exec(input);
      if (!hit || hit.index >= code.length) continue;
      if (rule === "vendored-open" && !masked[index].slice(hit.index).match(/\b\w+::/)) continue;
      const text = raw.trim();
      const key = `${rule}\0${text}`;
      const occurrence = (occurrences.get(key) ?? 0) + 1;
      occurrences.set(key, occurrence);
      found.push({ rule, path: rel, line: index + 1, text, occurrence });
    }
  }
  return found;
}

export const originalCore = f => f.path.startsWith(SRC) && ["sqlite-open", "json-write"].includes(f.rule);

export const identity = ({ rule, path, text, occurrence }) =>
  `${rule}\0${path}\0${text}\0${occurrence}`;

/** `added` must fail; `stale` must be removed from the baseline. */
export function compare(findings, baseline) {
  const expected = new Set(baseline.map(identity));
  const actual = new Set(findings.map(identity));
  return {
    added: findings.filter((f) => !expected.has(identity(f))),
    stale: baseline.filter((b) => !actual.has(identity(b))),
  };
}

async function rustFiles(dir) {
  const out = [];
  if (dir.endsWith(".rs")) {
    try { await readFile(dir); return [dir]; } catch { return []; }
  }
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const entry of entries) {
    const path = resolve(dir, entry.name);
    if (entry.isDirectory()) out.push(...(await rustFiles(path)));
    else if (entry.name.endsWith(".rs")) out.push(path);
  }
  return out.sort();
}

export async function collectFindings(root) {
  const findings = [];
  const scanRoot = resolve(root, "crates");
  const roots = (await readdir(scanRoot, { withFileTypes: true })).filter(e => e.isDirectory()).map(e => resolve(scanRoot, e.name, "src"));
  roots.push(...CURATED_VENDOR_ROOTS.map(p => resolve(root, p)));
  for (const path of (await Promise.all(roots.map(rustFiles))).flat()) {
    const rel = relative(root, path).split(sep).join("/");
    if (isTestFile(rel)) continue;
    const found = scan(rel, await readFile(path, "utf8"));
    findings.push(...withoutAllowed(rel, found));
  }

  return findings;
}

async function main() {
  const findings = await collectFindings(repoRoot);
  if (writeBaseline) {
    let previous;
    try { previous = JSON.parse(await readFile(baselinePath, "utf8")); } catch { previous = null; }
    if (previous && compare(findings.filter(f => originalCore(f)), previous).added.length) {
      console.error("Refusing baseline growth: fix new writers instead.");
      return 1;
    }
    await mkdir(dirname(baselinePath), { recursive: true });
    await writeFile(baselinePath, `${JSON.stringify(findings.filter(f => originalCore(f)), null, 2)}\n`);
    console.log(
      `Wrote ${findings.length} baselined storage-bypass sites to ${relative(repoRoot, baselinePath)}.`,
    );
    return 0;
  }

  let baseline;
  try {
    baseline = JSON.parse(await readFile(baselinePath, "utf8"));
  } catch (error) {
    console.error(
      `Unable to read ${relative(repoRoot, baselinePath)}: ${error.message}`,
    );
    return 1;
  }

  let extension = [];
  try { extension = JSON.parse(await readFile(resolve(repoRoot, "scripts/ci/storage-bypass-extension.json"), "utf8")); }
  catch (error) { if (error.code !== "ENOENT") throw error; }
  let debtErrors = validateCoverageDebt(findings, baseline, extension);
  // Fixture roots without Git history exercise source scanning; CI always checks
  // the real checkout against its canonical merge base and retained records.
  if (rootFlag === -1) {
    const history = historicalFindings(repoRoot, { scan, withoutAllowed, isTestFile, CURATED_VENDOR_ROOTS });
    debtErrors = validateCoverageDebt(findings, baseline, extension, history);
  }
  if (debtErrors.length) {
    console.error(debtErrors.join("\n"));
    return 1;
  }
  const { added, stale } = compare(findings, [...baseline, ...extension]);
  if (added.length === 0 && stale.length === 0) {
    console.log(
      `Storage-bypass ratchet holds (${findings.length} baselined sites; it only goes down).`,
    );
    return 0;
  }
  if (added.length) {
    console.error(`New storage-bypass sites (${added.length}):`);
    for (const f of added) {
      const hint = RULES.find((r) => r.rule === f.rule)?.hint ?? "";
      console.error(
        `  ${f.rule}: ${f.path}:${f.line}: ${f.text}\n    -> ${hint}`,
      );
    }
  }
  if (stale.length) {
    console.error(
      `\nFixed sites still in the baseline (${stale.length}); remove them:`,
    );
    for (const f of stale)
      console.error(`  ${f.rule}: ${f.path}: ${f.text} [${f.occurrence}]`);
    console.error(
      "\nTighten the baseline with --write-baseline once nothing new was added.",
    );
  }
  return 1;
}

if (fileURLToPath(import.meta.url) === resolve(process.argv[1] ?? "")) {
  process.exit(await main());
}
