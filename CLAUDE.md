# tix

File-based ticket board CLI. Rust core verified with Verus, compiled to wasm, wrapped by a small Node host, shipped as one npm package `@viewengine/tix`.

## Source of truth

- `tix.sdoc` is the spec. Requirements are `TIX-1` to `TIX-31`. Do not edit it without asking the user.
- `docs/HANDOFF.md` holds the fixed decisions and the order of work.
- `PLAN.md` is the working plan and the log of decisions made where the spec is silent.
- If a requirement is impossible or contradictory, stop and name the UID and the reason.

## Layout (target)

```
Cargo.toml                 workspace
rust-toolchain.toml        pinned to the toolchain the pinned Verus release needs
crates/tix-core/           VERIFIED. Deps: vstd, verus_builtin, verus_builtin_macros only
crates/tix-io/             clap, serde_yaml, gray_matter, comfy-table, ulid, Storage trait
crates/tix-wasm/           wasm-bindgen glue, exports run(argv, cwd) -> u32
crates/tix-cli/            standalone `tix` binary (native or wasm32-wasip1), host over std; not in npm
bin/tix.js                 Node host (~150 lines), FsAdapter over node:fs
dist/                      wasm-pack output (tix_wasm.js, tix_wasm_bg.wasm), not committed
tests/node/                Node integration + host-contract suites, golden files
tests/fixtures/            fixture workspaces
.github/workflows/ci.yml   TIX-30 pipeline
```

## Commands

```sh
cargo verus verify -p tix-core                              # proofs (TIX-2, TIX-24..28)
cargo build -p tix-core                                     # must also build without Verus
cargo test --workspace                                      # tix-io tests, native target
wasm-pack build crates/tix-wasm --target nodejs --release   # wasm
node --test tests/node/                                     # integration suite (runs on Node, not Bun)
npm pack                                                    # package check (TIX-29)
cargo install --path crates/tix-cli                         # native `tix` on PATH
cargo build -p tix-cli --release --target wasm32-wasip1     # portable WASI build
```

## Hard rules (from the handoff)

- `tix-core`: no deps besides Verus crates. No `HashMap`; use `Vec` with proved uniqueness. Plain `String`, `Vec`, structs, enums, `Option`, `Result`.
- A proof that does not close is never a reason to move code out of `tix-core`. Simplify the function or ask.
- Outside `tix-core`, use existing crates. No hand-rolled YAML parser, table renderer, or argv parser.
- All filesystem access goes through the five-method `Storage` trait (TIX-3). Paths are workspace-relative, `/`-separated, never `..`.
- wasm target `wasm32-unknown-unknown`, wasm-bindgen target `nodejs`. No native binaries, no postinstall.
- Deliverables are references, never copies.
- Exit codes: `0` ok, `1` validation, `2` usage/workspace, `3` I/O.

## Help text

- `crates/tix-io/src/help.rs` holds the long `--help` text, written for AI agents: formats, output and JSON shapes, exit codes, examples. Update it with any behaviour change; `tests/help.rs` guards the sections.

## Traceability

- Every `Proof` requirement maps to at least one `ensures` clause. Put the UID in a comment on it: `// TIX-25`.
- Every `Test` requirement maps to at least one named test with the UID in its name: `tix_14_init_refuses_existing_workspace`.
- No test re-checks a property already proved in TIX-24 to TIX-28 (TIX-31).

## Ask the user before

- Adding a dependency to `tix-core`.
- Changing the `Storage` trait.
- Changing any exit code.
- Adding a command not in the spec.
- Changing the shape of `tix.yaml`.
- Publishing to npm or pushing to a remote.

Everything else: decide, then record it under "Decisions" in `PLAN.md`.

## Tooling notes

- User default is `bun`/`bunx`, but this product is a Node 20 package: the integration suite runs under `node --test` and packaging uses `npm pack` / `npm publish` because the spec names them. Use `bun` only for ad-hoc scripting.
- Verus is installed from a pinned GitHub release binary; record the version in `PLAN.md` and `rust-toolchain.toml`.

## Git

- `main` holds docs and merged, green work only. Build work happens on `build/v1`.
- Do not merge to `main` until handoff step 6 (wasm + Node suite) is green.
- One commit per completed plan step, message prefixed with the step, e.g. `step 2: tix-core validation proofs`.
- Never commit `.env`, `dist/`, `target/`, `node_modules/`.

## Subagents

- Use Sonnet subagents for bounded, well-specified work: tix-io commands, golden files, Node host, CI YAML, fixture workspaces.
- Keep Verus proof design for `tix-core` in the main session; delegate only mechanical follow-up.
- Give each subagent the UIDs it owns and the exact files it may touch. Run agents that touch disjoint files in parallel; use a worktree when they could collide.
