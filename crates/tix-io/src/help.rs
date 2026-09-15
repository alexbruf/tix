//! Long `--help` text. Written for people and AI agents driving `tix` from a
//! shell: exact argument formats, output shapes, and exit codes. `-h` stays short.

pub const ROOT_LONG: &str = "\
tix keeps a ticket board as plain files. A workspace is a folder with a schema file, tix.yaml, \
and one folder per ticket, tickets/<ID>/ticket.md (YAML frontmatter plus a Markdown brief). \
There is no server and no database. Every command except `init` finds the workspace by walking \
up from the current directory to the nearest tix.yaml.";

pub const ROOT_AFTER: &str = "\
AGENT QUICKSTART
  Read `tix <command> --help` before using a command for the first time: each one documents its
  exact argument formats, output, --json shape, errors and examples (`tix help <command>` works too).
  Pass --no-prompt so a missing required value fails (exit 1) instead of waiting on stdin.
  Pass --json when parsing output: every command prints exactly one JSON document on stdout.
  Results go to stdout; errors and warnings go to stderr.

  tix init                                     # once, in the project folder
  tix new --help                               # lists this workspace's fields and allowed values
  tix new --no-prompt --title \"Product article\" --client nova --type article
  tix ls status:backlog --json
  tix set 01K5AQ9Z owner=charlie due=2026-09-30
  tix attach 01K5AQ9Z https://docs.google.com/document/d/abc --label draft
  tix mv 01K5AQ9Z in_progress
  tix check                                    # after editing tix.yaml or a ticket.md by hand

IDS
  Ticket ids are 26-character ULIDs. Wherever a command takes ID, a unique prefix of at least
  4 characters works. Tickets created in the same second share their first 10 characters; if tix
  says a prefix is ambiguous (exit 2, candidates listed on stderr), use a longer one or the full id.

SCHEMA (tix.yaml)
  statuses: ordered list of {name, group}; group is one of backlog, in_progress, completed.
            New tickets start in the first status whose group is backlog. Any status may move
            to any other status.
  fields:   list of {name, type, required, values, default}. Types:
              string  any non-empty text
              enum    one of `values`
              date    YYYY-MM-DD, a real calendar date
              list    list of non-empty strings; comma-separated on the command line
  Built-in ticket keys (not fields): id, title, status, created, updated, deliverables.

VALIDITY
  Write commands (new, mv, set, attach, detach) never write an invalid ticket: on any error
  nothing is written. Editing tix.yaml never breaks reads: ls, show and board still display
  tickets that no longer fit, marked with `!`, and board puts unknown statuses in a `?` column.
  Error messages name the rule broken, e.g. `rule 3: required field 'client' is missing or empty`.
  Rules: 1 id is a ULID and title non-empty; 2 status exists; 3 required fields present and
  non-empty; 4 every field declared and well-typed; 5 deliverables have label and ref, refs unique.

JSON TICKET OBJECT (used by new, mv, set, attach, detach, ls, show, board)
  {\"id\": \"01K5...\", \"title\": \"...\", \"status\": \"backlog\",
   \"created\": \"2026-09-14T15:02:11Z\", \"updated\": \"2026-09-14T15:02:11Z\",
   \"deliverables\": [{\"label\": \"draft\", \"ref\": \"https://...\"}],
   \"<field>\": \"value\" or [\"a\", \"b\"], ...}
  ls, show and board add \"valid\": bool and \"problem\": string or null; show adds \"body\".

EXIT CODES
  0  success
  1  validation: invalid value or resulting ticket, unknown status, `check` found problems
  2  usage or workspace: bad arguments, unknown filter key, id prefix not found / ambiguous /
     shorter than 4, no tix.yaml found, tix.yaml already exists (init)
  3  I/O error reading or writing files

MCP
  The standalone `tix` binary also runs as a local MCP server with one tool per command:
  `tix mcp` (setup: `claude mcp add tix -- tix mcp`; see `tix mcp --help`).

FILES
  <workspace>/tix.yaml                 schema; run `tix check` after editing
  <workspace>/tickets/<ID>/ticket.md   one ticket; safe to edit by hand, then run `tix check`
  Deliverables are references (URLs, paths, ids in other tools); tix never copies or downloads them.";

pub const INIT_LONG: &str = "\
Create tix.yaml (the default schema below) and an empty tickets/ folder in the current directory.
Fails with exit 2, writing nothing, if a tix.yaml already exists here or in any parent folder.";

pub const INIT_AFTER: &str = "\
DEFAULT tix.yaml
  statuses: backlog (backlog), in_progress (in_progress), done (completed)
  fields:   client (string, required), type (enum: article, landing_page, linkedin, other,
            required), owner (string), due (date)
  Edit it freely afterwards and run `tix check`.

OUTPUT
  stdout: initialized <absolute dir>
  --json: {\"root\": \"<absolute dir>\"}

EXAMPLE
  mkdir content-board && cd content-board && tix init";

pub const CHECK_LONG: &str = "\
Validate tix.yaml and every ticket. Run this after editing tix.yaml or any ticket.md by hand.
If tix.yaml itself is invalid, only that problem is reported.";

pub const CHECK_AFTER: &str = "\
OUTPUT
  No problems: stdout `ok`, exit 0.
  Problems: one line per problem on stdout as `PATH: MESSAGE`, `N problem(s)` on stderr, exit 1.
    tix.yaml: rule 3: no status has group 'backlog'
    tickets/01K5D1E2.../ticket.md: rule 3: required field 'client' is missing or empty
    tickets/01K5E9F8.../ticket.md: rule 2: unknown status 'review'; valid statuses: backlog, ...
  Also reported: a folder without ticket.md, unparseable frontmatter, and an id that differs
  from its folder name. Only the first broken rule per ticket is shown: re-run after each fix
  until it prints `ok`.
  --json: {\"ok\": false, \"problems\": [{\"path\": \"...\", \"message\": \"...\"}]} (exit code unchanged)

FIXING PROBLEMS
  missing/invalid field  tix set ID field=value
  unknown status         tix mv ID <status>   (or both at once: tix set ID status=<s> field=value)
  undeclared field       tix set ID field=    (removes it)";

pub const NEW_LONG: &str = "\
Create one ticket and print its id. Fields are given as --<field> VALUE flags generated from
tix.yaml (listed below when run inside a workspace). A required field that is not given and has
no default is prompted for on stdin, unless --no-prompt is set, in which case the command fails
with exit 1 and writes nothing. Optional fields are never prompted. Defaults from tix.yaml apply
to fields not given.";

pub const NEW_AFTER: &str = "\
VALUES
  string: any non-empty text   enum: one of the listed values   date: YYYY-MM-DD
  list: comma-separated, e.g. --tags web,category

OUTPUT
  stdout: the new ticket id, e.g. 01M2H2KNKGDK5CF9XQN3655GHA
  --json: the ticket object (see `tix --help`)

EXAMPLES
  id=$(tix new --no-prompt --title \"Product comparison\" --client nova --type article)
  tix new --no-prompt --title \"Launch post\" --client acme --type linkedin --status in_progress --due 2026-10-01

ERRORS
  exit 1  missing required field, value not allowed, unknown --status (message lists valid ones)";

pub const LS_LONG: &str = "\
List tickets as a table: short id (first 8 chars), status, title, then every required field.
Rows are ordered by status (schema order, unknown statuses last), then created ascending.
Tickets that no longer fit tix.yaml are shown with `!` before the id.";

pub const FILTERS: &str = "\
FILTERS
  Zero or more KEY:VALUE tokens, all of which must match (AND). Split at the first `:`.
    status:<name>                  exact status, e.g. status:in_progress
    group:<group>                  backlog | in_progress | completed
    <field>:<value>                exact field value, e.g. client:nova, type:article
    <list field>:<value>           matches if any element equals value, e.g. tags:web
  An unknown KEY or a token without `:` is an error (exit 2), never a silent non-match.";

pub const LS_AFTER: &str = "\
OUTPUT
   ID        STATUS       TITLE                  client  type
   01K5AQ9Z  backlog      Product comparison     nova    article
   !01K5D1E2 backlog      Missing client                 other
  --json: array of ticket objects (see `tix --help`) with \"valid\" and \"problem\", same order.

EXAMPLES
  tix ls
  tix ls status:backlog client:nova --json
  tix ls group:completed";

pub const SHOW_LONG: &str =
    "Print one ticket: every frontmatter key, its deliverables, then the brief (body) verbatim.";

pub const SHOW_AFTER: &str = "\
OUTPUT
  ! rule 3: required field 'client' is missing or empty      (only if the ticket is invalid)
  id: 01K5AQ9Z3R7M8N2P4Q6S8T0V1W
  title: Product comparison
  status: in_progress
  created: 2026-09-14T15:02:11Z
  updated: 2026-09-15T09:40:00Z
  client: nova
  type: article
  deliverables:                                               (only if there are any)
    draft -> https://docs.google.com/document/d/abc
  <empty line, then the body exactly as in ticket.md>
  --json: ticket object plus \"body\", \"valid\", \"problem\".

EXAMPLE
  tix show 01K5AQ9Z --json";

pub const MV_LONG: &str = "\
Set a ticket's status, changing nothing else except `updated`. Any status may move to any other.";

pub const MV_AFTER: &str = "\
OUTPUT
  stdout: the full ticket id    --json: the updated ticket object

EXAMPLE
  tix mv 01K5AQ9Z done

ERRORS
  exit 1  status not in tix.yaml (message lists valid statuses), or the ticket would still be
          invalid (e.g. a required field is missing: use `tix set ID status=<s> field=value`)";

pub const SET_LONG: &str = "\
Update the title, status or fields of a ticket in one write. All assignments apply together: if
the resulting ticket is invalid, nothing is written. Fields not named are left unchanged.";

pub const SET_AFTER: &str = "\
ASSIGNMENTS (KEY=VALUE, split at the first `=`)
  title=<text>        new title (must be non-empty)
  status=<name>       new status; useful to repair a ticket together with its fields
  <field>=<value>     set a schema field; list fields take comma-separated values
  <field>=            remove the field (an error if the field is required)
  Each KEY at most once. id, created, updated and deliverables cannot be set (exit 2);
  use attach/detach for deliverables.

OUTPUT
  stdout: the full ticket id    --json: the updated ticket object

EXAMPLES
  tix set 01K5AQ9Z owner=charlie due=2026-09-30
  tix set 01K5AQ9Z \"title=Product vs alternate\" owner=
  tix set 01K5E9F8 status=backlog client=acme";

pub const ATTACH_LONG: &str = "\
Append a deliverable reference to a ticket. REF is stored verbatim and never copied, downloaded
or checked: a URL, a Drive link, an absolute path, an id in another tool, or a path starting with
./ which is understood as relative to the ticket folder.";

pub const ATTACH_AFTER: &str = "\
LABEL
  --label L sets the label. Without it the label is the last non-empty /-segment of REF
  (https://docs.google.com/document/d/abc -> abc, ./outline.md -> outline.md).

OUTPUT
  stdout: the full ticket id    --json: the updated ticket object

EXAMPLES
  tix attach 01K5AQ9Z https://docs.google.com/document/d/abc --label draft
  tix attach 01K5AQ9Z ./outline.md

ERRORS
  exit 1  REF already attached to this ticket (refs are unique per ticket)";

pub const DETACH_LONG: &str = "\
Remove the single deliverable whose ref or label equals REF_OR_LABEL.";

pub const DETACH_AFTER: &str = "\
OUTPUT
  stdout: the full ticket id    --json: the updated ticket object

EXAMPLES
  tix detach 01K5AQ9Z draft
  tix detach 01K5AQ9Z https://docs.google.com/document/d/abc

ERRORS
  exit 1  nothing matches, or more than one deliverable matches (use the ref instead of a label)";

pub const BOARD_LONG: &str = "\
Show the board: one column per status in schema order, headed `STATUS (n)`, each cell
`<short id> <title>`, oldest first. A trailing `? (n)` column appears when tickets have a status
that is not in tix.yaml. --group collapses the columns into backlog, in_progress and completed.";

pub const BOARD_AFTER: &str = "\
OUTPUT
  --json: {\"columns\": [{\"name\": \"backlog\", \"tickets\": [ticket objects with valid, problem]}, ...]}
  The `?` entry is present only when non-empty. For reading tickets, `tix ls --json` is simpler.

EXAMPLES
  tix board
  tix board --group client:nova";

pub const PATH_LONG: &str = "\
Print the ticket folder path relative to the workspace root (the folder holding tix.yaml), e.g.
tickets/01K5AQ9Z3R7M8N2P4Q6S8T0V1W. tix never opens editors; read or edit ticket.md there directly.";

pub const PATH_AFTER: &str = "\
OUTPUT
  stdout: tickets/<ID>    --json: {\"path\": \"tickets/<ID>\"}

EXAMPLES
  cat \"$(tix path 01K5AQ9Z)/ticket.md\"        # from the workspace root
  $EDITOR \"$(tix path 01K5AQ9Z)/ticket.md\"";
