'use strict';
// TIX-29: the published npm package is pure wasm, has no native code, no
// install-time scripts, and the wasm module imports exactly the ten TIX-4
// host functions.

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const REPO_ROOT = path.join(__dirname, '..', '..');

/** The ten host imports TIX-4 requires, exactly. */
const HOST_FUNCTIONS = [
  'fs_read',
  'fs_write',
  'fs_list_dir',
  'fs_mkdir_all',
  'fs_exists',
  'prompt',
  'stdout',
  'stderr',
  'now_unix',
  'random_bytes',
].sort();

const NATIVE_EXTENSIONS = ['.node', '.exe', '.dll', '.so', '.dylib'];

function packDryRun() {
  const r = spawnSync('npm', ['pack', '--dry-run', '--json'], {
    cwd: REPO_ROOT,
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  assert.strictEqual(r.status, 0, r.stderr);
  const [entry] = JSON.parse(r.stdout);
  return entry;
}

test('TIX-29 package: tarball contains exactly the required files', () => {
  const entry = packDryRun();
  const paths = entry.files.map((f) => f.path).sort();

  for (const required of ['bin/tix.js', 'dist/tix_wasm.js', 'dist/tix_wasm_bg.wasm', 'package.json']) {
    assert.ok(paths.includes(required), `tarball is missing ${required}`);
  }

  const distFiles = paths.filter((p) => p.startsWith('dist/'));
  assert.deepStrictEqual(
    distFiles.sort(),
    ['dist/tix_wasm.js', 'dist/tix_wasm_bg.wasm'],
    'no other dist/ files should ship'
  );

  for (const p of paths) {
    assert.ok(
      !NATIVE_EXTENSIONS.some((ext) => p.endsWith(ext)),
      `tarball contains native code: ${p}`
    );
  }
});

test('TIX-29 package: no install-time scripts', () => {
  const pkg = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'package.json'), 'utf8'));
  const scripts = pkg.scripts || {};
  for (const hook of ['preinstall', 'install', 'postinstall']) {
    assert.ok(!(hook in scripts), `package.json must not define a "${hook}" script`);
  }
});

test('TIX-29 package: engines.node requires >=18', () => {
  const pkg = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'package.json'), 'utf8'));
  assert.strictEqual(pkg.engines && pkg.engines.node, '>=18');
});

test('TIX-29 package: the wasm module imports exactly the ten TIX-4 host functions', () => {
  const bytes = fs.readFileSync(path.join(REPO_ROOT, 'dist', 'tix_wasm_bg.wasm'));
  const mod = new WebAssembly.Module(bytes);
  const imports = WebAssembly.Module.imports(mod);

  const hostImportNames = [];
  for (const imp of imports) {
    if (imp.name.includes('wbindgen')) continue; // wasm-bindgen's internal shims (PLAN.md)
    const m = /^__wbg_(.+)_[0-9a-f]{16}$/.exec(imp.name);
    assert.ok(m, `unexpected non-host, non-wbindgen import: ${imp.module}.${imp.name}`);
    hostImportNames.push(m[1]);
  }

  assert.deepStrictEqual(hostImportNames.sort(), HOST_FUNCTIONS);
});

test('TIX-29 package: dist/tix_wasm.js references exactly the same ten tix_host.* functions', () => {
  const src = fs.readFileSync(path.join(REPO_ROOT, 'dist', 'tix_wasm.js'), 'utf8');
  const names = new Set();
  for (const m of src.matchAll(/tix_host\.([a-zA-Z0-9_]+)/g)) {
    names.add(m[1]);
  }
  assert.deepStrictEqual(Array.from(names).sort(), HOST_FUNCTIONS);
});
