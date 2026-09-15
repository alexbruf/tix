# tix

[![ci](https://github.com/alexbruf/tix/actions/workflows/ci.yml/badge.svg)](https://github.com/alexbruf/tix/actions/workflows/ci.yml)

A ticket board that lives in a folder. Tickets are Markdown files with YAML frontmatter, the schema is one `tix.yaml`, and there is no server, database or login. Built to be driven by people and AI agents alike.

```
my-board/
  tix.yaml                                  # statuses and fields
  tickets/01K5AQ9Z3R7M8N2P4Q6S8T0V1W/
    ticket.md                               # frontmatter + brief
```

## Install

**macOS / Linux**

```sh
curl -fsSL https://raw.githubusercontent.com/alexbruf/tix/main/install.sh | sh
```

Installs the latest release to `~/.local/bin/tix`. Set `TIX_INSTALL_DIR` to choose another folder.

**Windows**: download `tix-x86_64-pc-windows-msvc.zip` from [Releases](https://github.com/alexbruf/tix/releases), unzip, and put `tix.exe` on your `PATH`.

**With Rust**

```sh
cargo install --git https://github.com/alexbruf/tix tix-cli
```

**WASI**: `tix.wasm` from Releases runs anywhere with a WASI runtime:

```sh
wasmtime run --dir=. tix.wasm board
```

Check it works:

```sh
tix --version
```

## Quick start

```sh
mkdir my-board && cd my-board
tix init
tix new --title "Q4 comparison article" --client acme --type article
tix ls
tix mv 01K5 in_progress               # any unique id prefix of 4+ characters
tix attach 01K5 https://docs.google.com/document/d/abc --label draft
tix board
```

Every command has detailed help with argument formats, output, JSON shapes and examples:

```sh
tix --help
tix new --help        # inside a board: lists its fields and allowed values
```

## Use with AI agents

`tix` is designed to be scripted: `--no-prompt` never waits on stdin, `--json` prints one JSON document, errors go to stderr, and exit codes are stable (`0` ok, `1` validation, `2` usage, `3` I/O).

**MCP server**: the `tix` binary is also a local MCP server. Every command becomes a tool (`tix_new`, `tix_ls`, `tix_set`, ...) that returns the command's JSON or an error naming the broken rule.

```sh
claude mcp add tix -- tix mcp                       # Claude Code
```

Other clients (Claude Desktop, Cursor, ...):

```json
{ "mcpServers": { "tix": { "command": "tix", "args": ["mcp", "--workspace", "/path/to/board"] } } }
```

Tools take an optional `workspace` path; `--workspace` sets the default (otherwise the server's working directory). See `tix mcp --help`.

**Claude Code skill**: teaches agents how to use `tix` well (CLI or MCP).

```sh
mkdir -p ~/.claude/skills/tix
curl -fsSL https://raw.githubusercontent.com/alexbruf/tix/main/skills/tix/SKILL.md -o ~/.claude/skills/tix/SKILL.md
```

## Commands

| Command | What it does |
|---|---|
| `tix init` | Create `tix.yaml` and `tickets/` in the current folder |
| `tix new` | Create a ticket; fields become `--<field>` flags |
| `tix ls [KEY:VALUE ...]` | List tickets, filtered by `status:`, `group:` or any field |
| `tix show ID` | Print a ticket's fields, deliverables and brief |
| `tix mv ID STATUS` | Change status |
| `tix set ID KEY=VALUE ...` | Change title, status or fields in one write |
| `tix attach ID REF` / `tix detach ID REF_OR_LABEL` | Link or unlink deliverables (URLs, paths; never copied) |
| `tix board [--group]` | Kanban-style columns |
| `tix check` | Validate the schema and every ticket |
| `tix path ID` | Print the ticket folder path |

## The schema

`tix init` writes this default. Edit it freely, then run `tix check`; tickets that no longer fit are still shown, marked `!`.

```yaml
version: 1
statuses:
  - name: backlog
    group: backlog
  - name: in_progress
    group: in_progress
  - name: done
    group: completed
fields:
  - name: client
    type: string
    required: true
  - name: type
    type: enum
    values: [article, landing_page, linkedin, other]
    required: true
  - name: owner
    type: string
  - name: due
    type: date
```

Field types: `string`, `enum` (with `values`), `date` (`YYYY-MM-DD`), `list`. Every status belongs to a group: `backlog`, `in_progress` or `completed`.

## How it's built

- `crates/tix-core`: the ticket logic (validation, writes, filtering, sorting, board layout), formally verified with [Verus](https://github.com/verus-lang/verus). Validation is proved equivalent to the spec; every write is proved to keep tickets valid and change only what it names.
- `crates/tix-io`: argument parsing, YAML and Markdown, tables, and all commands, behind a five-method storage trait.
- `crates/tix-cli`: the `tix` binary (native or `wasm32-wasip1`) and the `tix mcp` server.
- `crates/tix-wasm` + `bin/tix.js`: the same core as a WebAssembly npm package for Node 20+.

Requirements are in [`tix.sdoc`](tix.sdoc); design decisions are logged in [`PLAN.md`](PLAN.md); contributor notes in [`CLAUDE.md`](CLAUDE.md).

## Development

Requires Rust (pinned in `rust-toolchain.toml`), [Verus](https://github.com/verus-lang/verus/releases) `0.2026.09.13.671956e`, `wasm-pack`, and Node 20+.

```sh
npm run ci                                   # verus verify, cargo test, wasm build, node tests
cargo install --path crates/tix-cli          # local tix binary
```

Releases: push a `v*` tag and the release workflow attaches binaries for macOS, Linux, Windows and WASI. npm publishing runs only when the `NPM_TOKEN` repository secret is set (see `.env.example`).

## License

MIT
