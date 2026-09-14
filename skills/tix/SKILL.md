---
name: tix
description: Manage a file-based ticket board with the `tix` CLI (tickets are tickets/<ID>/ticket.md files, the schema is tix.yaml). Use when the user wants to create, find, list, update, move, or review tickets or content-pipeline work items in a folder that has (or should have) a tix.yaml; when they mention "tix", "the board", "ticket.md", or tickets moving between statuses like backlog/in_progress/done; or when attaching deliverable links (Google Docs, drafts, paths) to a ticket. Do NOT use for Jira, Linear, GitHub Issues, or other hosted trackers.
---

# tix

`tix` is a command-line ticket board stored as plain files. No server, no database, no login.

```
<workspace>/tix.yaml                  statuses and fields (the schema)
<workspace>/tickets/<ID>/ticket.md    YAML frontmatter + Markdown brief
```

Commands find the workspace by walking up from the current directory to the nearest `tix.yaml`.

## Rule 1: the CLI help is the source of truth

This skill is a map; the help has the details and always matches the installed version.

- Run `tix --help` once per session: agent quickstart, schema, validity rules, JSON shapes, exit codes.
- Run `tix <command> --help` before the first use of each command: exact argument formats, output, `--json` shape, errors, examples.
- Run `tix new --help` **inside the workspace** before creating tickets: it lists that workspace's own fields with type, required, allowed enum values and defaults. Never guess field names or enum values.

## Rule 2: always run non-interactively

- Pass `--no-prompt` on every `tix new` (and any write). Without it a missing required field waits on stdin and hangs you.
- Pass `--json` whenever you read the output. Every command prints exactly one JSON document on stdout; errors go to stderr.
- Use full ids from `--json` output when you act on a ticket. The 8-character short ids in tables can collide for tickets created in the same second.

## Setup check

```sh
tix --version          # installed?
tix check --json       # inside a workspace? {"ok": true, "problems": []}
```

- `tix: command not found`: tell the user; it installs with `cargo install --path crates/tix-cli` from the tix repo.
- `no tix.yaml found` (exit 2): there is no workspace here or above. **Ask the user** before running `tix init`, and where; it creates `tix.yaml` and `tickets/` in the current directory.

## Recipes

Create a ticket (read `tix new --help` first for the fields):

```sh
id=$(tix new --no-prompt --title "Product comparison article" --client nova --type article)
tix new --no-prompt --json --title "Launch post" --client acme --type linkedin --status in_progress --due 2026-10-01
```

Find tickets:

```sh
tix ls --json                                  # everything, in board order
tix ls status:backlog client:nova --json       # filters are KEY:VALUE, ANDed
tix ls group:in_progress --json                # group: backlog | in_progress | completed
tix show <ID> --json                           # one ticket, including "body" (the brief)
```

Update a ticket (one all-or-nothing write; fields not named stay unchanged):

```sh
tix set <ID> owner=charlie due=2026-09-30
tix set <ID> "title=Product vs alternate"
tix set <ID> owner=                            # KEY= removes an optional field
tix mv <ID> done                               # status only
```

Deliverables are references (URLs, paths, ids); tix never copies or downloads them:

```sh
tix attach <ID> https://docs.google.com/document/d/abc --label draft
tix detach <ID> draft                          # by label or by exact ref
```

Review the board:

```sh
tix board                                      # human view; for data prefer `tix ls --json`
tix board --group
```

Edit the brief (there is no body command): open `"$(tix path <ID>)/ticket.md"` from the workspace root, change only the text **below** the closing `---`, then run `tix check`.

## After someone edits tix.yaml

Reads never break: `ls`, `show` and `board` still show tickets that no longer fit, marked `!` (and `"valid": false, "problem": "..."` in JSON); unknown statuses land in the board's `?` column. Repair them:

```sh
tix check --json                               # lists every problem as {path, message}
tix set <ID> client=acme                       # missing or invalid field
tix mv <ID> backlog                            # status no longer exists
tix set <ID> status=backlog client=acme        # both at once (mv alone would fail)
tix set <ID> old_field=                        # remove a field tix.yaml no longer declares
```

`check` reports only the first broken rule per ticket, so re-run `tix check --json` after each fix until it returns `"ok": true`.

## Errors

| Exit | Meaning | What to do |
|---|---|---|
| 0 | Success | |
| 1 | Validation: bad value, invalid resulting ticket, unknown status, `check` found problems | Read the stderr message; it names the rule, e.g. `rule 3: required field 'client' is missing or empty`. Fix the value; nothing was written. |
| 2 | Usage or workspace: bad arguments, unknown filter key, id prefix not found / ambiguous / under 4 chars, no `tix.yaml`, `tix.yaml` already exists | Check `tix <command> --help`. For ambiguous ids use the full id (candidates are on stderr). |
| 3 | I/O error | Report the message to the user. |

Validity rules named in messages: 1 id is a ULID and title non-empty; 2 status exists in `tix.yaml`; 3 required fields present and non-empty; 4 every field declared and of the right type (enum value allowed, date `YYYY-MM-DD`); 5 deliverables have label and ref, refs unique.

## Don't

- Don't hand-edit frontmatter when a command does the job (`set`, `mv`, `attach`, `detach`). If you must, run `tix check` afterwards.
- Don't create ticket folders or ids yourself; use `tix new`.
- Don't copy deliverable files into the workspace; attach a reference.
- Don't run `tix init` without the user's go-ahead.
