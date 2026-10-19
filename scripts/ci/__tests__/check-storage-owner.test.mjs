/**
 * Fixture-based self-tests for check-storage-owner.mjs
 *
 * Each fixture is a minimal `cargo metadata --all-features` JSON document.
 * Run with: node --test scripts/ci/__tests__/check-storage-owner.test.mjs
 *
 * Fixtures:
 *   - transitive-engine.mjson  : non-allowlisted crate transitively reaches rusqlite via tsd-sqlite → FAIL
 *   - tsd-tag-mismatch.mjson   : non-allowlisted crate declares tsd tag "99.0.0" vs vendored "1.0.0" → FAIL
 *   - allowlisted-root.mjson   : allowlisted crate reaches rusqlite → PASS
 *   - naming-violation.mjson   : crate named "foo-sqlite" → FAIL
 *   - vendored-ok.mjson        : all tsd crates are vendored paths → PASS
 *   - aws-smithy.mjson         : non-allowlisted crate reaches aws-sdk-s3 → FAIL
 */

import { checkMetadata } from '../check-storage-owner.mjs';

const ROOT = '/fake/repo';
const TSD_MANIFEST = `${ROOT}/vendor/tinystoragedrivers/crates/tinystoragedrivers-sqlite/Cargo.toml`;

// ---------------------------------------------------------------------------
// Helper builders
// ---------------------------------------------------------------------------

const pkg = (name, version, manifest, source = null, deps = []) => ({
  id: source ? `${name} ${version} ${source}` : `${name} ${version}`,
  name,
  version,
  manifest_path: manifest,
  source,
  dependencies: deps,
});

const node = (name, version, source, deps) => ({
  id: source ? `${name} ${version} ${source}` : `${name} ${version}`,
  deps: deps.map(([dep_name, dep_ver, dep_source]) => ({
    name: dep_name,
    pkg: dep_source ? `${dep_name} ${dep_ver} ${dep_source}` : `${dep_name} ${dep_ver}`,
    dep_kinds: [{ kind: null }],
  })),
});

const REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index';
const tsd_git = (tag) => `git+https://github.com/tinyhumansai/tinystoragedrivers?tag=${tag}#`;

// ---------------------------------------------------------------------------
// Fixture: transitive engine through tsd crate → must FAIL
// ---------------------------------------------------------------------------

const transitive_engine_fixture = {
  packages: [
    pkg('tinystoragedrivers-sqlite', '1.0.0', TSD_MANIFEST, null, [
      { name: 'rusqlite', pkg: `rusqlite 0.32.1 ${REGISTRY}` },
    ]),
    pkg('rusqlite', '0.32.1', `${ROOT}/.cargo/registry/src/foo/rusqlite-0.32.1/Cargo.toml`, REGISTRY),
    // Non-allowlisted crate depends on tsd-sqlite
    pkg('tinyagents-session', '0.1.0', `${ROOT}/crates/tinyagents-session/Cargo.toml`, null, [
      { name: 'tinystoragedrivers-sqlite', pkg: 'tinystoragedrivers-sqlite 1.0.0' },
    ]),
  ],
  resolve: {
    nodes: [
      node('tinyagents-session', '0.1.0', null, [['tinystoragedrivers-sqlite', '1.0.0', null]]),
      node('tinystoragedrivers-sqlite', '1.0.0', null, [['rusqlite', '0.32.1', REGISTRY]]),
      node('rusqlite', '0.32.1', REGISTRY, []),
    ],
  },
};

// ---------------------------------------------------------------------------
// Fixture: tsd tag mismatch → must FAIL
// ---------------------------------------------------------------------------

const tsd_tag_mismatch_fixture = {
  packages: [
    pkg('tinystoragedrivers-core', '1.0.0', `${ROOT}/vendor/tinystoragedrivers/crates/tinystoragedrivers-core/Cargo.toml`),
    pkg('some-odd-crate', '0.1.0', `${ROOT}/crates/some-odd-crate/Cargo.toml`, null, [
      { name: 'tinystoragedrivers-core', pkg: `tinystoragedrivers-core 1.0.0 ${tsd_git('99.0.0')}` },
    ]),
  ],
  resolve: {
    nodes: [
      node('some-odd-crate', '0.1.0', null, [['tinystoragedrivers-core', '1.0.0', tsd_git('99.0.0')]]),
      node('tinystoragedrivers-core', '1.0.0', null, []),
    ],
  },
};

// ---------------------------------------------------------------------------
// Fixture: allowlisted root reaches engine → must PASS
// ---------------------------------------------------------------------------

const allowlisted_root_fixture = {
  packages: [
    pkg('openhuman', '0.1.0', `${ROOT}/crates/openhuman/Cargo.toml`, null, [
      { name: 'rusqlite', pkg: `rusqlite 0.32.1 ${REGISTRY}` },
    ]),
    pkg('rusqlite', '0.32.1', '/fake/.cargo/registry/src/foo/rusqlite-0.32.1/Cargo.toml', REGISTRY),
  ],
  resolve: {
    nodes: [
      node('openhuman', '0.1.0', null, [['rusqlite', '0.32.1', REGISTRY]]),
      node('rusqlite', '0.32.1', REGISTRY, []),
    ],
  },
};

// ---------------------------------------------------------------------------
// Fixture: crate naming violation → must FAIL
// ---------------------------------------------------------------------------

const naming_violation_fixture = {
  packages: [
    pkg('my-sqlite-wrapper', '0.1.0', `${ROOT}/crates/my-sqlite-wrapper/Cargo.toml`),
  ],
  resolve: {
    nodes: [
      node('my-sqlite-wrapper', '0.1.0', null, []),
    ],
  },
};

// ---------------------------------------------------------------------------
// Fixture: all tsd crates vendored → must PASS
// ---------------------------------------------------------------------------

const vendored_ok_fixture = {
  packages: [
    pkg('tinystoragedrivers-core', '1.0.0', `${ROOT}/vendor/tinystoragedrivers/crates/tinystoragedrivers-core/Cargo.toml`),
    pkg('tinystoragedrivers-sqlite', '1.0.0', TSD_MANIFEST),
    pkg('openhuman-embed', '0.1.0', `${ROOT}/crates/openhuman-embed/Cargo.toml`, null, [
      { name: 'tinystoragedrivers-core', pkg: 'tinystoragedrivers-core 1.0.0' },
    ]),
  ],
  resolve: {
    nodes: [
      node('tinystoragedrivers-core', '1.0.0', null, []),
      node('tinystoragedrivers-sqlite', '1.0.0', null, []),
      node('openhuman-embed', '0.1.0', null, [['tinystoragedrivers-core', '1.0.0', null]]),
    ],
  },
};

// ---------------------------------------------------------------------------
// Fixture: aws-sdk-s3 reached directly → must FAIL
// ---------------------------------------------------------------------------

const aws_s3_fixture = {
  packages: [
    pkg('openhuman-rpc', '0.1.0', `${ROOT}/crates/openhuman-rpc/Cargo.toml`, null, [
      { name: 'aws-sdk-s3', pkg: `aws-sdk-s3 1.0.0 ${REGISTRY}` },
    ]),
    pkg('aws-sdk-s3', '1.0.0', '/fake/.cargo/registry/src/aws/aws-sdk-s3-1.0.0/Cargo.toml', REGISTRY),
  ],
  resolve: {
    nodes: [
      node('openhuman-rpc', '0.1.0', null, [['aws-sdk-s3', '1.0.0', REGISTRY]]),
      node('aws-sdk-s3', '1.0.0', REGISTRY, []),
    ],
  },
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

const { test, describe } = await import('node:test');

describe('transitive engine through tsd crate → must FAIL', () => {
  test('produces at least one engine-reach violation', () => {
    const violations = checkMetadata(transitive_engine_fixture, ROOT);
    const engineViolations = violations.filter(v => v.kind === 'engine-reach');
    if (engineViolations.length === 0) {
      throw new Error(`Expected engine-reach violation for tinyagents-session->rusqlite, got: ${JSON.stringify(violations)}`);
    }
  });
});

describe('tsd tag mismatch → must FAIL', () => {
  test('produces a tsd-tag-mismatch violation', () => {
    const violations = checkMetadata(tsd_tag_mismatch_fixture, ROOT);
    const mismatch = violations.filter(v => v.kind === 'tsd-tag-mismatch');
    if (mismatch.length === 0) {
      throw new Error(`Expected tsd-tag-mismatch violation, got: ${JSON.stringify(violations)}`);
    }
  });
});

describe('allowlisted root reaches engine → must PASS', () => {
  test('produces no violations', () => {
    const violations = checkMetadata(allowlisted_root_fixture, ROOT);
    if (violations.length > 0) {
      throw new Error(`Expected no violations for allowlisted root, got: ${JSON.stringify(violations)}`);
    }
  });
});

describe('crate naming violation → must FAIL', () => {
  test('produces a crate-name violation', () => {
    const violations = checkMetadata(naming_violation_fixture, ROOT);
    const naming = violations.filter(v => v.kind === 'crate-name');
    if (naming.length === 0) {
      throw new Error(`Expected crate-name violation, got: ${JSON.stringify(violations)}`);
    }
  });
});

describe('all tsd crates vendored → must PASS', () => {
  test('produces no violations', () => {
    const violations = checkMetadata(vendored_ok_fixture, ROOT);
    if (violations.length > 0) {
      throw new Error(`Expected no violations for vendored-clean tree, got: ${JSON.stringify(violations)}`);
    }
  });
});

describe('aws-sdk-s3 reached directly → must FAIL', () => {
  test('produces an engine-reach violation for aws-sdk-s3', () => {
    const violations = checkMetadata(aws_s3_fixture, ROOT);
    const engineViolations = violations.filter(v => v.kind === 'engine-reach' && v.engine === 'aws-sdk-s3');
    if (engineViolations.length === 0) {
      throw new Error(`Expected engine-reach violation for aws-sdk-s3, got: ${JSON.stringify(violations)}`);
    }
  });
});
