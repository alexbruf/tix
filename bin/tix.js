#!/usr/bin/env node
'use strict';
// Node host for tix (TIX-5). Provides the ten host imports of TIX-4 on
// `globalThis.tix_host` and calls the wasm `run(argv, cwd)`.

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { Worker } = require('node:worker_threads');

/** Paths from wasm are absolute and `/`-separated (Windows: `C:/...`). */
function toPosix(p) {
  return p.split(path.sep).join('/');
}

/** FsAdapter over node:fs. Other adapters (OPFS, S3) implement the same five methods. */
const nodeFs = {
  read(p) {
    try {
      return fs.readFileSync(p);
    } catch (e) {
      if (e.code === 'ENOENT') return null;
      throw String(e.message);
    }
  },
  write(p, bytes) {
    try {
      fs.writeFileSync(p, bytes);
    } catch (e) {
      throw String(e.message);
    }
  },
  listDir(p) {
    try {
      return fs.readdirSync(p);
    } catch (e) {
      throw String(e.message);
    }
  },
  mkdirAll(p) {
    try {
      fs.mkdirSync(p, { recursive: true });
    } catch (e) {
      throw String(e.message);
    }
  },
  exists(p) {
    return fs.existsSync(p);
  },
};

// node:readline is asynchronous, but the wasm `prompt` import must return a
// value. One worker owns a readline over fd 0 for the whole run; this thread
// prints the question with a synchronous write (worker stderr would queue behind
// the blocked main thread), signals the worker, and blocks on Atomics.wait.
// state[0]: 0 idle, 1 question posted, 2 answer ready, 3 end of input.
const PROMPT_WORKER = `
const { workerData } = require('node:worker_threads');
const fs = require('node:fs');
const readline = require('node:readline');
const state = new Int32Array(workerData, 0, 2);
const bytes = new Uint8Array(workerData, 8);
const lines = [];
let ended = false;
let wake = null;
const input = fs.createReadStream('', { fd: 0, autoClose: false });
const rl = readline.createInterface({ input, terminal: false });
rl.on('line', (l) => { lines.push(l); if (wake) wake(); });
rl.on('close', () => { ended = true; if (wake) wake(); });
// A pending Atomics.waitAsync does not keep a worker alive once stdin closes;
// this timer does (the worker is unref'd, so it never blocks process exit).
setInterval(() => {}, 1 << 30);
(async () => {
  for (;;) {
    while (Atomics.load(state, 0) !== 1) await Atomics.waitAsync(state, 0, Atomics.load(state, 0)).value;
    while (!lines.length && !ended) await new Promise((r) => (wake = r));
    wake = null;
    if (lines.length) {
      const enc = Buffer.from(lines.shift(), 'utf8').subarray(0, bytes.length);
      bytes.set(enc);
      Atomics.store(state, 1, enc.length);
      Atomics.store(state, 0, 2);
    } else {
      Atomics.store(state, 0, 3);
    }
    Atomics.notify(state, 0);
  }
})();
`;

let promptShared = null;

function promptSync(label, kind, options) {
  if (!promptShared) {
    promptShared = new SharedArrayBuffer(8 + 65536);
    new Worker(PROMPT_WORKER, { eval: true, workerData: promptShared }).unref();
  }
  const state = new Int32Array(promptShared, 0, 2);
  const bytes = new Uint8Array(promptShared, 8);
  const hint = options.length ? ` [${options.join('/')}]` : kind === 'date' ? ' (YYYY-MM-DD)' : kind === 'list' ? ' (comma-separated)' : '';
  fs.writeSync(2, `${label}${hint}: `);
  Atomics.store(state, 0, 1);
  Atomics.notify(state, 0);
  while (Atomics.load(state, 0) === 1) Atomics.wait(state, 0, 1);
  const done = Atomics.load(state, 0);
  Atomics.store(state, 0, 0);
  if (done === 3) return null;
  return Buffer.from(bytes.subarray(0, Atomics.load(state, 1))).toString('utf8').trim();
}

/**
 * Builds the `tix_host` import object. Tests pass a fake `fsAdapter` and
 * capture functions; the CLI uses the defaults.
 */
function createHost(opts = {}) {
  const a = opts.fsAdapter || nodeFs;
  return {
    fs_read: (p) => a.read(p),
    fs_write: (p, bytes) => a.write(p, bytes),
    fs_list_dir: (p) => a.listDir(p),
    fs_mkdir_all: (p) => a.mkdirAll(p),
    fs_exists: (p) => a.exists(p),
    prompt: opts.prompt || promptSync,
    stdout: opts.stdout || ((t) => process.stdout.write(t)),
    stderr: opts.stderr || ((t) => process.stderr.write(t)),
    now_unix: () => BigInt(opts.now !== undefined ? opts.now : Math.floor(Date.now() / 1000)),
    random_bytes: opts.randomBytes || ((n) => new Uint8Array(crypto.randomBytes(n))),
  };
}

/** Runs one invocation against `host` and returns the exit code. */
function run(argv, cwd, host = createHost()) {
  const wasm = require('../dist/tix_wasm.js');
  globalThis.tix_host = host;
  try {
    return wasm.run(argv, toPosix(cwd));
  } finally {
    delete globalThis.tix_host;
  }
}

/** `tix mcp`: serves the shared MCP server over stdio, one JSON message per line. */
function serveMcp(args, cwd, host = createHost()) {
  const wasm = require('../dist/tix_wasm.js');
  const [kind, value] = wasm.mcp_args(args, toPosix(cwd));
  if (kind === 'help') return void process.stdout.write(value);
  if (kind === 'error') return void (process.stderr.write(`${value}\n`), (process.exitCode = 2));
  globalThis.tix_host = host;
  const rl = require('node:readline').createInterface({ input: process.stdin, terminal: false });
  rl.on('line', (line) => {
    if (!line.trim()) return;
    const reply = wasm.mcp_handle(line, value);
    if (reply != null) process.stdout.write(`${reply}\n`);
  });
}

module.exports = { createHost, nodeFs, promptSync, run, serveMcp };

if (require.main === module) {
  if (Number(process.versions.node.split('.')[0]) < 18) {
    process.stderr.write(`tix needs Node.js 18 or newer (found ${process.version}).\n` +
      'Upgrade Node, or install the native binary: https://github.com/alexbruf/tix#install\n');
    process.exit(2);
  }
  const argv = process.argv.slice(2);
  if (argv[0] === 'mcp') serveMcp(argv.slice(1), process.cwd());
  else process.exitCode = run(argv, process.cwd());
}
