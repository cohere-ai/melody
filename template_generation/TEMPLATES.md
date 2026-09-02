# Templates on main

`main` keeps the **current** revision of each family, plus **variants** (different
flavors of that current revision). Old revisions are git tags, not extra files.

A **version** is a replacement: v2 supersedes v1, and only the latest stays on
`main`. A **variant** is not a replacement: `cmd4-hf` and `cmd5-strict` are
different products that coexist with the default.

## What's on main

| `template_id` | Kind | Generated file |
| --- | --- | --- |
| `cmd3-v1` | variant (default Command 3; Command R / A / Vision / Translate) | `gen/templates/jinja/cmd3-v1.jinja` |
| `cmd3-v2` | variant (reasoning default-on; North Large, some C4 translate) | `gen/templates/jinja/cmd3-v2.jinja` |
| `cmd3-v3` | variant (v2 + default thinking filler; Command A Reasoning) | `gen/templates/jinja/cmd3-v3.jinja` |
| `cmd3-v1-hf` | variant (HuggingFace wrapper of cmd3-v1) | `gen/templates/jinja/cmd3-v1-hf.jinja` |
| `cmd4` | current cmd4 (v2) | `gen/templates/jinja/cmd4.jinja` |
| `cmd4-hf` | variant (HuggingFace wrapper of current cmd4) | `gen/templates/jinja/cmd4-hf.jinja` |
| `cmd5` | current cmd5 | `gen/templates/jinja/cmd5.jinja` |
| `cmd5-strict` | variant (XML-escaped flat params) | `gen/templates/jinja/cmd5-strict.jinja` |
| `cmd5-no-escape` | variant (flat params, no body escaping) | `gen/templates/jinja/cmd5-no-escape.jinja` |

Liquid artifacts follow the same names under `gen/templates/liquid/`. cmd4
liquid is the liquid engine snapshot of cmd4 (`cmd4.tmpl`); jinja `cmd4` is
the current (v2) prompt.

## Old versions

History is the git tag `<template_id>-v<N>` (a commit, so `git show` and `curl`
both work). Hyphens keep the name a single URL path segment.

```bash
git show cmd4-v1:gen/templates/jinja/cmd4-v1.jinja

curl -L -H "Authorization: Bearer $GITHUB_TOKEN" \
  https://raw.githubusercontent.com/cohere-ai/melody/cmd4-v1/gen/templates/jinja/cmd4-v1.jinja
```

Paths on a tag are whatever those files were named at that commit. For cmd4:

| Version | Tag | File on that tag |
| --- | --- | --- |
| v1 | `cmd4-v1` | `gen/templates/jinja/cmd4-v1.jinja` |
| v2 | `cmd4-v2` | `gen/templates/jinja/cmd4-v2.jinja` (same content as current `cmd4.jinja`) |

## Freeze on merge

Merges to `main` that change a generated artifact under `gen/templates/` tag
the previous `main` SHA as `<template_id>-vN` (`N` is one past the highest
existing tag for that id). New files are not frozen until they are replaced.
Each id has its own series, including wrappers (`cmd4-hf`).

This is [`.github/workflows/freeze-templates.yml`](../.github/workflows/freeze-templates.yml).
