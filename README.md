<!-- Target languages: ["en"] -->
🌐 **Languages:** English (default) · Translations welcome via PR

# QueryCastle

<div align="center">

A fast desktop SQL workspace built with **Tauri + Svelte + Rust**.

[Quick Start](#quick-start) •
[Features](#features) •
[Supported-database](#supported-database) •
[Development](#development) •
[Contributing](#contributing)

</div>

## What Is QueryCastle?

QueryCastle is a cross-platform desktop SQL client for PostgreSQL, MySQL, and SQLite workflows.
It combines a modern SQL editor, schema exploration, query history, and inline table editing in one app.

## Highlights

- Desktop app for Windows / macOS / Linux via Tauri
- Connection profiles (fields or connection string where supported)
- SQL editor with autocomplete + formatting
- Query tabs and data tabs for parallel workflows
- Schema explorer with columns and foreign keys
- Inline row updates/inserts/deletes with transactional apply
- Saved queries and per-connection query history
- Built-in updater support via Tauri plugin

## Supported Database (Current)

| Database | Status |
| :-- | :-- |
| PostgreSQL | ✅ Full support |
| SQLite | ✅ Full support |
| MySQL | ✅ Supported (editing has some limitations, see notes below) |

## MySQL Editing Notes

- MySQL row editing is supported for View Data and editable SELECT results.
- To track row identity in editable grids, QueryCastle uses a deterministic row hash.
- If a table contains duplicate rows across all columns, update/delete operations may target one matching row (`limit 1`).

## Features

| Area | Included |
| :-- | :-- |
| Connection Management | Save, edit, test, connect, disconnect |
| SSL | Optional SSL mode |
| SQL Editor | Run, format, autocomplete |
| Results | Tabular results with duration + row count |
| Data Editing | Update, insert, delete rows from result grid |
| Explorer | Schemas, tables/views, columns, foreign keys |
| Query Workspace | Multi-tab query/data experience |
| Persistence | Local storage for connections, tabs, history, favorites |
| Updates | Configured updater endpoint in Tauri config |

## Quick Start

### 1. Prerequisites

- Bun
- Rust (stable toolchain)
- Tauri OS prerequisites

References:

- https://tauri.app/start/prerequisites/
- https://bun.sh/docs/installation
- https://www.rust-lang.org/tools/install

### 2. Install Dependencies

```bash
bun install
```

### 3. Run In Development

```bash
bun run dev
```

This starts Vite and launches the Tauri desktop shell.

## Development

### Scripts

- `bun run dev` — run full app (frontend + Tauri)
- `bun run dev:frontend` — run frontend only on `http://localhost:5173`
- `bun run build` — build frontend into `dist/`
- `bun run check` — type and Svelte checks

### Build Desktop Bundles

```bash
bun run build
bunx tauri build
```

## Project Structure

```text
src/                     Svelte frontend
src/components/          UI (editor, explorer, results, modals)
src/lib/                 RPC client + shared TS types
src-tauri/               Rust backend + Tauri config
src-tauri/src/lib.rs     Tauri commands and database operations
```

## Runtime Notes

- Query timeout: **30,000 ms**
- Max returned rows in payload: **1,000**
- Table edits are applied in a single SQL transaction
- Row mutation tracking uses:
	- PostgreSQL: `ctid`
	- SQLite: `rowid`
	- MySQL: deterministic row hash

## Contributing

Contributions are welcome.

1. Fork the repository
2. Create a feature branch
3. Make and test changes
4. Open a pull request with a clear description

## License

No `LICENSE` file is currently present in this repository.
If you plan to distribute QueryCastle, add an explicit license file.

