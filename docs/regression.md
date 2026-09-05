# Regression scenarios — full post-refactor pass

Give this file to a follow-up agent. Job is to **catch regressions**, not to refactor further.

Covers everything on `refactoring` after the session-model P0 **and** the follow-up (rowId, Rust `From` impls, unified row-cap, ResultsGridSession / ConnectionSessions).

Do **not** start a rewrite while testing. Report only.

Windows / PowerShell: chain with `;`, not `&&`.

---

## What must still be true

1. Every DB command takes a required `sessionId`. No active-connection fallback.
2. `ResultsPane` does not import `$lib/rpc-client`. COUNT / FK go through `runQuery` → `workspace.runSessionQuery`.
3. Wire identity is `rowId` / `_querycastle_row_id`. Postgres **SQL** still uses the `ctid` system column (`ctid::text as _querycastle_row_id`, `where t.ctid = $n::text::tid`).
4. Query results include `truncated`. UI shows a `+` on the row count and “(capped)” on the messages tab when the 1000-row cap hit.
5. Connection picker is only Postgres / MySQL / SQLite.
6. `src-tauri/main` (stray SQLite file) does not exist. `src-tauri/src/main.rs` does.

---

## Gate A — always runnable (no database)

Fail the whole run if any of these fail.

### A1. Tests

```
npm test
```

Expect: 13 files / 77 tests green.

```
Set-Location src-tauri; cargo test --offline
```

Expect: 9 tests green (`core::limits` row-cap + connection normalize + ssl_insecure).

### A2. Typecheck

```
npx svelte-check --tsconfig ./tsconfig.json
npx tsc --noEmit
```

Expect: 0 errors.

### A3. Contract greps (fail if a match appears)

| Check | Search in | Fail if |
|---|---|---|
| Nested RPC | `src/` | `rpc.request` |
| ResultsPane talks to Tauri | `ResultsPane.svelte` | `rpc-client` |
| Hidden column old name | `src/` + `src-tauri/src/` | `_querycastle_ctid` |
| API old names | `src/` + `src-tauri/src/` | `oldCtid`, `newCtid`, `UpdatedRowCtid` |
| Fake engines | `src/` | `duckdb`, `mongodb`, `mssql`, `redis` (ignore-case) |
| Session fallback | `src-tauri/` | `require_active` |
| Silent MySQL timeout | `src-tauri/src/adapters/mysql.rs` | `let _ = conn` around `max_execution_time` (must `tracing::warn`) |

### A4. Must still exist

- `HIDDEN_ROW_ID_COLUMN === '_querycastle_row_id'` in `src/lib/utils/dialect.ts`
- `sql::HIDDEN_ROW_ID_COLUMN` in Rust
- Postgres generated SQL still contains `ctid::text as _querycastle_row_id` and `t.ctid =`
- `rpc.runQuery({ sql, sessionId })` with `sessionId: string` (not optional)
- `QueryResultPayload.truncated: boolean` in `src/lib/rpc.ts` and Rust `truncated: bool`
- `src/lib/workspace/results-grid.svelte.ts` (`ResultsGridSession`)
- `src/lib/workspace/connection-sessions.svelte.ts` (`ConnectionSessions`)
- `Workspace` composes `readonly sessions = new ConnectionSessions()`
- `ResultsPane` has `const grid = new ResultsGridSession()`
- `.gitignore` has `src-tauri/main`, `*.db`, `*.sqlite`, `*.sqlite3`

### A5. Generated Postgres SQL vs API

Open `src/lib/utils/table-select.ts` and `src-tauri/src/adapters/postgres.rs` apply-updates.

- Select builder: `select ctid::text as ${HIDDEN_ROW_ID_COLUMN}, ...`
- Update: `where t.ctid = $n::text::tid returning t.ctid::text as _querycastle_row_id`
- Payload field is `row_id` / `rowId`, **not** `ctid`

If someone renamed the Postgres system column in SQL, that is a **blocker**.

---

## Gate B — one live connection

```
npm run dev
```

Use any real engine (SQLite file is enough for B1–B6).

### B1. Engine picker

New connection → exactly three tiles (PostgreSQL, MySQL, SQLite). No Soon / coming soon.

### B2. Connect + query

`select 1 as n;` runs. Grid shows one row. No “Session id is required”.

### B3. View data

Explorer → a table → View data. Rows appear. Bottom count is a number. Paging works.

### B4. Hidden column

Grid does **not** show a `_querycastle_row_id` column. Editing still works (row identity is hidden).

### B5. Cell edit + apply

Edit one cell on a PK/rowid table. Apply. Value sticks after refresh. Fail: apply error about `ctid` JSON field, or “Session id is required”.

### B6. FK dropdown (if the table has a single-column FK)

Options load from the **same** database.

### B7. Row cap (need a table with > 1000 rows, or `select generate_series(1, 5000)`)

- Postgres: `select generate_series(1, 5000) as n;`
- Result grid shows 1000 rows.
- Toolbar count shows a `+` (truncated).
- Messages tab says “(capped)”.
- Fail: app hangs, or shows 5000 rows, or `rowCount` is 1001.

### B8. Disconnect

Disconnect. Hub returns. Reconnect works. No `{ok:true}` in the UI.

---

## Gate C — two sessions (the original hole)

Two different databases **A** and **B** with distinguishable data.

### C1. Queries stay on the session

Connect A and B. Focus A, `select current_database()` / `pragma database_list`. Switch to B, same SQL. Results differ.

### C2. Explorer follows focus

A-only tables disappear when B is focused.

### C3. COUNT / FK / apply stay on A

1. Focus A, open a data tab, wait for count.
2. Switch to B, switch back to A.
3. Change page size so COUNT re-runs. Count is still A’s table.
4. FK options (if any) are A’s parents.
5. Optional: edit on A, Apply. B is unchanged.

Fail: “no such table”, B’s rows in A’s grid, or A’s write landed in B.

### C4. In-flight query does not paint on B

On A, `select pg_sleep(2);` (Postgres) or a large select. Switch to B while it runs. B’s grid does not fill with A’s rows.

### C5. Close one session

Close B. A still works. Close A. Hub.

---

## Gate D — row identity per engine

| Engine | What to check |
|---|---|
| Postgres | View data SQL contains `ctid::text as _querycastle_row_id`. Edit+apply uses `where t.ctid = …::tid` on the server (no FE error). |
| MySQL | View data uses `md5(concat_ws…)` as `_querycastle_row_id`. Apply still works on a table **with a PRIMARY KEY**. |
| SQLite | View data uses `cast(rowid as text) as _querycastle_row_id`. Apply works. WITHOUT ROWID table should error clearly, not panic. |

---

## Gate E — structure smoke (open the files)

Not a runtime test. Skim:

- `controller.svelte.ts`: session fields are getters over `this.sessions`, not a second `$state` copy of `openSessions` / `activeSessionId`.
- `ResultsPane.svelte`: pending maps / selection / FK cache live on `grid`, not a pile of sibling `$state` next to them.
- Opening `adapters/postgres.rs` `run_query`: `?` on driver errors, no `.map_err(sanitize_pg_error_to_db_error)?` spam.
- `core/state.rs`: `HashMap<String, Arc<ActiveConnection>>`.

If those were reverted, say so. Do not “improve” them in this run.

---

## Not a regression

- `ExplorerSidebar.svelte` still ~1k lines of markup.
- `ResultsPane.svelte` is still a large view (state moved; markup did not).
- `ctid` in **Postgres SQL strings**.
- Duplicate FE/Rust types (no codegen yet).
- SQLite timeout still does not interrupt rusqlite (documented; interrupt is later).

## Blockers

- Any DB invoke without `sessionId`.
- `_querycastle_ctid` or `oldCtid` anywhere in source.
- Postgres SQL that stopped using the `ctid` system column.
- Two-session mix-up (Gate C).
- Uncapped 5000-row `generate_series`.
- `npm test` / `cargo test` / `svelte-check` red.
- Fake engine tiles.

---

## Checklist

```
Gate A
- [ ] A1 npm test
- [ ] A1 cargo test
- [ ] A2 svelte-check + tsc
- [ ] A3 greps
- [ ] A4 must-exist
- [ ] A5 PG ctid SQL vs rowId API

Gate B (engine: ________)
- [ ] B1 three tiles
- [ ] B2 select 1
- [ ] B3 view data + count
- [ ] B4 hidden column not visible
- [ ] B5 apply edit
- [ ] B6 FK (or N/A)
- [ ] B7 row cap + truncated UI
- [ ] B8 disconnect

Gate C (A = ________, B = ________)
- [ ] C1 queries stay on session
- [ ] C2 explorer follows focus
- [ ] C3 COUNT/FK/apply stay on A
- [ ] C4 in-flight does not paint on B
- [ ] C5 close one session

Gate D
- [ ] Postgres row identity
- [ ] MySQL row identity (or N/A)
- [ ] SQLite row identity (or N/A)

Gate E
- [ ] ConnectionSessions / ResultsGridSession still composed, not inlined back

Blockers: (none / list)
```
