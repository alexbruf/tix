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

## Environment

`NPM_TOKEN` is only used by CI to publish on tags. See `.env.example`.

## Status

Pre-implementation.
