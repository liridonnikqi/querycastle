<script lang="ts">
	import { Database, KeyRound, Play, Timer, Trash2 } from "@lucide/svelte";
	import type { ApplyTableChangesResult, QueryResultPayload, TableChangesPayload } from "../lib/rpc";

	type RowContextMenu = { x: number; y: number; rowId: string } | null;
	type EditingCell = { rowId: string; column: string } | null;
	type PendingInsertRow = { id: string; values: Record<string, unknown> };
	type ResultView = "results" | "messages" | "explain";

	let {
		result,
		sqlError,
		resultContext,
		onRunSql,
		onApplyTableChanges,
		durationMs = 0,
		loading = false,
		refreshSql = "",
	}: {
		result: QueryResultPayload;
		sqlError: string;
		resultContext: { schema: string; table: string } | null;
		onRunSql: (sql: string) => Promise<void>;
		onApplyTableChanges?: (
			context: { schema: string; table: string },
			changes: TableChangesPayload,
		) => Promise<ApplyTableChangesResult>;
		durationMs?: number;
		loading?: boolean;
		refreshSql?: string;
	} = $props();

	let displayResult = $state<QueryResultPayload>({
		columns: [],
		rows: [],
		rowCount: 0,
		durationMs: 0,
	});

	let rerunning = $state(false);
	let runningExplain = $state(false);
	let syncingChanges = $state(false);
	let syncError = $state("");
	let selectedRows = $state(new Set<string>());
	let pendingUpdates = $state(new Map<string, Record<string, unknown>>());
	let pendingDeletes = $state(new Set<string>());
	let pendingInserts = $state<PendingInsertRow[]>([]);
	let rowContextMenu = $state<RowContextMenu>(null);
	let editingCell = $state<EditingCell>(null);
	let editDraft = $state("");
	let addRowDraft = $state<Record<string, string>>({});
	let addingNewRow = $state(false);
	let highlightCells = $state(new Set<string>());
	let highlightTimeouts = new Map<string, ReturnType<typeof setTimeout>>();
	let lastExternalResultSignature = $state("");
	let activeView = $state<ResultView>("results");

	let editable = $derived.by(
		() => !!resultContext && displayResult.columns.includes("_querycastle_ctid"),
	);
	let visibleColumns = $derived.by(() => displayResult.columns.filter((column) => column !== "_querycastle_ctid"));
	let editableColumns = $derived.by(() => displayResult.columns.filter((column) => column !== "_querycastle_ctid"));
	let visibleRows = $derived.by(() =>
		displayResult.rows.filter((row) => !pendingDeletes.has(String(row["_querycastle_ctid"] ?? ""))),
	);
	const skeletonRowCount = 10;
	const skeletonRows = Array.from({ length: skeletonRowCount }, (_, index) => index);
	let hasPendingChanges = $derived.by(
		() =>
			pendingUpdates.size > 0 || pendingDeletes.size > 0 || pendingInserts.length > 0,
	);

	function quoteIdent(value: string) {
		return `"${value.replaceAll('"', '""')}"`;
	}

	function buildExternalResultSignature(
		context: { schema: string; table: string } | null,
		payload: QueryResultPayload,
	): string {
		return JSON.stringify({
			context: context ? `${context.schema}.${context.table}` : "",
			columns: payload.columns,
			rowCount: payload.rowCount,
			rows: payload.rows,
		});
	}

	function resetDraftState() {
		selectedRows = new Set();
		pendingUpdates = new Map();
		pendingDeletes = new Set();
		pendingInserts = [];
		rowContextMenu = null;
		editingCell = null;
		editDraft = "";
		syncError = "";
		const nextDraft: Record<string, string> = {};
		for (const column of editableColumns) nextDraft[column] = "";
		addRowDraft = nextDraft;
		addingNewRow = false;
	}

	function buildDefaultContextSql() {
		if (!resultContext) return "";
		const firstVisibleColumn = visibleColumns[0];
		const orderByClause = firstVisibleColumn
			? ` order by ${quoteIdent(firstVisibleColumn)} asc nulls last`
			: "";
		return `select ctid::text as _querycastle_ctid, * from ${quoteIdent(resultContext.schema)}.${quoteIdent(resultContext.table)}${orderByClause} limit 100;`;
	}

	$effect(() => {
		const signature = buildExternalResultSignature(resultContext, result);
		if (signature === lastExternalResultSignature) return;
		lastExternalResultSignature = signature;
		displayResult = {
			columns: [...result.columns],
			rows: result.rows.map((row) => ({ ...row })),
			rowCount: result.rowCount,
			durationMs: result.durationMs,
		};
		clearCellHighlights();
		resetDraftState();
		activeView = "results";
	});

	function coerceValue(raw: string, sample: unknown): unknown {
		if (raw.trim() === "") return null;
		if (typeof sample === "number") {
			const num = Number(raw);
			return Number.isNaN(num) ? raw : num;
		}
		if (typeof sample === "boolean") {
			return ["1", "true", "yes", "on"].includes(raw.trim().toLowerCase());
		}
		return raw;
	}

	function sampleValue(column: string): unknown {
		const firstRow = displayResult.rows.find((row) => row[column] !== undefined);
		return firstRow ? firstRow[column] : null;
	}

	function displayValue(value: unknown): string {
		if (value === null || value === undefined) return "";
		return String(value);
	}

	function valuesEqual(a: unknown, b: unknown): boolean {
		if (a === b) return true;
		if (a == null && b == null) return true;
		return false;
	}

	function getRowValue(row: Record<string, unknown>, rowId: string, column: string): unknown {
		return pendingUpdates.get(rowId)?.[column] ?? row[column];
	}

	function cellKey(rowId: string, column: string): string {
		return `${rowId}::${column}`;
	}

	function markCellsRecentlyUpdated(cells: Array<{ rowId: string; column: string }>) {
		const ttlMs = 2400;
		const next = new Set(highlightCells);
		for (const cell of cells) {
			const key = cellKey(cell.rowId, cell.column);
			const existing = highlightTimeouts.get(key);
			if (existing) clearTimeout(existing);
			next.add(key);
			const timeout = setTimeout(() => {
				const updated = new Set(highlightCells);
				updated.delete(key);
				highlightCells = updated;
				highlightTimeouts.delete(key);
			}, ttlMs);
			highlightTimeouts.set(key, timeout);
		}
		highlightCells = next;
	}

	function clearCellHighlights() {
		for (const timeout of highlightTimeouts.values()) clearTimeout(timeout);
		highlightTimeouts.clear();
		highlightCells = new Set();
	}

	function beginEdit(rowId: string, column: string, currentValue: unknown) {
		if (!editable || column === "_querycastle_ctid") return;
		editingCell = { rowId, column };
		editDraft = displayValue(currentValue);
	}

	function commitEdit() {
		if (!editingCell) return;
		const { rowId, column } = editingCell;
		const nextValue = coerceValue(editDraft, sampleValue(column));
		const map = new Map(pendingUpdates);
		const row = displayResult.rows.find((item) => String(item["_querycastle_ctid"] ?? "") === rowId);
		const baseValue = row ? row[column] : undefined;
		const prev = { ...(pendingUpdates.get(rowId) ?? {}) };

		if (valuesEqual(nextValue, baseValue)) {
			delete prev[column];
		} else {
			prev[column] = nextValue;
		}

		if (Object.keys(prev).length === 0) {
			map.delete(rowId);
		} else {
			map.set(rowId, prev);
		}
		pendingUpdates = map;

		editingCell = null;
		editDraft = "";
	}

	function discardEdit() {
		editingCell = null;
		editDraft = "";
	}

	function toggleRowSelected(rowId: string) {
		const next = new Set(selectedRows);
		if (next.has(rowId)) next.delete(rowId);
		else next.add(rowId);
		selectedRows = next;
	}

	function toggleSelectAllVisible() {
		const ids = visibleRows
			.map((row) => String(row["_querycastle_ctid"] ?? ""))
			.filter((id) => id.length > 0);
		const allSelected = ids.length > 0 && ids.every((id) => selectedRows.has(id));
		selectedRows = allSelected ? new Set() : new Set(ids);
	}

	function queueDeleteRows(rowIds: string[]) {
		const nextDeletes = new Set(pendingDeletes);
		const nextUpdates = new Map(pendingUpdates);
		const nextSelected = new Set(selectedRows);
		for (const rowId of rowIds) {
			nextDeletes.add(rowId);
			nextUpdates.delete(rowId);
			nextSelected.delete(rowId);
		}
		pendingDeletes = nextDeletes;
		pendingUpdates = nextUpdates;
		selectedRows = nextSelected;
		rowContextMenu = null;
	}

	function openRowContextMenu(event: MouseEvent, rowId: string) {
		if (!editable) return;
		event.preventDefault();
		rowContextMenu = { x: event.clientX, y: event.clientY, rowId };
	}

	function submitAddRow() {
		if (editableColumns.length === 0) return;
		const hasAnyValue = editableColumns.some(
			(column) => (addRowDraft[column] ?? "").trim().length > 0,
		);
		if (!hasAnyValue) return;
		const values: Record<string, unknown> = {};
		for (const column of editableColumns) {
			values[column] = coerceValue(addRowDraft[column] ?? "", sampleValue(column));
		}
		pendingInserts = [...pendingInserts, { id: crypto.randomUUID(), values }];
		const nextDraft: Record<string, string> = {};
		for (const column of editableColumns) nextDraft[column] = "";
		addRowDraft = nextDraft;
		addingNewRow = true;
	}

	function removePendingInsert(id: string) {
		pendingInserts = pendingInserts.filter((row) => row.id !== id);
	}

	function startInlineAddRow() {
		if (editableColumns.length === 0) return;
		addingNewRow = true;
		const nextDraft: Record<string, string> = {};
		for (const column of editableColumns) nextDraft[column] = addRowDraft[column] ?? "";
		addRowDraft = nextDraft;
	}

	function cancelInlineAddRow() {
		addingNewRow = false;
		const nextDraft: Record<string, string> = {};
		for (const column of editableColumns) nextDraft[column] = "";
		addRowDraft = nextDraft;
	}

	async function rerunContextQuery() {
		if (!resultContext) return;
		rerunning = true;
		try {
			if (refreshSql.trim().length > 0) {
				await onRunSql(refreshSql);
				return;
			}
			const sql = buildDefaultContextSql();
			await onRunSql(sql);
		} finally {
			rerunning = false;
		}
	}

	async function runExplain() {
		const sourceSql = refreshSql.trim().length > 0 ? refreshSql : buildDefaultContextSql();
		if (!sourceSql) return;
		const normalized = sourceSql.trim().replace(/;+\s*$/, "");
		const explainSql = /^\s*explain\b/i.test(normalized)
			? normalized
			: `explain (costs true, verbose false, format text) ${normalized}`;
		runningExplain = true;
		try {
			await onRunSql(explainSql);
			activeView = "results";
		} finally {
			runningExplain = false;
		}
	}

	async function syncChanges() {
		if (!resultContext || !onApplyTableChanges || !hasPendingChanges) return;
		syncingChanges = true;
		syncError = "";
		try {
			const payload: TableChangesPayload = {
				updates: Array.from(pendingUpdates.entries()).map(([ctid, values]) => ({ ctid, values })),
				deletes: Array.from(pendingDeletes),
				inserts: pendingInserts.map((row) => row.values),
			};
			const applyResult = await onApplyTableChanges(resultContext, payload);
			if (payload.inserts.length > 0) {
				clearCellHighlights();
				resetDraftState();
				await rerunContextQuery();
				return;
			}

			const updatedRowsByOldCtid = new Map(
				applyResult.updatedRows.map((entry) => [entry.oldCtid, entry]),
			);
			const updatesByOldCtid = new Map(payload.updates.map((entry) => [entry.ctid, entry.values]));
			const deleteSet = new Set(payload.deletes);
			const nextHighlights: Array<{ rowId: string; column: string }> = [];
			const nextRows = displayResult.rows
				.filter((row) => {
					const ctid = String(row["_querycastle_ctid"] ?? "");
					return !ctid || !deleteSet.has(ctid);
				})
				.map((row) => {
					const ctid = String(row["_querycastle_ctid"] ?? "");
					const updatedRow = ctid ? updatedRowsByOldCtid.get(ctid) : undefined;
					if (!updatedRow) return row;
					const optimisticValues = ctid ? updatesByOldCtid.get(ctid) : undefined;
					const merged = {
						...row,
						...(optimisticValues ?? {}),
						...updatedRow.values,
						_querycastle_ctid: updatedRow.newCtid,
					};
					for (const column of Object.keys(optimisticValues ?? {})) {
						nextHighlights.push({ rowId: updatedRow.newCtid, column });
					}
					return merged;
				});
			displayResult = {
				...displayResult,
				rows: nextRows,
				rowCount: nextRows.length,
			};
			resetDraftState();
			markCellsRecentlyUpdated(nextHighlights);
		} catch (error) {
			syncError = error instanceof Error ? error.message : String(error);
		} finally {
			syncingChanges = false;
		}
	}

	$effect(() => {
		return () => {
			clearCellHighlights();
		};
	});
</script>

<div class="flex-1 flex flex-col bg-white min-w-[320px] min-h-0">
	<div class="flex border-b border-gray-200 bg-gray-50/80 px-2 pt-2 shrink-0">
		<button
			onclick={() => (activeView = "results")}
			class={`px-4 py-1.5 border-b-2 text-sm font-medium rounded-t-md relative z-10 -mb-[1px] ${activeView === "results" ? "border-emerald-500 text-gray-900 bg-white shadow-[0_-1px_2px_rgba(0,0,0,0.02)]" : "border-transparent text-gray-500 hover:text-gray-700"}`}
		>
			Results
		</button>
		<button
			onclick={() => (activeView = "messages")}
			class={`px-4 py-1.5 border-b-2 text-sm font-medium ${activeView === "messages" ? "border-emerald-500 text-gray-900 bg-white rounded-t-md shadow-[0_-1px_2px_rgba(0,0,0,0.02)] -mb-[1px]" : "border-transparent text-gray-500 hover:text-gray-700"}`}
		>
			Messages
		</button>
		<button
			onclick={() => (activeView = "explain")}
			class={`px-4 py-1.5 border-b-2 text-sm font-medium ${activeView === "explain" ? "border-emerald-500 text-gray-900 bg-white rounded-t-md shadow-[0_-1px_2px_rgba(0,0,0,0.02)] -mb-[1px]" : "border-transparent text-gray-500 hover:text-gray-700"}`}
		>
			Explain
		</button>
		<div class="flex-1 border-b-2 border-transparent -mb-[1px]"></div>
		<div class="flex items-center space-x-2 px-2 border-b-2 border-transparent text-xs text-gray-500 font-medium -mb-[1px]">
			{#if editable}
				<button onclick={() => queueDeleteRows(Array.from(selectedRows))} disabled={selectedRows.size === 0} class="h-7 px-2 rounded border border-red-200 text-red-600 hover:bg-red-50 disabled:opacity-50 inline-flex items-center gap-1"><Trash2 size={12} />Delete Selected</button>
			{/if}
			<span class="flex items-center"><Timer size={14} class="mr-1.5 text-gray-400" /> {durationMs}ms</span>
			<span class="flex items-center"><Database size={14} class="mr-1.5 text-gray-400" /> {displayResult.rowCount} rows</span>
			{#if resultContext}
				<button onclick={rerunContextQuery} disabled={rerunning} class="text-gray-500 hover:text-gray-800 disabled:opacity-60 inline-flex items-center gap-1">
					<Play size={12} />
					{rerunning ? "Running" : "Refresh"}
				</button>
			{/if}
		</div>
	</div>

	{#if sqlError}
		<div class="px-4 py-2 text-xs text-red-600 border-b border-red-100 bg-red-50">{sqlError}</div>
	{/if}
	{#if syncError}
		<div class="px-4 py-2 text-xs text-red-600 border-b border-red-100 bg-red-50">{syncError}</div>
	{/if}

	<div class="flex-1 overflow-auto bg-white min-h-0">
		{#if activeView === "messages"}
			<div class="h-full p-4 text-xs text-gray-700 space-y-2">
				{#if sqlError || syncError}
					{#if sqlError}
						<div class="rounded border border-red-200 bg-red-50 px-3 py-2 text-red-700">
							<div class="font-semibold mb-1">SQL Error</div>
							<div class="whitespace-pre-wrap">{sqlError}</div>
						</div>
					{/if}
					{#if syncError}
						<div class="rounded border border-red-200 bg-red-50 px-3 py-2 text-red-700">
							<div class="font-semibold mb-1">Sync Error</div>
							<div class="whitespace-pre-wrap">{syncError}</div>
						</div>
					{/if}
				{:else}
					<div class="rounded border border-gray-200 bg-gray-50 px-3 py-2">
						Last query executed successfully in {durationMs}ms and returned {displayResult.rowCount} rows.
					</div>
				{/if}
			</div>
		{:else if activeView === "explain"}
			<div class="h-full p-4 space-y-3">
				<div class="text-xs text-gray-600">Run `EXPLAIN` for the current query/result source.</div>
				<button
					onclick={runExplain}
					disabled={runningExplain || loading}
					class="h-8 px-3 rounded border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-100 disabled:opacity-60"
				>
					{runningExplain ? "Running EXPLAIN..." : "Run EXPLAIN"}
				</button>
				{#if refreshSql.trim().length > 0 || resultContext}
					<div class="rounded border border-gray-200 bg-gray-50 p-3">
						<div class="text-[11px] font-semibold text-gray-500 mb-1">Source SQL</div>
						<pre class="font-mono-code text-[11px] text-gray-700 whitespace-pre-wrap break-words">{refreshSql.trim().length > 0 ? refreshSql : buildDefaultContextSql()}</pre>
					</div>
				{:else}
					<div class="text-xs text-gray-500">Run a query first to generate an explain plan.</div>
				{/if}
			</div>
		{:else if loading}
			<div class="min-w-max animate-pulse">
				<table class="min-w-full text-left border-collapse text-sm whitespace-nowrap">
					<thead class="sticky top-0 bg-gray-50 border-b border-gray-200 shadow-sm z-10">
						<tr>
							{#if editable}
								<th class="px-3 py-2 w-8 border-r border-gray-200"></th>
							{/if}
							<th class="px-4 py-2 w-12 border-r border-gray-200"></th>
							{#if visibleColumns.length > 0}
								{#each visibleColumns as _}
									<th class="px-4 py-2 border-r border-gray-200">
										<div class="h-3.5 w-24 rounded bg-gray-200"></div>
									</th>
								{/each}
							{:else}
								{#each Array.from({ length: 6 }) as _}
									<th class="px-4 py-2 border-r border-gray-200">
										<div class="h-3.5 w-24 rounded bg-gray-200"></div>
									</th>
								{/each}
							{/if}
						</tr>
					</thead>
					<tbody>
						{#each skeletonRows as _}
							<tr class="border-b border-gray-100">
								{#if editable}
									<td class="px-3 py-1.5 border-r border-gray-100">
										<div class="h-3.5 w-3.5 rounded bg-gray-200"></div>
									</td>
								{/if}
								<td class="px-4 py-1.5 border-r border-gray-100">
									<div class="h-3.5 w-6 rounded bg-gray-100"></div>
								</td>
								{#if visibleColumns.length > 0}
									{#each visibleColumns as _}
										<td class="px-4 py-1.5 border-r border-gray-100">
											<div class="h-3.5 w-full max-w-[180px] rounded bg-gray-200"></div>
										</td>
									{/each}
								{:else}
									{#each Array.from({ length: 6 }) as _}
										<td class="px-4 py-1.5 border-r border-gray-100">
											<div class="h-3.5 w-full max-w-[180px] rounded bg-gray-200"></div>
										</td>
									{/each}
								{/if}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{:else if displayResult.columns.length === 0}
			<div class="h-full flex items-center justify-center text-sm text-gray-500">Run a query to see results.</div>
		{:else}
			<div class="min-w-max">
				<table class="min-w-full text-left border-collapse text-sm whitespace-nowrap">
					<thead class="sticky top-0 bg-gray-50 border-b border-gray-200 shadow-sm z-10">
						<tr>
							{#if editable}
								<th class="px-3 py-2 font-medium text-gray-500 border-r border-gray-200 text-xs w-8">
									<input
										type="checkbox"
										onchange={toggleSelectAllVisible}
										checked={visibleRows.length > 0 && visibleRows.every((row) => selectedRows.has(String(row["_querycastle_ctid"] ?? "")))}
									/>
								</th>
							{/if}
							<th class="px-4 py-2 font-medium text-gray-500 w-12 border-r border-gray-200 text-xs">#</th>
							{#each visibleColumns as column}
								<th class="px-4 py-2 font-medium text-gray-500 border-r border-gray-200 text-xs">
									<div class="flex items-center">
										{#if column.endsWith("_id")}
											<KeyRound size={12} class="mr-1.5 text-gray-400" />
										{/if}
										{column}
									</div>
								</th>
							{/each}
						</tr>
					</thead>
					<tbody class="text-gray-700">
						{#each visibleRows as row, rowIndex (String(row["_querycastle_ctid"] ?? `row-${rowIndex}`))}
							{@const rowId = String(row["_querycastle_ctid"] ?? "")}
							<tr class="border-b border-gray-100 hover:bg-gray-50" oncontextmenu={(event) => openRowContextMenu(event, rowId)}>
								{#if editable}
									<td class="px-3 py-1.5 border-r border-gray-100">
										<input type="checkbox" checked={selectedRows.has(rowId)} onchange={() => toggleRowSelected(rowId)} />
									</td>
								{/if}
								<td class="px-4 py-1.5 text-gray-400 border-r border-gray-100 text-xs bg-gray-50/30">{rowIndex + 1}</td>
								{#each visibleColumns as column (column)}
									{@const currentValue = getRowValue(row, rowId, column)}
									{@const isEditing = editingCell && editingCell.rowId === rowId && editingCell.column === column}
									{@const isRecentlyUpdated = highlightCells.has(cellKey(rowId, column))}
									<td
										class={`px-4 py-1.5 border-r border-gray-100 font-mono-code text-[12px] transition-colors duration-700 ${editable && column !== "_querycastle_ctid" ? "cursor-text" : ""} ${isEditing ? "outline outline-1 -outline-offset-1 outline-emerald-400 bg-white" : ""} ${isRecentlyUpdated ? "bg-emerald-50/70" : ""}`}
										onclick={() => beginEdit(rowId, column, currentValue)}
									>
										{#if isEditing}
											<input
												value={editDraft}
												onblur={commitEdit}
												onkeydown={(event) => {
													if (event.key === "Enter") commitEdit();
													if (event.key === "Escape") discardEdit();
												}}
												oninput={(event) => (editDraft = (event.currentTarget as HTMLInputElement).value)}
												class="w-full border-0 bg-transparent text-[12px] leading-[1.25rem] outline-none p-0 m-0"
											/>
										{:else}
											{currentValue == null ? "NULL" : String(currentValue)}
										{/if}
									</td>
								{/each}
							</tr>
						{/each}

						{#if editable && pendingInserts.length > 0}
							{#each pendingInserts as insertRow, insertIndex}
								<tr class="border-b border-emerald-100 bg-emerald-50/30">
									<td class="px-3 py-1.5 border-r border-gray-100 text-gray-400 text-xs">
										<button onclick={() => removePendingInsert(insertRow.id)} class="text-red-500 hover:text-red-700" aria-label="Remove pending insert">x</button>
									</td>
									<td class="px-4 py-1.5 text-gray-400 border-r border-gray-100 text-xs bg-gray-50/30">N{insertIndex + 1}</td>
									{#each visibleColumns as column}
										<td class="px-4 py-1.5 border-r border-gray-100 font-mono-code text-[12px]">
											{insertRow.values[column] == null ? "NULL" : String(insertRow.values[column])}
										</td>
									{/each}
								</tr>
							{/each}
						{/if}
					</tbody>
					{#if editable && !addingNewRow}
						<tbody>
							<tr class="border-b border-dashed border-gray-300 bg-gray-50/50">
								<td colspan={visibleColumns.length + (editable ? 2 : 1)} class="px-3 py-2">
									<button onclick={startInlineAddRow} class="w-full h-9 rounded-md text-left px-3 text-xs text-gray-500 hover:text-gray-700 hover:bg-white border border-dashed border-transparent hover:border-gray-300">
										+ Click here to add a new row
									</button>
								</td>
							</tr>
						</tbody>
					{:else if editable && addingNewRow}
						<tbody>
							<tr class="border-b border-dashed border-gray-300 bg-gray-50/60">
								<td class="px-3 py-2 border-r border-gray-100 text-gray-400 text-xs">+</td>
								<td class="px-4 py-2 border-r border-gray-100 text-gray-500 text-xs">New</td>
								{#each visibleColumns as column}
									<td class="px-2 py-1.5 border-r border-gray-100">
										<input
											value={addRowDraft[column] ?? ""}
											oninput={(event) => (addRowDraft = { ...addRowDraft, [column]: (event.currentTarget as HTMLInputElement).value })}
											onkeydown={(event) => {
												if (event.key === "Enter") submitAddRow();
											}}
											placeholder={column}
											class="w-full h-8 px-2 rounded-md border border-gray-200 bg-white text-[12px] text-gray-800 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500"
										/>
									</td>
								{/each}
							</tr>
							<tr class="bg-white">
								<td colspan={visibleColumns.length + (editable ? 2 : 1)} class="px-3 py-2 border-b border-gray-100">
									<div class="flex justify-end gap-2">
										<button onclick={cancelInlineAddRow} class="h-8 px-3 rounded border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-100">Cancel</button>
										<button onclick={submitAddRow} class="h-8 px-3 rounded border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-100">Add Staged Row</button>
									</div>
								</td>
							</tr>
						</tbody>
					{/if}
				</table>
			</div>
		{/if}
	</div>

	{#if hasPendingChanges}
		<div class="shrink-0 border-t border-amber-200 bg-amber-50 px-3 py-2 flex items-center justify-between">
			<div class="text-xs text-amber-700">You have unsaved changes. Please Save/Commit.</div>
			<div class="flex items-center gap-2">
				<button onclick={resetDraftState} class="h-8 px-3 rounded border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-100">Discard</button>
				<button onclick={syncChanges} disabled={syncingChanges} class="h-8 px-3 rounded border border-emerald-500 bg-emerald-500 text-xs text-white hover:bg-emerald-600 disabled:opacity-60">{syncingChanges ? "Committing..." : "Save / Commit"}</button>
			</div>
		</div>
	{/if}

	{#if rowContextMenu}
		<button class="fixed inset-0 z-40" aria-label="Close row menu" onclick={() => (rowContextMenu = null)}></button>
		<div class="fixed z-50 min-w-[170px] bg-white rounded-md border border-gray-200 shadow-[0_8px_24px_rgba(0,0,0,0.12)] py-1" style={`left:${rowContextMenu?.x ?? 0}px;top:${rowContextMenu?.y ?? 0}px;`}>
			<button onclick={() => rowContextMenu && queueDeleteRows([rowContextMenu.rowId])} class="w-full px-3 py-1.5 text-left text-sm text-red-600 hover:bg-red-50 inline-flex items-center gap-2">
				<Trash2 size={14} />
				Delete Row
			</button>
		</div>
	{/if}
</div>


