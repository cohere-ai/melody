# Template archive

Immutable compiled templates live in this repo at:

```text
gen/templates/archive/{name}/{name}@{revision}.jinja
```

`make generate-dev` writes the **current** revision only and never deletes older
`@N` files. Melody embeds pinned `@N` files via `include_str!`.

Floating pointer (git symlink, updated on each bump):

```text
gen/templates/archive/{name}/latest.jinja  →  {name}@{revision}.jinja
```

Changing content for an existing revision fails (see
`gen/template_revision_locks.json`); bump `revision` in
`template_registry.yaml` instead.

## CURL

Latest of a template:

```bash
curl -fsSL \
  "https://raw.githubusercontent.com/cohere-ai/melody/main/gen/templates/archive/cmd4-reasoning/latest.jinja"
```

Pin a revision (preferred for releases):

```bash
curl -fsSL \
  "https://raw.githubusercontent.com/cohere-ai/melody/refs/tags/vX.Y.Z/gen/templates/archive/cmd4-reasoning/cmd4-reasoning@1.jinja"
```
