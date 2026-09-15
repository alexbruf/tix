'use strict';
// `tix mcp` through bin/tix.js and the real wasm module, over real stdio.
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawn, spawnSync } = require('node:child_process');

const BIN = path.join(__dirname, '../../bin/tix.js');

function session(workspace) {
  const child = spawn(process.execPath, [BIN, 'mcp', '--workspace', workspace], { stdio: ['pipe', 'pipe', 'inherit'] });
  let buf = '';
  let nextId = 0;
  const waiting = new Map();
  child.stdout.on('data', (d) => {
    buf += d;
    let i;
    while ((i = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, i);
      buf = buf.slice(i + 1);
      if (!line.trim()) continue;
      const msg = JSON.parse(line);
      const resolve = waiting.get(msg.id);
      if (resolve) {
        waiting.delete(msg.id);
        resolve(msg);
      }
    }
  });
  const send = (msg) => child.stdin.write(`${JSON.stringify(msg)}\n`);
  const request = (method, params) =>
    new Promise((resolve) => {
      const id = ++nextId;
      waiting.set(id, resolve);
      send({ jsonrpc: '2.0', id, method, params });
    });
  const call = async (name, args) => {
    const r = await request('tools/call', { name, arguments: args });
    return { isError: r.result.isError, text: r.result.content.map((c) => c.text).join('') };
  };
  return { child, send, request, call, close: () => child.stdin.end() };
}

test('mcp --help prints usage and exits 0', () => {
  const r = spawnSync(process.execPath, [BIN, 'mcp', '--help'], { encoding: 'utf8' });
  assert.strictEqual(r.status, 0, r.stderr);
  assert.match(r.stdout, /Usage: tix mcp/);
});

test('mcp over npm host: initialize, tools/list, drive a board', async () => {
  const ws = fs.mkdtempSync(path.join(os.tmpdir(), 'tix-mcp-node-'));
  const s = session(ws);
  try {
    const init = await s.request('initialize', { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'test', version: '1' } });
    assert.strictEqual(init.result.serverInfo.name, 'tix');
    assert.strictEqual(init.result.protocolVersion, '2025-06-18');
    s.send({ jsonrpc: '2.0', method: 'notifications/initialized' });

    const tools = await s.request('tools/list', {});
    assert.strictEqual(tools.result.tools.length, 12);

    let r = await s.call('tix_ls', {});
    assert.ok(r.isError && r.text.includes('no tix.yaml found'), r.text);

    assert.ok(!(await s.call('tix_init', {})).isError);
    r = await s.call('tix_new', { title: 'Q4 article', fields: { client: 'acme', type: 'article' } });
    assert.ok(!r.isError, r.text);
    const id = JSON.parse(r.text).id;

    r = await s.call('tix_new', { title: 'missing client', fields: { type: 'article' } });
    assert.ok(r.isError && r.text.startsWith('exit 1') && r.text.includes('rule 3'), r.text);

    r = await s.call('tix_set', { id, values: { status: 'in_progress', owner: 'sam' } });
    assert.ok(!r.isError, r.text);
    assert.strictEqual(JSON.parse(r.text).status, 'in_progress');

    assert.ok(fs.readFileSync(path.join(ws, 'tickets', id, 'ticket.md'), 'utf8').includes('owner: sam'));
    r = await s.call('tix_check', {});
    assert.deepStrictEqual(JSON.parse(r.text), { ok: true, problems: [] });
  } finally {
    s.close();
    fs.rmSync(ws, { recursive: true, force: true });
  }
});
