'use strict';
const test = require('node:test');
const assert = require('node:assert');
const { spawnSync } = require('node:child_process');
const path = require('node:path');

test('host loads wasm and exits 0', () => {
  const r = spawnSync(process.execPath, [path.join(__dirname, '../../bin/tix.js')], { encoding: 'utf8' });
  assert.strictEqual(r.status, 0, r.stderr);
});
