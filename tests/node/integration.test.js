'use strict';
// TIX-31.3: Node integration suite. Each command in TIX-14..TIX-23 gets at
// least one success and one failure golden case, run through the real wasm
// module and `bin/tix.js` against a real on-disk fixture copy.

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const {
  freshDir,
  freshWorkspace,
  invoke,
  invokeWithPrompt,
  maskWs,
  assertGolden,
  readTextNormalized,
  cleanup,
  T_BACKLOG,
  T_PROGRESS,
  T_NO_CLIENT,
  T_UNKNOWN_STATUS,
  FIRST_NEW_ID,
} = require('./helpers.js');

function withFixture(fn) {
  const dir = freshWorkspace();
  try {
    fn(dir);
  } finally {
    cleanup(dir);
  }
}

function withEmptyDir(fn) {
  const dir = freshDir();
  try {
    fn(dir);
  } finally {
    cleanup(dir);
  }
}

// ---- TIX-14 init ----

test('TIX-31.3 init: creates tix.yaml and tickets/ in a fresh directory', () => {
  withEmptyDir((dir) => {
    const r = invoke(dir, ['init']);
    assertGolden('init_fresh', maskWs(r.transcript, dir));
    assert.strictEqual(r.code, 0);
    // The freshly written tix.yaml is always LF-only (a Rust string literal
    // baked into the wasm module); the fixture copy on disk may have been
    // CRLF-translated by a Windows checkout, so normalize that side only.
    const yaml = fs.readFileSync(path.join(dir, 'tix.yaml'), 'utf8');
    const expected = readTextNormalized(
      path.join(__dirname, '..', 'fixtures', 'workspace', 'tix.yaml')
    );
    assert.strictEqual(yaml, expected);
    assert.ok(fs.statSync(path.join(dir, 'tickets')).isDirectory());
  });
});

test('TIX-31.3 init: refuses an existing workspace, exit 2', () => {
  withFixture((dir) => {
    const before = fs.readFileSync(path.join(dir, 'tix.yaml'), 'utf8');
    const r = invoke(dir, ['init']);
    assertGolden('init_already_exists', maskWs(r.transcript, dir));
    assert.strictEqual(r.code, 2);
    assert.strictEqual(fs.readFileSync(path.join(dir, 'tix.yaml'), 'utf8'), before);
  });
});

// ---- TIX-22 check ----

test('TIX-31.3 check: ok on a freshly initialized workspace', () => {
  withEmptyDir((dir) => {
    invoke(dir, ['init']);
    const r = invoke(dir, ['check']);
    assertGolden('check_ok', r.transcript);
    assert.strictEqual(r.code, 0);
  });
});

test('TIX-31.3 check: reports one line per problem, exit 1', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['check']);
    assertGolden('check_problems', r.transcript);
    assert.strictEqual(r.code, 1);
  });
});

// ---- TIX-15 new ----

test('TIX-31.3 new: creates a ticket from flags alone', () => {
  withFixture((dir) => {
    const r = invoke(dir, [
      'new',
      '--title',
      'Article title',
      '--client',
      'acme',
      '--type',
      'article',
      '--owner',
      'charlie',
      '--due',
      '2026-10-01',
    ]);
    assertGolden('new_all_flags', r.transcript);
    assert.strictEqual(r.code, 0);
    assert.strictEqual(r.stdout, `${FIRST_NEW_ID}\n`);
    const body = fs.readFileSync(
      path.join(dir, 'tickets', FIRST_NEW_ID, 'ticket.md'),
      'utf8'
    );
    assert.match(body, /title: Article title/);
    assert.match(body, /client: acme/);
    assert.match(body, /type: article/);
    assert.match(body, /owner: charlie/);
    assert.match(body, /due: 2026-10-01/);
    assert.match(body, /status: backlog/);
  });
});

test('TIX-31.3 new: required fields come from the scripted prompt', () => {
  withFixture((dir) => {
    const r = invokeWithPrompt(dir, ['new', '--title', 'Prompted ticket'], {
      answers: ['acme', 'article'],
    });
    assertGolden('new_prompts_required', r.transcript);
    assert.strictEqual(r.code, 0);
    assert.deepStrictEqual(r.promptCalls, [
      { label: 'client', kind: 'string', options: [] },
      { label: 'type', kind: 'enum', options: ['article', 'landing_page', 'linkedin', 'other'] },
    ]);
    const body = fs.readFileSync(
      path.join(dir, 'tickets', FIRST_NEW_ID, 'ticket.md'),
      'utf8'
    );
    assert.match(body, /client: acme/);
    assert.match(body, /type: article/);
    assert.doesNotMatch(body, /owner:/);
    assert.doesNotMatch(body, /due:/);
  });
});

test('TIX-31.3 new: --no-prompt with a missing required field exits 1', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['new', '--title', 'T', '--no-prompt']);
    assertGolden('new_no_prompt_missing_required', r.transcript);
    assert.strictEqual(r.code, 1);
    assert.strictEqual(
      fs.readdirSync(path.join(dir, 'tickets')).length,
      5,
      'no ticket folder created'
    );
  });
});

// ---- TIX-16 ls ----

test('TIX-31.3 ls: lists every ticket, invalid ones marked with !', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['ls']);
    assertGolden('ls_all', r.transcript);
    assert.strictEqual(r.code, 0);
  });
});

test('TIX-31.3 ls: unknown filter key exits 2', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['ls', 'bogus:val']);
    assertGolden('ls_unknown_key', r.transcript);
    assert.strictEqual(r.code, 2);
  });
});

// ---- TIX-17 show ----

test('TIX-31.3 show: prints frontmatter, deliverables, and body', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['show', T_PROGRESS.slice(0, 6)]);
    assertGolden('show_valid_progress', r.transcript);
    assert.strictEqual(r.code, 0);
  });
});

test('TIX-31.3 show: ambiguous id prefix exits 2', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['show', '01K5']);
    assertGolden('show_ambiguous', r.transcript);
    assert.strictEqual(r.code, 2);
  });
});

// ---- TIX-18 mv ----

test('TIX-31.3 mv: changes status and updated', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['mv', T_BACKLOG.slice(0, 6), 'in_progress']);
    assertGolden('mv_ok', r.transcript);
    assert.strictEqual(r.code, 0);
    const body = fs.readFileSync(path.join(dir, 'tickets', T_BACKLOG, 'ticket.md'), 'utf8');
    assert.match(body, /status: in_progress/);
    assert.match(body, /updated: 2026-09-14T15:02:11Z/);
  });
});

test('TIX-31.3 mv: unknown status exits 1 and writes nothing', () => {
  withFixture((dir) => {
    const before = fs.readFileSync(path.join(dir, 'tickets', T_BACKLOG, 'ticket.md'), 'utf8');
    const r = invoke(dir, ['mv', T_BACKLOG.slice(0, 6), 'bogus']);
    assertGolden('mv_unknown_status', r.transcript);
    assert.strictEqual(r.code, 1);
    assert.strictEqual(
      fs.readFileSync(path.join(dir, 'tickets', T_BACKLOG, 'ticket.md'), 'utf8'),
      before
    );
  });
});

// ---- TIX-19 set ----

test('TIX-31.3 set: updates title and a field together', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['set', T_BACKLOG.slice(0, 6), 'title=New title', 'owner=alex']);
    assertGolden('set_title_and_field', r.transcript);
    assert.strictEqual(r.code, 0);
    // Normalize CRLF the checkout may have introduced into the *original*
    // fixture body; commands preserve the body byte-for-byte (TIX-12), so a
    // Windows checkout would otherwise carry that CRLF straight through.
    const body = readTextNormalized(path.join(dir, 'tickets', T_BACKLOG, 'ticket.md'));
    assert.strictEqual(
      body,
      [
        '---',
        `id: ${T_BACKLOG}`,
        'title: New title',
        'status: backlog',
        'created: 2026-09-10T09:00:00Z',
        'updated: 2026-09-14T15:02:11Z',
        'deliverables: []',
        'client: nova',
        'type: article',
        'owner: alex',
        'due: 2026-09-30',
        '---',
        'Brief goes here.\n',
      ].join('\n')
    );
  });
});

test('TIX-31.3 set: unknown field key exits 1', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['set', T_BACKLOG.slice(0, 6), 'bogus=foo']);
    assertGolden('set_unknown_key', r.transcript);
    assert.strictEqual(r.code, 1);
  });
});

test('TIX-31.3 set: repairs ticket with status= and field', () => {
  withFixture((dir) => {
    // Break the fixture further: unknown status AND missing required
    // `client`. Neither `mv` alone (fails rule 3) nor a field-only `set`
    // (fails rule 2) can repair this; `set status=... field=...` can
    // (PLAN.md decision on TIX-19 vs TIX-10).
    const ticketPath = path.join(dir, 'tickets', T_UNKNOWN_STATUS, 'ticket.md');
    const broken = fs.readFileSync(ticketPath, 'utf8').replace('client: acme\n', '');
    fs.writeFileSync(ticketPath, broken);

    const mvFail = invoke(dir, ['mv', T_UNKNOWN_STATUS.slice(0, 6), 'backlog']);
    assert.strictEqual(mvFail.code, 1);
    const setFail = invoke(dir, ['set', T_UNKNOWN_STATUS.slice(0, 6), 'client=acme']);
    assert.strictEqual(setFail.code, 1);

    const r = invoke(dir, [
      'set',
      T_UNKNOWN_STATUS.slice(0, 6),
      'status=backlog',
      'client=acme',
    ]);
    assertGolden('set_status_repair', r.transcript);
    assert.strictEqual(r.code, 0);
    const after = fs.readFileSync(ticketPath, 'utf8');
    assert.match(after, /status: backlog\n/);
    assert.match(after, /client: acme\n/);
  });
});

test('TIX-31.3 set: repairs a ticket missing a required field', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['set', T_NO_CLIENT.slice(0, 6), 'client=acme']);
    assertGolden('set_repair_missing_client', r.transcript);
    assert.strictEqual(r.code, 0);
    const after = fs.readFileSync(path.join(dir, 'tickets', T_NO_CLIENT, 'ticket.md'), 'utf8');
    assert.match(after, /client: acme/);
  });
});

// ---- TIX-20 attach / detach ----

test('TIX-31.3 attach: appends a deliverable with an explicit label', () => {
  withFixture((dir) => {
    const r = invoke(dir, [
      'attach',
      T_BACKLOG.slice(0, 6),
      'https://docs.google.com/document/d/xyz',
      '--label',
      'draft',
    ]);
    assertGolden('attach_explicit_label', r.transcript);
    assert.strictEqual(r.code, 0);
    const body = fs.readFileSync(path.join(dir, 'tickets', T_BACKLOG, 'ticket.md'), 'utf8');
    assert.match(body, /label: draft\n\s*ref: https:\/\/docs\.google\.com\/document\/d\/xyz/);
  });
});

test('TIX-31.3 attach: a ref already attached exits 1', () => {
  withFixture((dir) => {
    const r = invoke(dir, [
      'attach',
      T_PROGRESS.slice(0, 6),
      'https://docs.google.com/document/d/abc',
    ]);
    assertGolden('attach_duplicate_ref', r.transcript);
    assert.strictEqual(r.code, 1);
  });
});

test('TIX-31.3 detach: removes the deliverable matching a label', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['detach', T_PROGRESS.slice(0, 6), 'outline']);
    assertGolden('detach_by_label', r.transcript);
    assert.strictEqual(r.code, 0);
    const body = fs.readFileSync(path.join(dir, 'tickets', T_PROGRESS, 'ticket.md'), 'utf8');
    assert.doesNotMatch(body, /outline/);
  });
});

test('TIX-31.3 detach: no matching deliverable exits 1', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['detach', T_PROGRESS.slice(0, 6), 'nope']);
    assertGolden('detach_no_match', r.transcript);
    assert.strictEqual(r.code, 1);
  });
});

// ---- TIX-21 board ----

test('TIX-31.3 board: one column per status plus a trailing ? column', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['board']);
    assertGolden('board_default', r.transcript);
    assert.strictEqual(r.code, 0);
  });
});

test('TIX-31.3 board: a filter token without a colon exits 2', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['board', 'bogus']);
    assertGolden('board_no_colon', r.transcript);
    assert.strictEqual(r.code, 2);
  });
});

// ---- TIX-23 path ----

test('TIX-31.3 path: prints the workspace-relative ticket folder', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['path', T_BACKLOG.slice(0, 6)]);
    assertGolden('path_ok', r.transcript);
    assert.strictEqual(r.code, 0);
    assert.strictEqual(r.stdout, `tickets/${T_BACKLOG}\n`);
  });
});

test('TIX-31.3 path: a too-short prefix exits 2', () => {
  withFixture((dir) => {
    const r = invoke(dir, ['path', '01K']);
    assertGolden('path_too_short', r.transcript);
    assert.strictEqual(r.code, 2);
  });
});
