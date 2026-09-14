# tix: build handoff

You are building `tix`, a file-based ticket board CLI. The spec is `tix.sdoc` (strictdoc format) in this repo. It is the source of truth. Every requirement has a UID (`TIX-1` to `TIX-31`) and a `VERIFICATION` field: `Proof` means a Verus proof, `Test` means an automated test, `Inspection` means a reviewer checks it. Do not change the spec without asking; if a requirement is impossible or contradictory, stop and say which UID and why.

## What it is

A Rust core compiled to wasm, verified with Verus, wrapped by a ~150-line Node host, published as one npm package. Storage is a folder of `ticket.md` files with YAML frontmatter plus one `tix.yaml` schema file. No database, no server, no TUI.

Read `tix.sdoc` fully before writing any code. The Architecture section and TIX-2 through TIX-5 fix the crate layout and the host interface.

## Fixed decisions (do not relitigate)

- Three crates: `tix-core` (verified, no deps except `vstd`), `tix-io` (clap, serde_yaml, gray_matter, comfy-table, ulid, the `Storage` trait), `tix-wasm` (wasm-bindgen glue).
- Target `wasm32-unknown-unknown`, wasm-bindgen target `nodejs`. No native binaries, no postinstall.
- All filesystem access through the five-function `Storage` trait (TIX-3). The Node host implements it over `node:fs`.
- Use existing crates for everything outside `tix-core`. Do not hand-roll a YAML parser, a table renderer, or an argv parser.
- Statuses and required fields come from `tix.yaml`; every status has a group `backlog | in_progress | completed`.
- Deliverables are stored as references, never copied.

## Order of work

1. Cargo workspace with the three crates, CI running `cargo verus verify -p tix-core`, `cargo test --workspace`, `wasm-pack build`, and the Node test suite (TIX-30). Get a green pipeline on an empty core before anything else.
2. `tix-core`: types (`Schema`, `Ticket`, `Deliverable`, groups), spec predicates `valid_schema` and `valid_ticket`, then `Schema::validate` and `Ticket::validate` proved equivalent to the predicates (TIX-9, TIX-11, TIX-24).
3. `tix-core`: `new_ticket`, `transition`, `set_fields`, `attach`, `detach` with the `ensures` clauses in TIX-25. Then `board`, `filter`, `resolve` (TIX-26 to TIX-28).
4. `tix-io`: `tix.yaml` and `ticket.md` parse and render (TIX-8, TIX-12), property test for round trip (TIX-31.1), `Storage` trait, workspace discovery (TIX-6).
5. `tix-io` commands in this order: `init`, `check`, `new`, `ls`, `show`, `mv`, `set`, `attach`, `detach`, `board`, `path` (TIX-14 to TIX-23). Golden-file test per command.
6. `tix-wasm` + `bin/tix.js`, then the Node integration suite against the real wasm (TIX-31.3, TIX-31.4).
7. `npm pack` and confirm `npx` works from a clean machine on macOS, Linux and Windows (TIX-29).

Ship after step 5 as a native binary if wasm is blocking you, but do not merge to main without step 6.

## Verus notes

- Install per https://verus-lang.github.io/verus/guide/getting_started.html. Pin the Verus release and the matching Rust toolchain in `rust-toolchain.toml`.
- `tix-core` must build with plain `cargo build` as well; Verus annotations erase.
- Keep `tix-core` types to what `vstd` supports: `Vec`, `String`, structs, enums, `Option`, `Result`. No `HashMap` in the verified surface; use `Vec` with proved uniqueness.
- A proof that will not close is not a reason to move the function to `tix-io`. Simplify the function until it closes, or ask.

## Definition of done

- CI green on every step in TIX-30.
- `tix check` returns `ok` on the fixture workspace and reports each of the rejection cases in TIX-9 and TIX-11.
- `npx @viewengine/tix init && npx @viewengine/tix new --title x --client y --type article && npx @viewengine/tix board` works on a clean machine with Node 20 and nothing else.
- Every `Proof` requirement maps to at least one `ensures` clause; every `Test` requirement maps to at least one named test. Put the UID in the test name or the proof comment.

## When to ask

Ask before: adding a dependency to `tix-core`, changing the `Storage` trait, changing any exit code, adding a command not in the spec, or changing `tix.yaml`'s shape. Everything else, decide and note it in the PR.
