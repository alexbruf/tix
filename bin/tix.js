#!/usr/bin/env node
'use strict';
// Node host for tix (TIX-5). Implements the host imports of TIX-4 and calls run().

const { run } = require('../dist/tix_wasm.js');

process.exitCode = run(process.argv.slice(2), process.cwd());
