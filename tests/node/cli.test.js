'use strict';
// Spawns `node bin/tix.js` as a real child process against a temp copy of
// the fixture workspace, proving the CLI entry point (not just `run()`)
// works end to end, including the worker-thread prompt (TIX-4, TIX-5).

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { FIXTURE_DIR, T_BACKLOG } = require('./helpers.js');

const BIN = path.join(__dirname, '..', '..', 'bin', 'tix.js');

function freshWorkspaceDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tix-cli-'));
  fs.cpSync(FIXTURE_DIR, dir, { recursive: true });
  return dir;
}

function freshEmptyDir() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'tix-cli-empty-'));
}

function cleanup(dir) {
  try {
    fs.rmSync(dir, { recursive: true, force: true, maxRetries: 3 });
  } catch {
    // best effort
  }
}

function spawnTix(args, opts) {
  return spawnSync(process.execPath, [BIN, ...args], {
    encoding: 'utf8',
    timeout: 15000,
    ...opts,
  });
}

test('TIX-31.3 cli: ls runs as a real child process', () => {
  const dir = freshWorkspaceDir();
  try {
    const r = spawnTix(['ls'], { cwd: dir });
    assert.strictEqual(r.status, 0, r.stderr);
    assert.match(r.stdout, /Nova product comparison article/);
    assert.match(r.stdout, new RegExp(T_BACKLOG.slice(0, 8)));
  } finally {
    cleanup(dir);
  }
});

test('TIX-31.3 cli: board runs as a real child process', () => {
  const dir = freshWorkspaceDir();
  try {
    const r = spawnTix(['board'], { cwd: dir });
    assert.strictEqual(r.status, 0, r.stderr);
    assert.match(r.stdout, /backlog \(2\)/);
    assert.match(r.stdout, /\? \(1\)/);
  } finally {
    cleanup(dir);
  }
});

test('TIX-31.3 cli: path runs as a real child process', () => {
  const dir = freshWorkspaceDir();
  try {
    const r = spawnTix(['path', T_BACKLOG.slice(0, 6)], { cwd: dir });
    assert.strictEqual(r.status, 0, r.stderr);
    assert.strictEqual(r.stdout, `tickets/${T_BACKLOG}\n`);
  } finally {
    cleanup(dir);
  }
});

test('TIX-31.3 cli: no workspace exits 2 with "no tix.yaml found"', () => {
  const dir = freshEmptyDir();
  try {
    const r = spawnTix(['ls'], { cwd: dir });
    assert.strictEqual(r.status, 2);
    assert.strictEqual(r.stderr, 'no tix.yaml found\n');
    assert.strictEqual(r.stdout, '');
  } finally {
    cleanup(dir);
  }
});

test('TIX-31.3 cli: new prompts over the worker-thread readline end to end', () => {
  const dir = freshWorkspaceDir();
  try {
    const r = spawnTix(['new', '--title', 'Piped ticket'], {
      cwd: dir,
      input: 'acme\narticle\n',
    });
    assert.strictEqual(r.status, 0, r.stderr);
    const newId = r.stdout.trim();
    assert.match(newId, /^[0-9A-Z]{26}$/);
    const body = fs.readFileSync(path.join(dir, 'tickets', newId, 'ticket.md'), 'utf8');
    assert.match(body, /title: Piped ticket/);
    assert.match(body, /client: acme/);
    assert.match(body, /type: article/);
    // Prompts are written to stderr, so stdout carries only the new id.
    assert.match(r.stderr, /client.*: $/m);
    assert.match(r.stderr, /type \[article\/landing_page\/linkedin\/other\]/);
  } finally {
    cleanup(dir);
  }
});
