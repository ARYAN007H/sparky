import { execFileSync } from "node:child_process";

const CORE = "crates/openhuman-core/src/";
const BASELINE = "scripts/ci/storage-bypass-baseline.json";
const EXTENSION = "scripts/ci/storage-bypass-extension.json";
const key = x => JSON.stringify([x.rule, x.path, x.text, x.occurrence]);
const original = x => x.path.startsWith(CORE) && ["sqlite-open", "json-write"].includes(x.rule);

/** Exact existing-debt records never authorize another site or later growth. */
export function validateCoverageDebt(findings, baseline, extension, history) {
  if (!Array.isArray(baseline) || !Array.isArray(extension)) return ["Storage debt manifests must be arrays"];
  const errors = [];
  const actual = new Set(findings.map(key));
  const seen = new Set();
  for (const record of baseline) if (!original(record)) errors.push(`Core baseline cannot accept extended coverage: ${record.path}`);
  for (const record of extension) {
    const id = key(record);
    if (seen.has(id)) errors.push(`Duplicate extension record: ${record.path}`);
    seen.add(id);
    if (original(record)) errors.push(`Extension cannot conceal original core debt: ${record.path}`);
    if (!actual.has(id)) errors.push(`Stale extension record: ${record.path}: ${record.text}`);
    if (!["classic", "machine", "scratch", "external-export", "read-only", "unmigrated"].includes(record.classification)
        || typeof record.reason !== "string" || record.reason.trim().length < 20
        || !/^https:\/\/github\.com\/tinyhumansai\/openhuman\/issues\/\d+$/.test(record.issue ?? "")) {
      errors.push(`Incomplete audited extension record: ${record.path}`);
    }
  }
  if (history) {
    const oldCore = new Set(history.baseline.map(key));
    for (const record of baseline) if (!oldCore.has(key(record))) errors.push(`Core baseline growth forbidden: ${record.path}: ${record.text}`);
    const oldExtension = history.extension === null ? null : new Set(history.extension.map(key));
    const oldSites = new Set(history.findings.map(key));
    for (const record of extension) {
      if (oldExtension && !oldExtension.has(key(record))) errors.push(`Extension growth forbidden: ${record.path}: ${record.text}`);
      if (!oldExtension && !oldSites.has(key(record))) errors.push(`Initial extension cannot approve a branch-added writer: ${record.path}: ${record.text}`);
    }
  }
  return errors;
}

/** Re-scan canonical merge-base source, including the recorded curated gitlinks. */
export function historicalFindings(root, scanner) {
  const git = (args, cwd = root) => execFileSync("git", args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  let ref = process.env.GITHUB_BASE_REF ? `origin/${process.env.GITHUB_BASE_REF}` : "upstream/main";
  if (!process.env.GITHUB_BASE_REF) {
    try { git(["rev-parse", "--verify", ref]); } catch { ref = "origin/main"; }
  }
  const base = git(["merge-base", ref, "HEAD"]).trim();
  const files = git(["ls-tree", "-r", "-z", "--name-only", base, "--", "crates"]).split("\0").filter(p => /^crates\/[^/]+\/src\/.*\.rs$/.test(p));
  const findings = [];
  const add = (path, source) => {
    if (!scanner.isTestFile(path)) findings.push(...scanner.withoutAllowed(path, scanner.scan(path, source)));
  };
  for (const file of files) add(file, git(["show", `${base}:${file}`]));
  for (const directory of scanner.CURATED_VENDOR_ROOTS) {
    const parts = directory.split("/");
    const module = parts.slice(0, 2).join("/");
    const inside = parts.slice(2).join("/");
    const entry = git(["ls-tree", base, "--", module]);
    const commit = /^160000 commit ([a-f0-9]{40})\t/.exec(entry)?.[1];
    if (!commit) throw new Error(`Missing canonical gitlink for ${module}`);
    const cwd = `${root}/${module}`;
    const names = git(["ls-tree", "-r", "-z", "--name-only", commit, "--", inside], cwd).split("\0").filter(p => p.endsWith(".rs"));
    for (const file of names) add(`${module}/${file}`, git(["show", `${commit}:${file}`], cwd));
  }
  const json = (path, optional = false) => {
    if (!git(["ls-tree", "--name-only", base, "--", path]).trim()) {
      if (optional) return null;
      throw new Error(`Canonical baseline missing: ${path}`);
    }
    const value = JSON.parse(git(["show", `${base}:${path}`]));
    if (!Array.isArray(value)) throw new Error(`Canonical manifest must be an array: ${path}`);
    return value;
  };
  return { base, findings, baseline: json(BASELINE), extension: json(EXTENSION, true) };
}
