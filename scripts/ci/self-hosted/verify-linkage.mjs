#!/usr/bin/env node
// Inspect real lane outputs, rather than assuming a linker flag took effect.
import { execFileSync } from "node:child_process";
import { readdirSync, statSync, mkdirSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { pathToFileURL } from "node:url";

export function classifyElf(sections, comments) {
  return {
    executable: /\s\.interp\s/.test(sections),
    instrumented: /__llvm_prf_(cnts|data)/.test(sections),
    mold: /\bmold\s+\d/.test(comments),
  };
}

export function findProof(root, instrumented) {
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) {
        if (["incremental", "build", ".fingerprint"].includes(entry.name)) continue;
        const result = visit(path);
        if (result) return result;
      } else if (entry.isFile() && (statSync(path).mode & 0o111) && /\/debug\/(deps|examples)\//.test(path)) {
        try {
          const options = { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] };
          const sections = execFileSync("readelf", ["-SW", path], options);
          const comments = execFileSync("readelf", ["--string-dump=.comment", path], options);
          const kind = classifyElf(sections, comments);
          if (!kind.executable || kind.instrumented !== instrumented) continue;
          if (!kind.mold) throw new Error(`ELF lacks mold metadata: ${path}`);
          return { path, ...kind, comments: comments.trim() };
        } catch (error) {
          if (error.message.startsWith("ELF lacks mold")) throw error;
        }
      }
    }
    return null;
  };
  const result = visit(root);
  if (!result) throw new Error(`No ${instrumented ? "instrumented" : "uninstrumented"} lane executable under ${root}`);
  return result;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const output = process.argv[2] ?? "ci-out/linkage.json";
  const proof = [
    findProof("target/ci-private-build", false),
    findProof("target/ci-private-coverage", true),
  ];
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(JSON.stringify(proof, null, 2));
}
