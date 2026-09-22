---
title: Naming Conventions
discipline: code
status: active
updated: 2026-09-10
---

# Naming Conventions

> **Purpose.** Settle naming once so it's never a review topic.
> **Related.** [coding-standards.md](coding-standards.md) · [project-structure.md](project-structure.md) · [git-workflow.md](git-workflow.md)

## Files

| Kind                    | Convention            | Example                          |
| ----------------------- | --------------------- | -------------------------------- |
| Folders                 | `lowercase-kebab`     | `src-tauri/`, `docs/engineering/` |
| Rust modules            | `snake_case.rs`       | `windows_ocr.rs`, `preprocess.rs` |
| React components        | `PascalCase.tsx`      | `SelectionLayer.tsx`             |
| Other TypeScript        | `camelCase.ts`        | `fitText.ts`, `coords.ts`        |
| Generated files         | `*.gen.ts`            | `types.gen.ts` — never hand-edit |
| Tests (Rust)            | `snake_case.rs` in `tests/` | `pipeline_test.rs`         |
| Tests (TS)              | `<name>.test.ts`      | `coords.test.ts`                 |
| Fixtures                | `<case>-<detail>.png` | `small-ui-text-12px.png`         |
| Root meta               | `UPPERCASE.md`        | `README.md`, `CHANGELOG.md`      |
| Docs under `docs/`      | `lowercase-kebab.md`  | `project-structure.md`           |
| ADRs                    | `NNNN-kebab-title.md` | `0004-capture-and-coordinate-model.md` |

## Variables & Functions

**Rust** — `snake_case` for values and functions, `SCREAMING_SNAKE_CASE` for consts.
**TypeScript** — `camelCase` for values and functions, `SCREAMING_SNAKE_CASE` for module consts.

**Booleans** read as assertions: `is_primary`, `hasProviderKey`, `alignmentOk`. Not `flag`, not `check`, not `status`.

**Functions are verbs**, and the verb should say how expensive it is:

| Prefix          | Means                                    | Example                       |
| --------------- | ---------------------------------------- | ----------------------------- |
| `get_` / `get`  | Cheap accessor, no I/O                   | `get_settings`                |
| `load_` / `read_` | Touches disk or the registry           | `load_settings`               |
| `fetch_`        | Touches the network                      | `fetch_translation`           |
| `grab_` / `capture_` | Touches the screen                  | `grab_virtual_desktop`        |
| `to_` / `into_` | Pure conversion                          | `to_physical`, `into_blocks`  |
| `try_`          | Returns `Result` where a sibling doesn't | `try_register_hotkey`         |

That table exists for one reason: in this codebase a reader must be able to tell from a call site whether a function
hits the network, the disk, or the screen. Those are the three things with a latency budget.

### Units in names

Anything with a unit carries it: `elapsed_ms`, `max_chars_per_request`, `scale_factor`, `text_height_px`.

**Coordinates are the strict case.** Because two pixel spaces exist
([ADR-0004](../architecture/adr/0004-capture-and-coordinate-model.md)), any variable holding a coordinate says which
one it is:

- `rect_physical`, `bbox_physical` — physical pixels, virtual-desktop space (the core's only space)
- `rect_css`, `pos_css` — CSS pixels, inside a webview

A bare `rect` is only acceptable inside `coords.ts` and `geometry.rs`, where the space is unambiguous from context.
Everywhere else, name it. This convention is cheap and it catches the project's worst bug class at read time.

## Types

**Rust** — `PascalCase` for structs, enums, and traits. Traits are capability nouns (`OcrEngine`,
`TranslationProvider`), not `-able` adjectives and never `IFoo`. Enum variants are `PascalCase`.

**TypeScript** — `PascalCase` for interfaces and type aliases. No `I` prefix. Prefer `interface` for object shapes,
`type` for unions and aliases.

**Error types** end in `Error` (`OcrError`, `TranslateError`); the IPC-facing enum is `ErrorCode` with
`SCREAMING_SNAKE_CASE` variants matching [../architecture/api.md](../architecture/api.md) exactly. Those strings are
a contract — the TS side matches on them.

**IPC commands** are `snake_case` verbs in Rust and stay `snake_case` across the boundary (`translate_region`,
`set_provider_key`). Don't camelCase them on the TS side; the wire name is the name.

**Events** are namespaced with the app scheme: `otr://progress`, `otr://error`.

## Branches

`<type>/<short-kebab-description>`, types matching the commit types below:

```
feat/in-place-overlay
fix/dpi-offset-secondary-monitor
docs/adr-0005-live-mode
chore/bump-tauri
```

Work happens on a branch even solo — it keeps `main` releasable and makes "what was this change?" answerable.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <subject>`.

**Types:** `feat` · `fix` · `docs` · `refactor` · `perf` · `test` · `chore` · `build` · `ci`

**Scopes** are the module or area: `capture`, `ocr`, `translate`, `overlay`, `panel`, `settings`, `tray`, `hotkey`,
`ipc`, `docs`, `ci`.

```
feat(overlay): draw translated lines at their source bounding boxes
fix(capture): use virtual-desktop origin instead of primary-monitor bounds
perf(pipeline): crop from the cached frame instead of re-capturing
docs(adr): record the capture and coordinate model
```

Subject: imperative mood, lowercase, no trailing period, ≤ 72 characters. The body explains **why** — the diff
already covers what. See [git-workflow.md](git-workflow.md).

## Tags & Releases

Tags are `v<semver>`: `v0.1.0`, `v0.2.0`, `v1.0.0`. The tag triggers the release build
([../operations/ci-cd.md](../operations/ci-cd.md)); the version in `tauri.conf.json` must match it exactly.

## Settings keys

`camelCase` in the JSON file, matching the TypeScript `Settings` interface field for field
(`targetLanguage`, `resultDisplayMode`, `startWithWindows`). One name for one concept across Rust, TypeScript, and
the file on disk — a key that's renamed in one place and not the others is a migration bug waiting to happen.

Credential Manager entries: `on-screen-translator/<provider>` — one entry per provider, so switching providers and
back doesn't lose a key ([ADR-0003](../architecture/adr/0003-translation-provider-abstraction.md)).
