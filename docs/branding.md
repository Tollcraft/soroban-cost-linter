# Brand & Design Guide

GitBook does not support custom CSS via Git Sync — visual theming lives in the hosted space's **Customization** settings. This page is the spec for maintainers configuring the space, plus the in-repo conventions that keep pages consistent.

## Visual Identity

High-contrast dark mode with neon accents — built for smart contract developers who live in dark editors.

### Palette

| Role               | Color           | Hex       |
| ------------------ | --------------- | --------- |
| Primary accent     | Neon teal       | `#00F5D4` |
| Background (dark)  | Near-black blue | `#0B0E14` |
| Warning highlights | Neon amber      | `#FFB800` |
| Danger highlights  | Neon pink       | `#FF2E88` |

### GitBook space settings

Apply under **Space → Customize**:

1. **Theme:** set default appearance to **Dark**; keep the light/dark toggle enabled.
2. **Primary color:** `#00F5D4` (drives links, active nav items, and hint accents).
3. **Fonts:** a geometric sans-serif for headings (e.g. _Inter_ or _Space Grotesk_ where the plan allows), default mono for code.
4. **Logo & favicon:** upload the project mark on a transparent background so it sits on the dark canvas; a simple ⚡ glyph works as a placeholder favicon.
5. **Page layout:** enable page outlines ("On this page") for the lint reference pages.

{% hint style="info" %}
Font selection and some customization options depend on the GitBook plan. If an option is unavailable, the palette and dark-default settings above carry most of the aesthetic.
{% endhint %}

## In-Repo Conventions

These keep new pages consistent without any hosted configuration:

* **Hint blocks** carry the accent colors: `danger` for cost pitfalls ("Why is this bad?"), `success` for fixes, `warning` for operational gotchas, `info` for cross-references.
* **One emoji per H1/section header** as a visual marker (⚡ 🔍 🔌 🎨) — one, not several.
* **Code blocks** use `{% code title="..." %}` with the target filename when the reader is meant to create that file.
* **Lint pages** follow the shared template: severity → what it does → why it's bad (`danger` hint) → `❌ Bad` example → suggested fix (`success` hint).

## Enabling Git Sync (cutover)

1. Create a GitBook space and connect **Git Sync** to `Tollcraft/soroban-cost-linter`, branch `main`.
2. GitBook reads `.gitbook.yaml` at the repo root (`root: ./docs/`) and builds navigation from `docs/SUMMARY.md`.
3. Apply the customization settings above.
4. Update any published docs links (repo About field, README) to the new GitBook URL.

{% hint style="warning" %}
The previous mdBook deployment to GitHub Pages was removed in the GitBook migration — until Git Sync is enabled, no hosted docs are live.
{% endhint %}
