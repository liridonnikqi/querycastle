# QueryCastle

Fast desktop SQL workspace — **Tauri + Svelte + Rust**.

PostgreSQL, MySQL and SQLite in one app: editor with autocomplete & formatting, schema explorer, query tabs, inline editing, saved queries & history.

![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB) ![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00) ![Rust](https://img.shields.io/badge/Rust-stable-orange)

## Features

- Cross-platform desktop (Windows / macOS / Linux)
- Connection profiles or connection string, SSL optional
- SQL editor (autocomplete, formatting, `Ctrl+Enter` to run)
- Schema explorer (schemas, tables, columns, foreign keys)
- Editable data grid (transactional apply: `ctid` / `rowid` / row hash)
- Multi-tab workspace, saved queries, per-connection history
- Auto-updates via GitHub Releases

**Databases:** PostgreSQL ✅ · SQLite ✅ · MySQL ✅ (row hash, `LIMIT 1` on duplicates)

## Quick Start

**Prereqs:** [Bun](https://bun.sh/docs/installation) · [Rust](https://www.rust-lang.org/tools/install) · [Tauri prerequisites](https://tauri.app/start/prerequisites/)

```bash
bun install
bun run dev          # app + frontend
# or
bun run dev:frontend # frontend only http://localhost:5173
```

## Scripts

| Command | Description |
|---|---|
| `bun run dev` | Run Tauri + Vite |
| `bun run build` | Build frontend to `dist/` |
| `bunx tauri build` | Build desktop bundles |
| `bun run check` | `svelte-check` + `tsc` |

## Project Structure

```
src/               SvelteKit frontend
src-tauri/         Rust backend, Tauri config
src-tauri/src/lib.rs  commands & DB adapters
```

- Query timeout `30s`, max `1000` rows, transactional edits.

## Releases

GitHub Releases via `tauri-plugin-updater` (`latest.json`). Tags `v*` trigger `.github/workflows/publish.yml` (Windows).

## Contributing

Fork → feature branch → PR.

## License

Add a `LICENSE` file before distributing.
