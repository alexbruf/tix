# tix build plan

Status (2026-09-14): steps 0-6 done and green locally; step 7 verified on macOS via `npm pack` + `npx --package`. Linux/Windows runs and publishing wait for a remote and user approval.

Follows the order of work in `docs/HANDOFF.md`. Each step ends with a green gate and a commit on `build/v1`.

## Step 0: Toolchain and repo (done in this commit, minus Verus install)

- git repo, `CLAUDE.md`, this plan, spec and handoff copied in.
- Install Verus release `0.2026.09.13.671956e` (arm64-macos locally, x86-linux in CI). Pin its required Rust toolchain in `rust-toolchain.toml`.
- Gate: `verus --version` and `cargo verus --help` run.

## Step 1: Workspace skeleton and green CI

- Cargo workspace with `tix-core`, `tix-io`, `tix-wasm` as empty crates that compile.
- `bin/tix.js` stub, `package.json` (`@viewengine/tix`, `bin`, `files`, `engines.node >=20`, no postinstall).
- `.github/workflows/ci.yml`: verus verify, cargo test, wasm-pack build, node test, publish on tags (TIX-30).
- Gate: all four local commands pass on the empty core.
- Owner: main session (small, sets conventions).

## Step 2: tix-core types and validation (TIX-9, TIX-11, TIX-24)

- Types: `Group`, `Status`, `FieldType`, `Field`, `Schema`, `FieldValue` (Str / List), `Deliverable`, `Ticket`, error enums naming the rule number.
- Spec fns: `valid_ulid`, `valid_date`, `valid_field_name`, `valid_schema`, `valid_ticket`.
- `Schema::validate` and `Ticket::validate` with `ensures result is Ok <==> valid_*`, and error variant = first violated rule.
- Gate: `cargo verus verify -p tix-core` zero errors; `cargo build -p tix-core` passes.
- Owner: main session.

## Step 3: tix-core operations (TIX-7, TIX-15..21 core parts, TIX-25..28)

- `new_ticket`, `transition`, `set_fields`, `attach`, `detach` with TIX-25 ensures.
- `resolve(prefix, ids)` (TIX-28), `filter` + `parse_query` (TIX-27), `sort_for_ls` (TIX-16), `board` + group mode (TIX-26, TIX-21).
- Gate: verify green.
- Owner: main session, one proof at a time.

## Step 4: tix-io parsing and storage (TIX-3, TIX-6, TIX-8, TIX-12, TIX-13, TIX-31.1, TIX-31.2)

- `tix.yaml` parse/render via serde_yaml into core `Schema`.
- `ticket.md` parse/render: frontmatter via gray_matter + serde_yaml, fixed key order, body byte-exact.
- `Storage` trait, `MemStorage` for tests, workspace discovery walking up from cwd.
- ULID from `now_unix` + `random_bytes` via `ulid::Ulid::from_parts`.
- proptest round trip (31.1); schema rejection cases (31.2).
- Gate: `cargo test --workspace`.
- Owner: Sonnet subagent, main session reviews.

## Step 5: tix-io commands (TIX-14..23)

- clap command tree, global `--json` and `--no-prompt`, exit-code mapping.
- Order: `init`, `check`, `new`, `ls`, `show`, `mv`, `set`, `attach`, `detach`, `board`, `path`. Golden-file test per command against `MemStorage`.
- Gate: `cargo test --workspace`; native binary usable as fallback.
- Owner: Sonnet subagents, split into two parallel batches over disjoint files (read commands / write commands) after a shared `cli.rs` skeleton lands.

## Step 6: wasm and Node host (TIX-4, TIX-5, TIX-31.3, TIX-31.4)

- `tix-wasm`: the ten imports, `run(argv, cwd) -> u32`, `Storage` impl over imports.
- `bin/tix.js`: FsAdapter over `node:fs`, prompt, stdout/stderr, clock, crypto random.
- Node suite: fixture workspace + golden files per command through real wasm; fake FsAdapter recording calls (no writes on validation error).
- Gate: full TIX-30 pipeline green locally. Then merge `build/v1` to `main`.
- Owner: Sonnet subagent for host + suite; main session for wasm glue.

## Step 7: Packaging (TIX-29)

- `npm pack`, install tarball into a clean temp dir, run the definition-of-done command chain.
- CI matrix on macOS, Linux, Windows with Node 20.
- Publishing waits for user approval.

## Decisions (spec is silent; recorded, not asked)

- TIX-4 vs TIX-5 (sync `prompt` vs async `node:readline`): the host runs `node:readline` in a `worker_threads` worker and blocks the main thread with `Atomics.wait` on a `SharedArrayBuffer` until the answer arrives. Both requirements hold as written. User approved (2026-09-14).

- Core timestamps are `u64` unix seconds; `tix-io` renders RFC 3339 UTC.
- Status `group` is a `String` in the core so TIX-9 rule 2 is proved there; field `type` is an enum because TIX-8 (parse) owns that check.
- A ticket with duplicate field keys violates TIX-11 rule 4 (unrepresentable in YAML anyway; keeps lookups well-defined).
- Write operations (TIX-25) do not require `valid_ticket(t)` as a precondition, so `set`/`mv` can repair tickets broken by a schema change (TIX-10). They validate the result instead.
- `detach` failures (no match, or more than one match) use their own error type; they are not TIX-11 rules.
- Field `default`s are applied by `tix-io` in `new` before prompting; a required field with a default is not prompted.
- `board`, `filter` and `ls` sorting return indices into the input `Vec<Ticket>`, which keeps the partition proofs simple.
- An invalid `tix.yaml` makes every command except `init` and `check` exit 1 with the schema error.
- `tix set` with the same KEY twice is a usage error (exit 2).
- **Deviation from TIX-19 (user said "figure it out", 2026-09-14):** `tix set` also accepts `status=S`. Without it, a ticket with an unknown status and a missing required field could not be repaired by any single command, since `mv` fails rule 3 and `set` fails rule 2 (TIX-10 vs TIX-25). `set_fields` takes `status: Option<String>` and keeps every TIX-25 guarantee.
- Verus pinned: release `0.2026.09.13.671956e`, Rust `1.98.1`, `vstd =0.0.0-2026-09-06-0133`.
- `gray_matter` is not used: it trims the body, which breaks TIX-12 byte-for-byte preservation. Frontmatter is split on the `---` lines by hand and parsed with `serde_yaml`.
- `serde_yaml` writes `deliverables` list items flush with the key (`- label: x`), not indented as in the TIX-12 example. Both forms parse.
- Storage paths reaching the host are absolute; commands address them through `Ctx`, which implements `Storage` with workspace-relative paths (TIX-3).
- The wasm module imports its ten functions from `globalThis.tix_host`; wasm-bindgen's internal `__wbindgen_*` shims are not counted as imports.
- `tix check` prints problems on stdout and `N problem(s)` on stderr; exit 1.
- `tix check` also reports a folder name that differs from the ticket `id` (TIX-7) and a folder without `ticket.md`.
- Unparseable `ticket.md` files are skipped with a stderr warning by `ls`/`board`; `show` on one exits 1.
- Write commands print the ticket id on success; `--json` prints the ticket object.
- `tix set KEY=` on a key not in the schema removes it (repairs TIX-10 leftovers); `id`/`created`/`updated`/`deliverables` are exit 2.
- `list` field values on the command line are comma-separated.
- `attach` label default: last non-empty `/` segment of the ref, else the whole ref.
- Id `NotFound`/`TooShort`/`Ambiguous` all exit 2.
- Board uses comfy-table `ASCII_BORDERS_ONLY_CONDENSED`; `ls` uses the borderless `NOTHING` preset. ASCII keeps Windows consoles readable.
- `?` board column appears only when non-empty (TIX-21 wording wins over TIX-26's).
- With `--group`, unknown-status tickets still go to a trailing `?` column.
- `filter` takes the schema as an extra argument, since `group:v` needs it (TIX-27).
- ULID well-formed = 26 chars, Crockford base32 uppercase, first char `0`..`7` (no 128-bit overflow).
- Id `NotFound` and `TooShort` exit 2, same as `Ambiguous` (usage error class).
- TIX-4 "exactly these imports" is read as the user-level imports; wasm-bindgen's internal `__wbindgen_*` shims are not counted.
- `serde_yaml` is archived upstream but is named by the handoff, so it is used as-is (0.9).
- **Fourth crate `tix-cli` (user request, 2026-09-14):** a standalone `tix` executable. Wasmer removed `create-exe`, so the user chose a native Rust build; the same crate also builds for `wasm32-wasip1` (runs under wasmtime `--dir=.` or wasmer `--volume .:/ws --env TIX_CWD=/ws`). It is not part of the npm package, which stays pure wasm (TIX-29).
- **`tix mcp` (user request, 2026-09-14):** the native `tix-cli` binary serves every command as an MCP tool over stdio using `rmcp`. Tools run the real command in-process with `--json --no-prompt`, so behaviour matches the CLI exactly. Not available in the WASI build or the npm package.
- `--help` (long) is written for AI agents; `-h` stays a short summary.
- `tix-spec-html.zip` is a rendering of `tix.sdoc`; not committed.
