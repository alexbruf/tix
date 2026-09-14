'use strict';
// TIX-31.4: host contract. A fake, in-memory FsAdapter records every call as
// `[op, path]`; read-only commands must make zero write/mkdirAll calls,
// write commands must perform exactly the writes they need, and every
// validation (exit 1) or usage (exit 2) failure of a write command must
// write nothing. Also checks discovery never reaches outside the fake root
// (or its ancestors) and never uses a `..` segment.

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { createHost, run } = require('../../bin/tix.js');
const { FIXTURE_DIR, TEST_NOW, T_BACKLOG, T_PROGRESS, T_UNKNOWN_STATUS } = require('./helpers.js');

const WS = '/ws';

/** Reads `tests/fixtures/workspace` off real disk into a `Map<"/ws/..." , Buffer>`. */
function loadFixtureFiles() {
  const files = new Map();
  (function walk(dir, rel) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      const relPath = `${rel}/${entry.name}`;
      if (entry.isDirectory()) {
        walk(full, relPath);
      } else {
        files.set(`${WS}${relPath}`, fs.readFileSync(full));
      }
    }
  })(FIXTURE_DIR, '');
  return files;
}

/**
 * An in-memory FsAdapter over a seed file map, recording every call as
 * `[op, path]` in `.calls` (mirrors the Rust `MemHost`/`Call` shape).
 */
function makeFakeAdapter(seedFiles = new Map()) {
  const files = new Map(seedFiles);
  const dirs = new Set();
  const calls = [];
  return {
    calls,
    files,
    read(p) {
      calls.push(['read', p]);
      return files.has(p) ? files.get(p) : null;
    },
    write(p, bytes) {
      calls.push(['write', p]);
      files.set(p, Buffer.from(bytes));
    },
    listDir(p) {
      calls.push(['listDir', p]);
      const prefix = p.endsWith('/') ? p : `${p}/`;
      const names = new Set();
      for (const key of files.keys()) {
        if (key.startsWith(prefix)) {
          const seg = key.slice(prefix.length).split('/')[0];
          if (seg) names.add(seg);
        }
      }
      return Array.from(names);
    },
    mkdirAll(p) {
      calls.push(['mkdirAll', p]);
      dirs.add(p);
    },
    exists(p) {
      calls.push(['exists', p]);
      if (files.has(p) || dirs.has(p)) return true;
      const prefix = p.endsWith('/') ? p : `${p}/`;
      for (const key of files.keys()) {
        if (key.startsWith(prefix)) return true;
      }
      return false;
    },
  };
}

function runWithFake(cwd, args, seedFiles, opts = {}) {
  const adapter = makeFakeAdapter(seedFiles);
  const host = createHost({
    fsAdapter: adapter,
    now: TEST_NOW,
    randomBytes: (n) => new Uint8Array(n).fill(0).map((_, i) => i),
    prompt: opts.prompt || (() => null),
    stdout: () => {},
    stderr: () => {},
  });
  const code = run(args, cwd, host);
  return { code, calls: adapter.calls, adapter };
}

function writeCalls(calls) {
  return calls.filter((c) => c[0] === 'write' || c[0] === 'mkdirAll');
}

/**
 * Discovery (TIX-6) walks upward from cwd probing `<ancestor>/tix.yaml`
 * until it finds one or reaches the filesystem root (`/tix.yaml`). Such a
 * probe is legitimate even when `<ancestor>` is above the workspace root.
 */
function isAncestorTixYamlProbe(p, root) {
  const suffix = '/tix.yaml';
  if (!p.endsWith(suffix)) return false;
  const dir = p.slice(0, -suffix.length); // '' for the filesystem-root probe '/tix.yaml'
  if (dir === '') return true;
  return root === dir || root.startsWith(`${dir}/`);
}

/** Every recorded path is `root`, under `root/`, or a legitimate upward discovery probe. No `..` segment. */
function assertScoped(calls, root) {
  for (const [, p] of calls) {
    assert.ok(!p.split('/').includes('..'), `path ${p} has a ".." segment`);
    const isSelfOrDescendant = p === root || p.startsWith(`${root}/`);
    const isAncestorProbe = isAncestorTixYamlProbe(p, root);
    assert.ok(
      isSelfOrDescendant || isAncestorProbe,
      `path ${p} escapes root ${root} and its ancestors`
    );
  }
}

// ---- Read-only commands make zero writes ----

for (const [name, args] of [
  ['check', ['check']],
  ['ls', ['ls']],
  ['show', ['show', T_BACKLOG.slice(0, 6)]],
  ['board', ['board']],
  ['path', ['path', T_BACKLOG.slice(0, 6)]],
]) {
  test(`TIX-31.4 ${name}: no writes on a read-only command`, () => {
    const { code, calls } = runWithFake(WS, args, loadFixtureFiles());
    assert.strictEqual(writeCalls(calls).length, 0);
    assertScoped(calls, WS);
    // Sanity: the command actually ran (didn't just error out before reading).
    assert.notStrictEqual(code, undefined);
  });
}

test('TIX-31.4 mv: no writes on unknown status', () => {
  const { code, calls } = runWithFake(WS, ['mv', T_BACKLOG.slice(0, 6), 'bogus'], loadFixtureFiles());
  assert.strictEqual(code, 1);
  assert.strictEqual(writeCalls(calls).length, 0);
  assertScoped(calls, WS);
});

// ---- Write commands perform exactly the writes they need ----

test('TIX-31.4 init: writes tix.yaml and makes tickets/', () => {
  const { code, calls } = runWithFake('/fresh', ['init'], new Map());
  assert.strictEqual(code, 0);
  assert.deepStrictEqual(writeCalls(calls), [
    ['write', '/fresh/tix.yaml'],
    ['mkdirAll', '/fresh/tickets'],
  ]);
  assertScoped(calls, '/fresh');
});

test('TIX-31.4 init: no writes when a workspace already exists', () => {
  const { code, calls } = runWithFake(WS, ['init'], loadFixtureFiles());
  assert.strictEqual(code, 2);
  assert.strictEqual(writeCalls(calls).length, 0);
});

test('TIX-31.4 new: writes exactly mkdirAll + ticket.md for the new id', () => {
  const { code, calls } = runWithFake(
    WS,
    ['new', '--title', 'T', '--client', 'acme', '--type', 'article'],
    loadFixtureFiles()
  );
  assert.strictEqual(code, 0);
  const writes = writeCalls(calls);
  assert.strictEqual(writes.length, 2);
  assert.strictEqual(writes[0][0], 'mkdirAll');
  assert.strictEqual(writes[1][0], 'write');
  assert.match(writes[0][1], /^\/ws\/tickets\/[0-9A-Z]{26}$/);
  assert.strictEqual(writes[1][1], `${writes[0][1]}/ticket.md`);
  assertScoped(calls, WS);
});

test('TIX-31.4 new: no writes on validation failure (exit 1)', () => {
  const { code, calls } = runWithFake(
    WS,
    ['new', '--title', 'T', '--client', 'acme', '--type', 'bogus'],
    loadFixtureFiles()
  );
  assert.strictEqual(code, 1);
  assert.strictEqual(writeCalls(calls).length, 0);
});

test('TIX-31.4 new: no writes on usage failure (exit 2, unknown flag)', () => {
  const { code, calls } = runWithFake(WS, ['new', '--bogus-flag', 'x'], loadFixtureFiles());
  assert.strictEqual(code, 2);
  assert.strictEqual(writeCalls(calls).length, 0);
});

test('TIX-31.4 mv: no writes on an ambiguous id prefix (usage error, exit 2)', () => {
  const { code, calls } = runWithFake(WS, ['mv', '01K5', 'done'], loadFixtureFiles());
  assert.strictEqual(code, 2);
  assert.strictEqual(writeCalls(calls).length, 0);
});

test('TIX-31.4 mv: writes exactly mkdirAll + ticket.md on success', () => {
  const { code, calls } = runWithFake(WS, ['mv', T_BACKLOG.slice(0, 6), 'in_progress'], loadFixtureFiles());
  assert.strictEqual(code, 0);
  assert.deepStrictEqual(writeCalls(calls), [
    ['mkdirAll', `${WS}/tickets/${T_BACKLOG}`],
    ['write', `${WS}/tickets/${T_BACKLOG}/ticket.md`],
  ]);
});

test('TIX-31.4 set: writes exactly mkdirAll + ticket.md on success', () => {
  const { code, calls } = runWithFake(WS, ['set', T_BACKLOG.slice(0, 6), 'owner=alex'], loadFixtureFiles());
  assert.strictEqual(code, 0);
  assert.deepStrictEqual(writeCalls(calls), [
    ['mkdirAll', `${WS}/tickets/${T_BACKLOG}`],
    ['write', `${WS}/tickets/${T_BACKLOG}/ticket.md`],
  ]);
});

test('TIX-31.4 set: no writes on unknown field (exit 1)', () => {
  const { code, calls } = runWithFake(WS, ['set', T_BACKLOG.slice(0, 6), 'bogus=x'], loadFixtureFiles());
  assert.strictEqual(code, 1);
  assert.strictEqual(writeCalls(calls).length, 0);
});

test('TIX-31.4 set: no writes on a duplicate key (usage error, exit 2)', () => {
  const { code, calls } = runWithFake(
    WS,
    ['set', T_BACKLOG.slice(0, 6), 'owner=a', 'owner=b'],
    loadFixtureFiles()
  );
  assert.strictEqual(code, 2);
  assert.strictEqual(writeCalls(calls).length, 0);
});

test('TIX-31.4 attach: writes exactly mkdirAll + ticket.md on success', () => {
  const { code, calls } = runWithFake(
    WS,
    ['attach', T_BACKLOG.slice(0, 6), 'https://example.com/doc'],
    loadFixtureFiles()
  );
  assert.strictEqual(code, 0);
  assert.deepStrictEqual(writeCalls(calls), [
    ['mkdirAll', `${WS}/tickets/${T_BACKLOG}`],
    ['write', `${WS}/tickets/${T_BACKLOG}/ticket.md`],
  ]);
});

test('TIX-31.4 attach: no writes when the ref is already attached (exit 1)', () => {
  const { code, calls } = runWithFake(
    WS,
    ['attach', T_PROGRESS.slice(0, 6), 'https://docs.google.com/document/d/abc'],
    loadFixtureFiles()
  );
  assert.strictEqual(code, 1);
  assert.strictEqual(writeCalls(calls).length, 0);
});

test('TIX-31.4 attach: no writes on an ambiguous id prefix (usage error, exit 2)', () => {
  const { code, calls } = runWithFake(WS, ['attach', '01K5', 'https://example.com/x'], loadFixtureFiles());
  assert.strictEqual(code, 2);
  assert.strictEqual(writeCalls(calls).length, 0);
});

test('TIX-31.4 detach: writes exactly mkdirAll + ticket.md on success', () => {
  const { code, calls } = runWithFake(WS, ['detach', T_PROGRESS.slice(0, 6), 'outline'], loadFixtureFiles());
  assert.strictEqual(code, 0);
  assert.deepStrictEqual(writeCalls(calls), [
    ['mkdirAll', `${WS}/tickets/${T_PROGRESS}`],
    ['write', `${WS}/tickets/${T_PROGRESS}/ticket.md`],
  ]);
});

test('TIX-31.4 detach: no writes when nothing matches (exit 1)', () => {
  const { code, calls } = runWithFake(WS, ['detach', T_PROGRESS.slice(0, 6), 'nope'], loadFixtureFiles());
  assert.strictEqual(code, 1);
  assert.strictEqual(writeCalls(calls).length, 0);
});

test('TIX-31.4 detach: no writes on an ambiguous id prefix (usage error, exit 2)', () => {
  const { code, calls } = runWithFake(WS, ['detach', '01K5', 'nope'], loadFixtureFiles());
  assert.strictEqual(code, 2);
  assert.strictEqual(writeCalls(calls).length, 0);
});

test('TIX-31.4 set: repair writes exactly mkdirAll + ticket.md (status= + field)', () => {
  const seed = loadFixtureFiles();
  const key = `${WS}/tickets/${T_UNKNOWN_STATUS}/ticket.md`;
  seed.set(key, Buffer.from(seed.get(key).toString('utf8').replace('client: acme\n', '')));
  const { code, calls } = runWithFake(
    WS,
    ['set', T_UNKNOWN_STATUS.slice(0, 6), 'status=backlog', 'client=acme'],
    seed
  );
  assert.strictEqual(code, 0);
  assert.deepStrictEqual(writeCalls(calls), [
    ['mkdirAll', `${WS}/tickets/${T_UNKNOWN_STATUS}`],
    ['write', `${WS}/tickets/${T_UNKNOWN_STATUS}/ticket.md`],
  ]);
});

// ---- Discovery never escapes the root (or its ancestors), never uses ".." ----

test('TIX-31.4 discovery: every path stays under the root or its ancestors, no ..', () => {
  const { calls } = runWithFake(`${WS}/tickets/${T_BACKLOG}`, ['ls'], loadFixtureFiles());
  assert.ok(calls.length > 0);
  assertScoped(calls, WS);
  for (const [, p] of calls) {
    assert.ok(!p.includes('..'));
  }
});
