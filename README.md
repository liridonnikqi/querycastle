# QueryCastle

Fast desktop SQL workspace — **Tauri + Svelte + Rust**.

PostgreSQL, MySQL and SQLite in one app: editor with autocomplete & formatting, schema explorer, query tabs, inline editing, saved queries & history.

![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB) ![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00) ![Rust](https://img.shields.io/badge/Rust-stable-orange)

## Features

- Windows desktop (cross-platform via Tauri — macOS/Linux builds planned, currently Windows-only)
- Connection profiles or connection string, SSL optional
- SQL editor (autocomplete, formatting, `Ctrl+Enter` to run)
- Schema explorer (schemas, tables, columns, foreign keys)
- Editable data grid (transactional apply: `ctid` / `rowid` / row hash)
- Multi-tab workspace, saved queries, per-connection history
- Auto-updates via GitHub Releases

**Databases:** PostgreSQL ✅ · SQLite ✅ · MySQL ✅ (row hash, `LIMIT 1` on duplicates)

## Quick Start

**Prereqs:** [Node.js](https://nodejs.org/) (npm) · [Rust](https://www.rust-lang.org/tools/install) · [Tauri prerequisites](https://tauri.app/start/prerequisites/)

```bash
npm install
npm run dev          # app + frontend
# or
npm run dev:frontend # frontend only http://localhost:5173
```

## Scripts

| Command | Description |
|---|---|
| `npm run dev` | Run Tauri + Vite |
| `npm run build` | Build frontend to `dist/` |
| `npx tauri build` | Build desktop bundles |
| `npm run check` | `svelte-check` + `tsc` |

## Project Structure

```
src/               SvelteKit frontend
src-tauri/         Rust backend, Tauri config
src-tauri/src/lib.rs  commands & DB adapters
```

- Query timeout `30s`, max `1000` rows, transactional edits.

## Releases

GitHub Releases via `tauri-plugin-updater` (`latest.json`). Tags `v*` trigger `.github/workflows/publish.yml` (Windows-only; macOS/Linux artifacts planned).

## Contributing

Fork → feature branch → PR.

## License

MIT — see [LICENSE](LICENSE).
