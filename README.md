# tix

A file-based ticket board for content work. A workspace is a folder of `ticket.md` files plus one `tix.yaml` schema. No database, no server.

```sh
npx @viewengine/tix init
npx @viewengine/tix new --title "Product comparison" --client nova --type article
npx @viewengine/tix board
```

- Spec: [`tix.sdoc`](tix.sdoc) (strictdoc, requirements TIX-1 to TIX-31)
- Build plan: [`PLAN.md`](PLAN.md)
- Contributor notes: [`CLAUDE.md`](CLAUDE.md)
- Agent skill: [`skills/tix/SKILL.md`](skills/tix/SKILL.md) (install: `ln -s "$PWD/skills/tix" ~/.claude/skills/tix`)

## Environment

`NPM_TOKEN` is only used by CI to publish on tags. See `.env.example`.

## Commands

`init`, `check`, `new`, `ls`, `show`, `mv`, `set`, `attach`, `detach`, `board`, `path`. Global flags: `--json`, `--no-prompt`. Exit codes: `0` ok, `1` validation, `2` usage or workspace, `3` I/O.

## Development

Requires Rust (pinned in `rust-toolchain.toml`), Verus `0.2026.09.13.671956e`, `wasm-pack`, and Node 20+.

```sh
npm run ci   # verus verify, cargo test, wasm-pack build, node tests
```

## Status

v1 feature-complete locally: core proofs, Rust and Node suites pass. Not yet published; CI has not run on a remote.
