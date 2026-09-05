# Regression scenarios — P0 session model + fake-engine deletion

Give this file to a follow-up agent. The job is to **catch regressions** from the P0 change on `refactoring`, not to refactor further.

## What shipped (so you know what must still work)

1. **Required `session_id` on every DB command.** No fallback to “active connection”. Missing/empty/unknown id must fail.
2. **Frontend always passes `sessionId`.** `ResultsPane` must not call Tauri itself. FK lookup and COUNT go through `workspace.runSessionQuery`.
3. **`rpc.*` is flat.** There is no `rpc.request`.
4. **Fake engines are gone.** Connection UI only offers Postgres, MySQL, SQLite.
5. **`src-tauri/main` (stray SQLite blob) is deleted** and gitignored.
6. **`disconnect` returns `()`.** `switch_session` must not panic.

Do **not** implement `ctid` → `rowId`, `From` error impls, or a ResultsPane split while running this. Report only.

---

## How to report

Fill the checklist at the bottom. For every failure: file path, what you did, expected vs actual. Quote the error. Do not “fix while testing” unless the user asked.

Windows / PowerShell: chain with `;`, not `&&`.

---

## Gate A — always runnable (no database)

Must all pass. Fail the whole run if any fail.

### A1. Unit tests

```
npm test
```

Expect: Vitest, all files green (currently 13 files / 77 tests).

```
Set-Location src-tauri; cargo test --offline
```

Expect: 9 tests ok (`apply_select_row_cap` + connection normalize + `ssl_insecure`).

### A2. Typecheck

```
npx svelte-check --tsconfig ./tsconfig.json
npx tsc --noEmit
```

Expect: 0 errors.

### A3. Contract greps (the P0 holes)

Run these from repo root. **Fail if any match.**

| Check | Command | Why |
|---|---|---|
| No nested RPC | search `rpc.request` in `src/` | Flattened to `rpc.*` |
| ResultsPane does not invoke Tauri | search `rpc-client` / `from '$lib/rpc-client'` in `ResultsPane.svelte` | Grid COUNT/FK used to hit the **active** pool |
| No fake engines | search `duckdb\|mongodb\|mssql\|redis` in `src/` (ignore-case) | Marketing chrome deleted |
| No fake icon files | `DuckdbIcon.svelte`, `MongodbIcon.svelte`, `MssqlIcon.svelte`, `RedisIcon.svelte` must not exist | |
| No stray DB blob | `src-tauri/main` must not exist as a file (keep `src-tauri/src/main.rs`) | Accidental 4KB SQLite |
| No silent session fallback | search `require_active` in `src-tauri/` | Deleted; commands use `require_session` |
| `run_query` has no Option session | `QueryParams.session_id` is `String`, not `Option<String>` | |

**Must match (fail if missing):**

- `src/lib/rpc-client.ts`: `runQuery` params include `sessionId: string` (not optional).
- `getDatabaseExplorer(sessionId: string)`, `listDatabases(sessionId: string)`.
- `selectDatabase` / `applyTableChanges` / `getObjectDefinition` include `sessionId`.
- `ResultsPane.svelte` has a required `runQuery: (sql: string) => Promise<QueryResultPayload>` prop.
- `SqlWorkspace.svelte` both `<ResultsPane>` instances pass `runQuery={(sql) => workspace.runSessionQuery(sql)}`.
- `controller.svelte.ts` `loadExplorer` / `loadDatabases` / `applyTableChanges` / `selectDatabase` / `openObjectDefinition` pass `this.activeSessionId`.
- `.gitignore` contains `src-tauri/main`, `*.db`, `*.sqlite`, `*.sqlite3`.

### A4. Engine picker is honest

Open `src/lib/components/connection/ConnectionModal.svelte`.

- `engineOptions` has **exactly three** entries: postgres, mysql, sqlite.
- No `disabled`, no “Soon”, no “coming soon”.
- `DatabaseIcon.svelte` `type` is `DatabaseType` (`postgres \| mysql \| sqlite` only). Fallback icon is Postgres, not DuckDB.

---

## Gate B — one live connection (needs `tauri dev`)

Start the app:

```
npm run dev
```

Use a real engine you have (Postgres preferred; SQLite file is enough for B1–B6).

### B1. Connect

1. New connection → only three engine tiles.
2. Fill host/user/db (or SQLite path) → Test → Connect.
3. Explorer loads tables. Status bar shows engine + database name.
4. Fail: modal still shows Redis/Mongo/DuckDB/SQL Server, or connect succeeds but explorer stays empty with no error.

### B2. Run a query

1. `select 1 as n;` (or `select 1 as n` on SQLite) → Run (Ctrl+Enter).
2. Grid shows one row. Duration > 0.
3. Fail: error “Session id is required” or “Connection session not found”.

### B3. Browse a table (data tab)

1. Explorer → a real table → View data.
2. Grid shows rows. Paging / page size still work.
3. Status / count at the bottom is a number (COUNT goes through `runSessionQuery`).
4. Fail: count stays 0 while rows are visible, or COUNT errors in logs.

### B4. FK dropdown (if the table has a single-column FK)

1. Open an editable table that has an FK column.
2. Click the FK cell editor so it loads options.
3. Options come from the **same** database (ids exist in the referenced table).
4. Fail: empty list on a populated parent table, or SQL error from the other engine.

### B5. Grid edit + apply

1. Edit one cell on a table with a primary key / row id (`_querycastle_ctid` still exists — **do not rename in this run**).
2. Apply pending changes.
3. Cell keeps the new value after refresh.
4. Fail: apply hits “Session id is required”, or the row reverts, or the wrong table is updated.

### B6. Object definition + database switch

1. Right-click a function/view/table → Open definition (or equivalent). A SQL tab opens with DDL.
2. If the server has multiple databases (Postgres/MySQL): switch database in the explorer dropdown. Explorer reloads for the **new** database. Old tables disappear.
3. Fail: definition is empty, or switch still shows the previous catalog.

### B7. Disconnect

1. Disconnect the session (tab close or disconnect all).
2. Workspace returns to the hub / empty state. No panic, no `{ok: true}` leaking into UI.
3. Reconnect works.

---

## Gate C — two sessions at once (the actual bug)

This is the regression that P0 exists to prevent. **You need two different databases** (two Postgres DBs, or Postgres + SQLite, or two SQLite files). Same engine, different catalogs is fine.

Call them **A** and **B**. They must have distinguishable data, e.g.:

- A: table `users` with a row `email = 'a@example.com'`
- B: table `users` with a row `email = 'b@example.com'`  
  or B has no `users` table at all.

### C1. Queries stay on the session that ran them

1. Connect A. Run `select current_database();` (Postgres) / `select database();` (MySQL) / `pragma database_list;` (SQLite). Note the result.
2. Connect B **without** closing A. Two connection tabs.
3. Click A’s tab. Re-run the same SQL. Result is still A.
4. Click B’s tab. Run the same SQL. Result is B.
5. Fail: both tabs show the same catalog, or A’s editor run hits B’s pool.

### C2. Explorer follows the focused session

1. Focus A → explorer lists A’s tables.
2. Focus B → explorer lists B’s tables (A-only tables gone).
3. Fail: explorer frozen on the first connection.

### C3. ResultsPane COUNT / FK / apply do not follow the other tab

This was the hole: `ResultsPane` called `runQuery({ sql })` with no `sessionId`, so it used whatever was active.

1. Focus A. Open `users` (or any A table) as a data tab. Wait until rows and the total-count are visible.
2. **Without closing that mental picture**, switch the connection tab to B.
3. Switch **back** to A.
4. Change page size or type in a column filter so COUNT re-runs. Count still matches A’s table, not B’s.
5. If A’s table has an FK: open the FK editor. Options are A’s parent rows, not B’s.
6. Optional: edit a cell on A, switch to B and back, Apply. The write lands in **A**. Check with a query on B that B’s table is unchanged.
7. Fail: count/FK/apply error “no such table”, or B’s data appears in A’s grid, or A’s edit writes into B.

### C4. Stale in-flight query after switch

1. On A, run a slow query if you can (`select pg_sleep(2);` on Postgres) or a large `select *`.
2. While it is running, switch to B.
3. A’s result must **not** paint into B’s tab. B remains B’s last result or empty.
4. Fail: B’s grid fills with A’s rows after the slow query returns.

(`executeQuery` already guards `queryEpoch` + `sessionId`; this confirms it still does.)

### C5. Close one session, the other survives

1. Both A and B connected.
2. Close B’s connection tab.
3. A still connected, explorer is A, queries on A still work.
4. Close A. Back to hub.

---

## Gate D — engine smoke (if you have the engine)

Run B2 + B3 on each engine you can. Do not skip an engine you have credentials for.

| Engine | Minimum |
|---|---|
| Postgres | `select 1`, view a table, one cell edit |
| MySQL | same |
| SQLite | open a file, `select 1`, view a table |

Row identity is still called `ctid` / `_querycastle_ctid` on the wire. That is **not** a failure of this P0.

---

## What is *not* a regression

- `ctid` in API / hidden column names (rename is a later PR).
- MySQL/SQLite still client-truncate at 1000 rows while Postgres also injects `LIMIT` (row-cap unification is later).
- `ExplorerSidebar.svelte` still ~1k lines of markup.
- `rpc.ts` types still duplicated with Rust types.

## What *is* a regression (blockers)

- Any DB invoke without `sessionId` from the frontend.
- `run_query` / explorer / apply succeeding when `sessionId` is omitted (backend must reject).
- Fake engine tiles or icons returning.
- `src-tauri/main` reappearing as a SQLite file.
- Two-session tests C1–C3 mixing catalogs.
- `npm test` / `cargo test` / `svelte-check` red.

---

## Optional: prove the backend rejects a missing session

If you can invoke Tauri commands directly (or add a throwaway test):

- `run_query` with `{ sql: "select 1", sessionId: "" }` → validation error.
- `run_query` with a random uuid → `Connection session not found` / not_found.
- `get_database_explorer` with no params object → deserialize / missing field error.

Do not leave throwaway tests in the tree.

---

## Checklist (copy into your report)

```
Gate A
- [ ] A1 npm test
- [ ] A1 cargo test
- [ ] A2 svelte-check + tsc
- [ ] A3 contract greps
- [ ] A4 engine picker

Gate B (engine: ________)
- [ ] B1 connect, three tiles only
- [ ] B2 select 1
- [ ] B3 view data + count
- [ ] B4 FK options (or N/A: no FK)
- [ ] B5 apply cell edit
- [ ] B6 object def / switch database
- [ ] B7 disconnect

Gate C (A = ________, B = ________)
- [ ] C1 queries stay on session
- [ ] C2 explorer follows focus
- [ ] C3 COUNT/FK/apply stay on A
- [ ] C4 in-flight query does not paint on B
- [ ] C5 close one session

Gate D
- [ ] Postgres smoke (or N/A)
- [ ] MySQL smoke (or N/A)
- [ ] SQLite smoke (or N/A)

Blockers found: (none / list)
```
