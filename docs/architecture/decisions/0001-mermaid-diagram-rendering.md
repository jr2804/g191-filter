---
title: 0001 — Mermaid Diagram Rendering Pipeline
---

## 0001 — Mermaid Diagram Rendering Pipeline

**Date**: 2026-08-30
**Status**: Accepted

### Context

The documentation site builds with Zensical. Zensical executes Markdown extensions
(such as `pymdownx.superfences`) through its Python bridge, but it does not run
MkDocs plugin event hooks — only `mkdocstrings` is wired up natively. The
`mermaid2` plugin therefore cannot inject the `mermaid.js` runtime: a
` ```mermaid ` fence without a custom formatter renders as a plain code block,
and even with `mermaid2.fence_mermaid` emitting a `<div class="mermaid">`, no
JavaScript reaches the page.

### Decision

Mermaid diagrams render through three parts:

1. **Fence**: `pymdownx.superfences` custom fence `name = "mermaid"` with
   `format = "mermaid2.fence_mermaid"` converts the fence body into
   `<div class="mermaid">`. The `mkdocs-mermaid2-plugin` dependency is used only
   for this formatter function.
2. **Runtime**: `mermaid.min.js` (CDN, pinned version) is loaded via
   `[[project.extra_javascript]]` in `zensical.toml`.
3. **Init**: `docs/assets/javascripts/mermaid-init.js` calls
   `mermaid.initialize({ startOnLoad: false })` with `theme: "base"` and
   light/dark `themeVariables`, then renders via `mermaid.run()` on
   DOMContentLoaded. The theme follows Material's `data-md-color-scheme`
   attribute at page load.

Authoring a diagram requires nothing beyond a standard ` ```mermaid ` code fence
in any page.

### Consequences

#### Positive

- Diagrams render on GitHub and on the site from the same Markdown source.
- Light/dark palette support without a plugin hook.

#### Negative / Trade-offs

- Do **not** register the `mermaid2` plugin under `[project.plugins]` — its
  `on_post_page` hook never fires under Zensical, and a second init path would
  double-render diagrams.
- Theme switches require a page reload; diagrams do not re-render on palette
  toggle.
- The CDN pin in `zensical.toml` must be bumped manually to upgrade Mermaid.
