'use strict';
// Shared helpers for the Node integration/host-contract suites (TIX-31.3, TIX-31.4).

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const assert = require('node:assert');
const { createHost, run, nodeFs } = require('../../bin/tix.js');

/** Same fixed clock as the Rust `MemHost` default (`TEST_NOW`, 2026-09-14T15:02:11Z). */
const TEST_NOW = 1789398131;

const FIXTURE_DIR = path.join(__dirname, '..', 'fixtures', 'workspace');

/** Ticket ids in `tests/fixtures/workspace`, matching `crates/tix-io/tests/common/mod.rs`. */
const T_BACKLOG = '01K5AQ9Z3R7M8N2P4Q6S8T0V1W';
const T_PROGRESS = '01K5B2C3D4E5F6G7H8J9K0M1N2';
const T_DONE = '01K5C7Q8R9S0T1V2W3X4Y5Z6A7';
const T_NO_CLIENT = '01K5D1E2F3G4H5J6K7M8N9P0Q1';
const T_UNKNOWN_STATUS = '01K5E9F8G7H6J5K4M3N2P1Q0R9';

/** The id `tix new` produces against a fresh fixture copy and the default deterministic host. */
const FIRST_NEW_ID = '01M2G7189R000G40R40M30E209';

/** Converts an OS path to the absolute, `/`-separated form the wasm module sees. */
function toPosix(p) {
  return p.split(path.sep).join('/');
}

/** Creates a fresh empty temp directory (never hard-coded `/tmp`). */
function freshDir() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'tix-test-'));
}

/** Creates a fresh temp directory seeded with a copy of the fixture workspace. */
function freshWorkspace() {
  const dir = freshDir();
  fs.cpSync(FIXTURE_DIR, dir, { recursive: true });
  return dir;
}

/**
 * A deterministic `randomBytes` matching Rust's `MemHost` (`next_byte` starts
 * at 0 and wraps at 256 across successive calls within one host).
 */
function makeRandomBytes(start = 0) {
  let next = start & 0xff;
  return (n) => {
    const out = new Uint8Array(n);
    for (let i = 0; i < n; i++) {
      out[i] = next;
      next = (next + 1) & 0xff;
    }
    return out;
  };
}

/**
 * A scripted `prompt` that answers from a queue and records every call as
 * `{label, kind, options}`, mirroring the Rust `MemHost.prompts` field.
 */
function makePrompt(answers = []) {
  const queue = answers.slice();
  const calls = [];
  const fn = (label, kind, options) => {
    calls.push({ label, kind, options: Array.from(options) });
    return queue.length ? queue.shift() : null;
  };
  fn.calls = calls;
  return fn;
}

/** Captures everything written to a stdout/stderr sink as one string. */
function makeCapture() {
  let buf = '';
  const fn = (t) => {
    buf += t;
  };
  fn.text = () => buf;
  return fn;
}

/**
 * Builds a deterministic host for one invocation: fixed clock, deterministic
 * entropy, scripted prompt, and capturing stdout/stderr. `fsAdapter` defaults
 * to the real `node:fs` adapter.
 */
function testHost({ now = TEST_NOW, answers = [], fsAdapter = nodeFs, randomStart = 0 } = {}) {
  const stdout = makeCapture();
  const stderr = makeCapture();
  const prompt = makePrompt(answers);
  const host = createHost({
    fsAdapter,
    now,
    randomBytes: makeRandomBytes(randomStart),
    prompt,
    stdout,
    stderr,
  });
  return { host, stdout, stderr, prompt };
}

/**
 * Runs `tix <args>` in `cwd` through the real wasm module, returning
 * `{code, stdout, stderr}` plus the transcript in the same format as the
 * Rust `golden()` helper (see `crates/tix-io/tests/common/mod.rs`).
 */
function invoke(cwd, args, opts = {}) {
  const { host, stdout, stderr } = testHost(opts);
  const code = run(args, cwd, host);
  const out = stdout.text();
  const err = stderr.text();
  const transcript = `$ tix ${args.join(' ')}\nexit: ${code}\n--- stdout\n${out}--- stderr\n${err}`;
  return { code, stdout: out, stderr: err, transcript };
}

/** Runs `tix <args>` and returns `{code, stdout, stderr, transcript, promptCalls}`. */
function invokeWithPrompt(cwd, args, opts = {}) {
  const { host, stdout, stderr, prompt } = testHost(opts);
  const code = run(args, cwd, host);
  const out = stdout.text();
  const err = stderr.text();
  const transcript = `$ tix ${args.join(' ')}\nexit: ${code}\n--- stdout\n${out}--- stderr\n${err}`;
  return { code, stdout: out, stderr: err, transcript, promptCalls: prompt.calls };
}

/**
 * Replaces every occurrence of `cwd` (in its posix form) with `<WS>` in
 * `text`, for commands (`init`) that print the absolute workspace root.
 */
function maskWs(text, cwd) {
  return text.split(toPosix(cwd)).join('<WS>');
}

const GOLDEN_DIR = path.join(__dirname, 'golden');

/**
 * Compares `transcript` against `tests/node/golden/<name>.txt`. With
 * `TIX_UPDATE_GOLDEN=1` set, (re)writes the golden instead of asserting.
 */
function assertGolden(name, transcript) {
  const file = path.join(GOLDEN_DIR, `${name}.txt`);
  if (process.env.TIX_UPDATE_GOLDEN) {
    fs.mkdirSync(GOLDEN_DIR, { recursive: true });
    fs.writeFileSync(file, transcript);
    return;
  }
  // Normalize CRLF that a Windows checkout (core.autocrlf) may have
  // introduced into the golden file on disk; the transcript itself is
  // always LF-only since the wasm module writes bare `\n`.
  const expected = fs.readFileSync(file, 'utf8').replace(/\r\n/g, '\n');
  assert.strictEqual(transcript, expected, `golden mismatch: ${name}`);
}

/** Reads a UTF-8 file, normalizing any CRLF a Windows checkout may have introduced. */
function readTextNormalized(file) {
  return fs.readFileSync(file, 'utf8').replace(/\r\n/g, '\n');
}

/** Removes a temp directory tree, tolerating Windows file-lock flakiness. */
function cleanup(dir) {
  try {
    fs.rmSync(dir, { recursive: true, force: true, maxRetries: 3 });
  } catch {
    // best effort
  }
}

module.exports = {
  TEST_NOW,
  FIXTURE_DIR,
  T_BACKLOG,
  T_PROGRESS,
  T_DONE,
  T_NO_CLIENT,
  T_UNKNOWN_STATUS,
  FIRST_NEW_ID,
  toPosix,
  freshDir,
  freshWorkspace,
  makeRandomBytes,
  makePrompt,
  makeCapture,
  testHost,
  invoke,
  invokeWithPrompt,
  maskWs,
  assertGolden,
  readTextNormalized,
  cleanup,
};
