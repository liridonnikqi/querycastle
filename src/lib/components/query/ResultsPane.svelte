<script lang="ts">
	import {
		ArrowDown,
		ArrowUp,
		ArrowUpDown,
		ArrowUpRight,
		ChevronLeft,
		ChevronRight,
		Filter,
		Play,
		Plus,
		Timer,
		Trash2,
		X,
	} from '@lucide/svelte';
	import type {
		ApplyTableChangesResult,
		DatabaseExplorer,
		DatabaseForeignKey,
		DatabaseType,
		QueryResultPayload,
		TableChangesPayload,
	} from '$lib/rpc';
	import { rpc } from '$lib/rpc-client';
	import {
		outgoingFkColumns,
		resolveIncomingRelations,
		resolveOutgoingRelations,
		type IncomingRelation,
	} from '$lib/utils/relation-resolve';
	import {
		createRelationHop,
		formatFollowValue,
		isFollowableValue,
	} from '$lib/utils/relation-sql';
	import { loadFkOptions, type FkOption } from '$lib/utils/fk-lookup';
	import {
		coerceByColumn,
		displayCellText,
		formatBooleanLabel,
		gridColumnsForTable,
		isEmptyCell,
		missingRequiredColumns,
		valuesForInsert,
		type GridColumnMeta,
	} from '$lib/utils/grid-editors';
	import { quoteSqlIdentifier } from '$lib/utils/sql';
	import type { RelationHop } from '$lib/utils/workspace';
	import RelationTrail from '$lib/components/query/RelationTrail.svelte';
	import ColumnTypeIcon from '$lib/components/query/ColumnTypeIcon.svelte';
	import GridCellEditor from '$lib/components/query/GridCellEditor.svelte';
	import PendingChangesPane from '$lib/components/query/PendingChangesPane.svelte';
	import {
		buildPendingChangeCards,
		buildPendingSqlPreview,
		pendingChangeCount,
	} from '$lib/utils/pending-changes';
	import {
		PAGE_SIZE_OPTIONS,
		buildTableBrowseSql,
		buildTableCountSql,
		extractWhereClause,
		nextSortState,
		parseCountResult,
		totalPages,
		filterSortRows,
		type GridSort,
		type PageSize,
	} from '$lib/utils/table-browse';

	type RowContextMenu = {
		x: number;
		y: number;
		rowId: string;
		row: Record<string, unknown>;
	} | null;
	type EditingCell = { rowId: string; column: string } | null;
	type PendingInsertRow = { id: string; values: Record<string, unknown> };
	type ResultView = 'results' | 'messages' | 'explain';
	type ColumnResizeState = {
		column: string;
		startX: number;
		startWidth: number;
	} | null;

	let {
		result,
		sqlError,
		databaseType,
		resultContext,
		explorer = null,
		relationTrail = [],
		onRunSql,
		onApplyTableChanges,
		onFollowRelation,
		onActivateRelationTrail,
		durationMs = 0,
		loading = false,
		refreshSql = '',
	}: {
		result: QueryResultPayload;
		sqlError: string;
		databaseType: DatabaseType;
		resultContext: { schema: string; table: string } | null;
		explorer?: DatabaseExplorer | null;
		relationTrail?: RelationHop[];
		onRunSql: (sql: string) => Promise<void>;
		onApplyTableChanges?: (
			context: { schema: string; table: string },
			changes: TableChangesPayload,
		) => Promise<ApplyTableChangesResult>;
		onFollowRelation?: (hop: RelationHop) => void | Promise<void>;
		onActivateRelationTrail?: (index: number) => void | Promise<void>;
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
	let syncError = $state('');
	let selectedRows = $state(new Set<string>());
	let activeRowId = $state<string | null>(null);
	let pendingUpdates = $state(new Map<string, Record<string, unknown>>());
	let pendingDeletes = $state(new Set<string>());
	let pendingInserts = $state<PendingInsertRow[]>([]);
	let rowContextMenu = $state<RowContextMenu>(null);
	let relatedSubmenuOpen = $state(false);
	let editingCell = $state<EditingCell>(null);
	let editDraft = $state('');
	let columnWidths = $state<Record<string, number>>({});
	let columnResizeState = $state<ColumnResizeState>(null);
	let highlightCells = $state(new Set<string>());
	let highlightTimeouts = new Map<string, ReturnType<typeof setTimeout>>();
	let lastExternalResultSignature = $state('');
	let activeView = $state<ResultView>('results');
	const minColumnWidth = 120;

	let fkOptionCache = $state(new Map<string, FkOption[]>());
	let fkLoadingKeys = $state(new Set<string>());
	let keepDraftsOnNextResult = $state(false);
	let pageSize = $state<PageSize>(100);
	let page = $state(1);
	let totalRowCount = $state(0);
	let sort = $state<GridSort | null>(null);
	let columnFilters = $state<Record<string, string>>({});
	let showFilterRow = $state(false);
	let showSortMenu = $state(false);
	let pendingPanelOpen = $state(false);
	let userCollapsedPending = $state(false);
	let baseWhere = $state('');
	let lastBrowseSourceKey = $state('');
	let deletedSnapshots = $state(new Map<string, Record<string, unknown>>());
	let filterTimer: ReturnType<typeof setTimeout> | null = null;

	let editable = $derived.by(
		() =>
			!!resultContext && displayResult.columns.includes('_querycastle_ctid'),
	);
	let visibleColumns = $derived.by(() =>
		displayResult.columns.filter((column) => column !== '_querycastle_ctid'),
	);
	let visibleRows = $derived.by(() =>
		displayResult.rows.filter(
			(row) => !pendingDeletes.has(String(row['_querycastle_ctid'] ?? '')),
		),
	);
	let columnMetas = $derived.by(() =>
		gridColumnsForTable(
			explorer ?? null,
			resultContext,
			visibleColumns,
			sampleValue,
		),
	);
	let changeCount = $derived.by(() =>
		pendingChangeCount({
			updates: pendingUpdates,
			inserts: pendingInserts,
			deletes: pendingDeletes,
		}),
	);
	let pendingCards = $derived.by(() => {
		if (!resultContext) return [];
		return buildPendingChangeCards({
			schema: resultContext.schema,
			table: resultContext.table,
			rows: displayResult.rows,
			updates: pendingUpdates,
			inserts: pendingInserts,
			deletes: pendingDeletes,
			deletedSnapshots,
		});
	});
	let pendingSqlPreview = $derived.by(() => {
		if (!resultContext) return '';
		return buildPendingSqlPreview({
			databaseType,
			schema: resultContext.schema,
			table: resultContext.table,
			updates: Array.from(pendingUpdates.entries()).map(([ctid, values]) => ({ ctid, values })),
			deletes: Array.from(pendingDeletes),
			inserts: pendingInserts.map((row) => row.values),
		});
	});
	let browseSourceKey = $derived.by(() => {
		const contextKey = resultContext ? `${resultContext.schema}.${resultContext.table}` : '';
		const trailKey = relationTrail
			.map((hop) => `${hop.direction}:${hop.to.schema}.${hop.to.table}:${String(hop.from.value)}`)
			.join('|');
		return `${contextKey}::${trailKey}`;
	});
	let canServerBrowse = $derived(!!resultContext);
	let filterList = $derived.by(() =>
		Object.entries(columnFilters)
			.filter(([, value]) => value.trim().length > 0)
			.map(([column, value]) => ({ column, value })),
	);
	let hasActiveFilters = $derived(filterList.length > 0);
	let effectiveSort = $derived.by((): GridSort | null => {
		if (sort) return sort;
		const column = visibleColumns[0];
		return column ? { column, dir: 'asc' } : null;
	});
	let clientPreparedRows = $derived.by(() =>
		canServerBrowse ? visibleRows : filterSortRows(visibleRows, filterList, effectiveSort),
	);
	let displayTotal = $derived(canServerBrowse ? totalRowCount : clientPreparedRows.length);
	let pageCount = $derived.by(() => totalPages(displayTotal, pageSize));
	let pageRows = $derived.by(() => {
		if (canServerBrowse) return visibleRows;
		const start = (page - 1) * pageSize;
		return clientPreparedRows.slice(start, start + pageSize);
	});
	const skeletonRowCount = 10;
	const skeletonRows = Array.from(
		{ length: skeletonRowCount },
		(_, index) => index,
	);
	let editingCellHasPendingChange = $derived.by(() => {
		if (!editingCell) return false;
		const { rowId, column } = editingCell;
		const row = displayResult.rows.find(
			(item) => String(item['_querycastle_ctid'] ?? '') === rowId,
		);
		if (!row) return false;
		const nextValue = coerceValue(editDraft, column);
		return !valuesEqual(nextValue, row[column]);
	});
	let hasPendingChanges = $derived.by(
		() =>
			pendingUpdates.size > 0 ||
			pendingDeletes.size > 0 ||
			pendingInserts.length > 0 ||
			editingCellHasPendingChange,
	);

	let fkColumns = $derived.by(() =>
		outgoingFkColumns(explorer ?? null, resultContext, visibleColumns),
	);

	function quoteIdent(value: string) {
		return quoteSqlIdentifier(databaseType, value);
	}

	function buildExternalResultSignature(
		context: { schema: string; table: string } | null,
		payload: QueryResultPayload,
	): string {
		return JSON.stringify({
			context: context ? `${context.schema}.${context.table}` : '',
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
		relatedSubmenuOpen = false;
		editingCell = null;
		editDraft = '';
		syncError = '';
		deletedSnapshots = new Map();
		keepDraftsOnNextResult = false;
		pendingPanelOpen = false;
		userCollapsedPending = false;
	}

	function buildDefaultContextSql() {
		if (!resultContext) return '';
		const firstVisibleColumn = visibleColumns[0];
		const orderByClause = firstVisibleColumn
			? ` order by ${quoteIdent(firstVisibleColumn)} asc nulls last`
			: '';
		if (databaseType === 'sqlite') {
			return `select cast(rowid as text) as _querycastle_ctid, * from ${quoteIdent(resultContext.schema)}.${quoteIdent(resultContext.table)}${orderByClause} limit 100;`;
		}
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
		if (keepDraftsOnNextResult) {
			keepDraftsOnNextResult = false;
			if (!canServerBrowse) totalRowCount = result.rowCount;
			return;
		}
		clearCellHighlights();
		resetDraftState();
		if (!canServerBrowse) totalRowCount = result.rowCount;
		activeView = 'results';
	});

	function coerceValue(raw: string, column: string): unknown {
		return coerceByColumn(raw, metaFor(column), sampleValue(column));
	}

	function sampleValue(column: string): unknown {
		const firstRow = displayResult.rows.find(
			(row) => row[column] !== undefined,
		);
		return firstRow ? firstRow[column] : null;
	}

	function metaFor(column: string): GridColumnMeta | undefined {
		return columnMetas.find((item) => item.name === column);
	}

	function fkCacheKey(fk: { referencedSchema: string; referencedTable: string; referencedColumn: string }) {
		return `${fk.referencedSchema}.${fk.referencedTable}.${fk.referencedColumn}`;
	}

	async function ensureFkOptions(
		fk: { referencedSchema: string; referencedTable: string; referencedColumn: string; column: string },
		search = '',
	) {
		const key = `${fkCacheKey(fk)}::${search.trim().toLowerCase()}`;
		if (fkOptionCache.has(key) || fkLoadingKeys.has(key)) return;
		const nextLoading = new Set(fkLoadingKeys);
		nextLoading.add(key);
		fkLoadingKeys = nextLoading;
		try {
			const options = await loadFkOptions({
				runQuery: (sql) => rpc.request.runQuery({ sql }),
				databaseType,
				explorer: explorer ?? null,
				fk,
				search,
			});
			const nextCache = new Map(fkOptionCache);
			nextCache.set(key, options);
			const baseKey = fkCacheKey(fk);
			const existing = nextCache.get(baseKey) ?? [];
			const merged = [...existing];
			for (const option of options) {
				if (!merged.some((item) => String(item.id) === String(option.id))) {
					merged.push(option);
				}
			}
			nextCache.set(baseKey, merged);
			fkOptionCache = nextCache;
		} catch {
			const nextCache = new Map(fkOptionCache);
			if (!nextCache.has(fkCacheKey(fk))) nextCache.set(fkCacheKey(fk), []);
			fkOptionCache = nextCache;
		} finally {
			const done = new Set(fkLoadingKeys);
			done.delete(key);
			fkLoadingKeys = done;
		}
	}

	function isFkLoading(fk: { referencedSchema: string; referencedTable: string; referencedColumn: string }) {
		const prefix = fkCacheKey(fk);
		for (const key of fkLoadingKeys) {
			if (key === prefix || key.startsWith(`${prefix}::`)) return true;
		}
		return false;
	}

	function optionsForFk(fk: { referencedSchema: string; referencedTable: string; referencedColumn: string } | null) {
		if (!fk) return [];
		return fkOptionCache.get(fkCacheKey(fk)) ?? [];
	}

	function draftFromValue(column: string, value: unknown): string {
		const meta = metaFor(column);
		if (isEmptyCell(value)) return '';
		if (meta?.kind === 'boolean') {
			if (value === true || value === 'true' || value === 1 || value === '1') return 'true';
			if (value === false || value === 'false' || value === 0 || value === '0') return 'false';
		}
		if (meta?.kind === 'date') return String(value).slice(0, 10);
		if (meta?.kind === 'datetime') {
			const text = String(value).trim();
			const match = text.match(/^(\d{4}-\d{2}-\d{2})[ T](\d{2}:\d{2})/);
			return match ? `${match[1]}T${match[2]}` : text;
		}
		return String(value);
	}

	function valuesEqual(a: unknown, b: unknown): boolean {
		if (a === b) return true;
		if (a == null && b == null) return true;
		return false;
	}

	function getRowValue(
		row: Record<string, unknown>,
		rowId: string,
		column: string,
	): unknown {
		return pendingUpdates.get(rowId)?.[column] ?? row[column];
	}

	function getRowValueByName(
		row: Record<string, unknown>,
		rowId: string,
		column: string,
	): unknown {
		if (Object.prototype.hasOwnProperty.call(row, column) || pendingUpdates.get(rowId)?.[column] !== undefined) {
			return getRowValue(row, rowId, column);
		}
		const match = visibleColumns.find(
			(item) => item.toLowerCase() === column.toLowerCase(),
		);
		if (!match) return undefined;
		return getRowValue(row, rowId, match);
	}

	function blockFollowIfPending(): boolean {
		return hasPendingChanges;
	}

	function startOutgoingFollow(fk: DatabaseForeignKey, value: unknown) {
		if (!resultContext || !onFollowRelation || !isFollowableValue(value)) return;
		if (blockFollowIfPending()) return;
		void onFollowRelation(
			createRelationHop({
				direction: 'outgoing',
				from: {
					schema: resultContext.schema,
					table: resultContext.table,
					column: fk.column,
					value,
				},
				to: {
					schema: fk.referencedSchema,
					table: fk.referencedTable,
					column: fk.referencedColumn,
				},
			}),
		);
		rowContextMenu = null;
	}

	function startIncomingFollow(rel: IncomingRelation, value: unknown) {
		if (!resultContext || !onFollowRelation || !isFollowableValue(value)) return;
		if (blockFollowIfPending()) return;
		void onFollowRelation(
			createRelationHop({
				direction: 'incoming',
				from: {
					schema: resultContext.schema,
					table: resultContext.table,
					column: rel.fk.referencedColumn,
					value,
				},
				to: {
					schema: rel.schema,
					table: rel.table,
					column: rel.fk.column,
				},
			}),
		);
		rowContextMenu = null;
	}

	function outgoingActions(
		row: Record<string, unknown>,
		rowId: string,
	): Array<{ fk: DatabaseForeignKey; value: unknown }> {
		if (!explorer || !resultContext || !onFollowRelation) return [];
		const items: Array<{ fk: DatabaseForeignKey; value: unknown }> = [];
		for (const column of visibleColumns) {
			const value = getRowValue(row, rowId, column);
			for (const fk of resolveOutgoingRelations(explorer, resultContext, column)) {
				if (!isFollowableValue(value)) continue;
				items.push({ fk, value });
			}
		}
		return items;
	}

	function incomingActions(
		row: Record<string, unknown>,
		rowId: string,
	): Array<IncomingRelation & { value: unknown }> {
		if (!explorer || !resultContext || !onFollowRelation) return [];
		const items: Array<IncomingRelation & { value: unknown }> = [];
		for (const rel of resolveIncomingRelations(explorer, resultContext)) {
			const value = getRowValueByName(row, rowId, rel.fk.referencedColumn);
			if (!isFollowableValue(value)) continue;
			items.push({ ...rel, value });
		}
		return items;
	}

	function incomingMenuLabel(
		rel: IncomingRelation,
		all: Array<IncomingRelation & { value: unknown }>,
	): string {
		const selfRef =
			!!resultContext &&
			rel.table === resultContext.table &&
			rel.schema === resultContext.schema;
		const duplicates = all.filter((item) => item.table === rel.table).length > 1;
		return selfRef || duplicates
			? `Related in ${rel.table} (${rel.fk.column})`
			: `Related in ${rel.table}`;
	}

	function handleCellClick(
		event: MouseEvent,
		rowId: string,
		column: string,
		currentValue: unknown,
	) {
		if (rowId) activeRowId = rowId;
		if (editingCell?.rowId === rowId && editingCell?.column === column) return;
		const fks = explorer
			? resolveOutgoingRelations(explorer, resultContext, column)
			: [];
		const canFollow =
			fks.length > 0 && isFollowableValue(currentValue);
		const modifierClick = event.altKey || event.metaKey || event.ctrlKey;
		if (canFollow && (modifierClick || !editable)) {
			event.preventDefault();
			startOutgoingFollow(fks[0]!, currentValue);
			return;
		}
		beginEdit(rowId, column, currentValue);
	}

	function cellKey(rowId: string, column: string): string {
		return `${rowId}::${column}`;
	}

	function markCellsRecentlyUpdated(
		cells: Array<{ rowId: string; column: string }>,
	) {
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

	function getColumnWidth(column: string): number {
		return (
			columnWidths[column] ??
			Math.max(minColumnWidth, Math.min(260, column.length * 11 + 56))
		);
	}

	function setColumnWidth(column: string, width: number) {
		const nextWidth = Math.max(minColumnWidth, Math.round(width));
		if ((columnWidths[column] ?? getColumnWidth(column)) === nextWidth) return;
		columnWidths = { ...columnWidths, [column]: nextWidth };
	}

	function beginColumnResize(event: PointerEvent, column: string) {
		if (event.button !== 0) return;
		event.preventDefault();
		event.stopPropagation();
		columnResizeState = {
			column,
			startX: event.clientX,
			startWidth: getColumnWidth(column),
		};
	}

	function handleColumnResizeMove(event: PointerEvent) {
		if (!columnResizeState) return;
		const deltaX = event.clientX - columnResizeState.startX;
		setColumnWidth(
			columnResizeState.column,
			columnResizeState.startWidth + deltaX,
		);
	}

	function stopColumnResize() {
		if (!columnResizeState) return;
		columnResizeState = null;
	}

	function beginEdit(rowId: string, column: string, currentValue: unknown) {
		if (!editable || column === '_querycastle_ctid') return;
		const meta = metaFor(column);
		if (meta?.isAuto || meta?.isPrimary) return;
		editingCell = { rowId, column };
		editDraft = draftFromValue(column, currentValue);
		if (meta?.fk) void ensureFkOptions(meta.fk);
	}

	function commitEdit() {
		if (!editingCell) return;
		const { rowId, column } = editingCell;
		const nextValue = coerceValue(editDraft, column);
		const map = new Map(pendingUpdates);
		const row = displayResult.rows.find(
			(item) => String(item['_querycastle_ctid'] ?? '') === rowId,
		);
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
		if (!userCollapsedPending) pendingPanelOpen = true;

		editingCell = null;
		editDraft = '';
	}

	function discardEdit() {
		editingCell = null;
		editDraft = '';
	}

	function toggleRowSelected(rowId: string) {
		const next = new Set(selectedRows);
		if (next.has(rowId)) next.delete(rowId);
		else next.add(rowId);
		selectedRows = next;
		activeRowId = rowId;
	}

	function toggleSelectAllVisible() {
		const ids = pageRows
			.map((row) => String(row['_querycastle_ctid'] ?? ''))
			.filter((id) => id.length > 0);
		const allSelected =
			ids.length > 0 && ids.every((id) => selectedRows.has(id));
		selectedRows = allSelected ? new Set() : new Set(ids);
	}

	function queueDeleteRows(rowIds: string[]) {
		const nextDeletes = new Set(pendingDeletes);
		const nextUpdates = new Map(pendingUpdates);
		const nextSelected = new Set(selectedRows);
		for (const rowId of rowIds) {
			const row = displayResult.rows.find(
				(item) => String(item['_querycastle_ctid'] ?? '') === rowId,
			);
			if (row) {
				const nextSnapshots = new Map(deletedSnapshots);
				nextSnapshots.set(rowId, { ...row });
				deletedSnapshots = nextSnapshots;
			}
			nextDeletes.add(rowId);
			nextUpdates.delete(rowId);
			nextSelected.delete(rowId);
		}
		pendingDeletes = nextDeletes;
		pendingUpdates = nextUpdates;
		selectedRows = nextSelected;
		rowContextMenu = null;
		if (!userCollapsedPending) pendingPanelOpen = true;
	}

	function openRowContextMenu(
		event: MouseEvent,
		rowId: string,
		row: Record<string, unknown>,
	) {
		const outgoing = outgoingActions(row, rowId);
		const incoming = incomingActions(row, rowId);
		if (!editable && outgoing.length === 0 && incoming.length === 0) return;
		event.preventDefault();
		relatedSubmenuOpen = false;
		rowContextMenu = { x: event.clientX, y: event.clientY, rowId, row };
	}

	function startInsertRow() {
		if (!editable) return;
		const values: Record<string, unknown> = {};
		pendingInserts = [...pendingInserts, { id: crypto.randomUUID(), values }];
		if (!userCollapsedPending) pendingPanelOpen = true;
		for (const column of columnMetas) {
			if (column.fk) void ensureFkOptions(column.fk);
		}
	}

	function setInsertValue(id: string, column: string, raw: string) {
		pendingInserts = pendingInserts.map((row) => {
			if (row.id !== id) return row;
			return {
				...row,
				values: { ...row.values, [column]: coerceValue(raw, column) },
			};
		});
	}

	function removePendingInsert(id: string) {
		pendingInserts = pendingInserts.filter((row) => row.id !== id);
	}

	async function refreshCount() {
		if (!resultContext) {
			totalRowCount = visibleRows.length;
			return;
		}
		try {
			const payload = await rpc.request.runQuery({
				sql: buildTableCountSql({
					databaseType,
					schema: resultContext.schema,
					table: resultContext.table,
					baseWhere,
					filters: filterList,
				}),
			});
			totalRowCount = parseCountResult(payload.rows);
		} catch {
			totalRowCount = Math.max(displayResult.rowCount, visibleRows.length);
		}
	}

	async function applyBrowse() {
		if (!resultContext) {
			totalRowCount = visibleRows.length;
			const maxPage = totalPages(totalRowCount, pageSize);
			if (page > maxPage) page = maxPage;
			return;
		}
		const sql = buildTableBrowseSql({
			databaseType,
			explorer: explorer ?? null,
			schema: resultContext.schema,
			table: resultContext.table,
			baseWhere,
			filters: filterList,
			sort: effectiveSort,
			limit: pageSize,
			offset: (page - 1) * pageSize,
		});
		if (!sql) return;
		keepDraftsOnNextResult = true;
		rerunning = true;
		try {
			await onRunSql(sql);
			await refreshCount();
		} finally {
			rerunning = false;
		}
	}

	async function rerunContextQuery() {
		page = 1;
		await applyBrowse();
	}

	function scheduleFilterBrowse() {
		if (filterTimer) clearTimeout(filterTimer);
		filterTimer = setTimeout(() => {
			page = 1;
			void applyBrowse();
		}, 280);
	}

	function handleHeaderSort(column: string) {
		sort = nextSortState(sort, column);
		page = 1;
		showSortMenu = false;
		void applyBrowse();
	}

	function clearSort() {
		sort = null;
		showSortMenu = false;
		page = 1;
		void applyBrowse();
	}

	function clearFilters() {
		columnFilters = {};
		showFilterRow = false;
		page = 1;
		if (filterTimer) clearTimeout(filterTimer);
		void applyBrowse();
	}

	function setPageSize(next: PageSize) {
		pageSize = next;
		page = 1;
		void applyBrowse();
	}

	function goToPage(next: number) {
		const maxPage = pageCount;
		page = Math.min(maxPage, Math.max(1, next));
		void applyBrowse();
	}

	$effect(() => {
		const key = browseSourceKey;
		if (key === lastBrowseSourceKey) return;
		lastBrowseSourceKey = key;
		baseWhere = extractWhereClause(refreshSql);
		page = 1;
		columnFilters = {};
		sort = null;
		if (resultContext) void refreshCount();
		else totalRowCount = displayResult.rowCount;
	});

	$effect(() => {
		if (changeCount > 0 && !userCollapsedPending) pendingPanelOpen = true;
		if (changeCount === 0) {
			pendingPanelOpen = false;
			userCollapsedPending = false;
		}
	});

	async function runExplain() {
		const sourceSql =
			refreshSql.trim().length > 0 ? refreshSql : buildDefaultContextSql();
		if (!sourceSql) return;
		const normalized = sourceSql.trim().replace(/;+\s*$/, '');
		const explainSql = /^\s*explain\b/i.test(normalized)
			? normalized
			: `explain (costs true, verbose false, format text) ${normalized}`;
		runningExplain = true;
		try {
			await onRunSql(explainSql);
			activeView = 'results';
		} finally {
			runningExplain = false;
		}
	}

	async function syncChanges() {
		if (!resultContext || !onApplyTableChanges || !hasPendingChanges) return;
		if (editingCell) commitEdit();
		for (const insert of pendingInserts) {
			const draft: Record<string, string> = {};
			for (const column of columnMetas) {
				draft[column.name] = isEmptyCell(insert.values[column.name])
					? ''
					: String(insert.values[column.name]);
			}
			const missing = missingRequiredColumns(draft, columnMetas);
			if (missing.length > 0) {
				syncError = `New row is missing ${missing.join(', ')}.`;
				pendingPanelOpen = true;
				return;
			}
		}
		syncingChanges = true;
		syncError = '';
		try {
			const payload: TableChangesPayload = {
				updates: Array.from(pendingUpdates.entries()).map(([ctid, values]) => ({
					ctid,
					values,
				})),
				deletes: Array.from(pendingDeletes),
				inserts: pendingInserts
					.map((row) => valuesForInsert(
						Object.fromEntries(
							Object.entries(row.values).map(([key, value]) => [
								key,
								isEmptyCell(value) ? '' : String(value),
							]),
						),
						columnMetas,
						sampleValue,
					))
					.filter((row) => Object.keys(row).length > 0),
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
			const updatesByOldCtid = new Map(
				payload.updates.map((entry) => [entry.ctid, entry.values]),
			);
			const deleteSet = new Set(payload.deletes);
			const nextHighlights: Array<{ rowId: string; column: string }> = [];
			const nextRows = displayResult.rows
				.filter((row) => {
					const ctid = String(row['_querycastle_ctid'] ?? '');
					return !ctid || !deleteSet.has(ctid);
				})
				.map((row) => {
					const ctid = String(row['_querycastle_ctid'] ?? '');
					const updatedRow = ctid ? updatedRowsByOldCtid.get(ctid) : undefined;
					if (!updatedRow) return row;
					const optimisticValues = ctid
						? updatesByOldCtid.get(ctid)
						: undefined;
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

	function scheduleFkSearch(
		fk: { referencedSchema: string; referencedTable: string; referencedColumn: string; column: string },
		query: string,
	) {
		window.setTimeout(() => {
			void ensureFkOptions(fk, query);
		}, 180);
	}

	$effect(() => {
		const columnsSet = new Set(visibleColumns);
		const nextWidths: Record<string, number> = {};
		for (const [column, width] of Object.entries(columnWidths)) {
			if (columnsSet.has(column)) nextWidths[column] = width;
		}
		if (Object.keys(nextWidths).length !== Object.keys(columnWidths).length) {
			columnWidths = nextWidths;
		}
	});

	$effect(() => {
		if (!columnResizeState) return;
		const moveListener = (event: PointerEvent) => handleColumnResizeMove(event);
		const upListener = () => stopColumnResize();
		window.addEventListener('pointermove', moveListener);
		window.addEventListener('pointerup', upListener);
		document.body.style.userSelect = 'none';
		document.body.style.cursor = 'col-resize';
		return () => {
			window.removeEventListener('pointermove', moveListener);
			window.removeEventListener('pointerup', upListener);
			document.body.style.userSelect = '';
			document.body.style.cursor = '';
		};
	});

	$effect(() => {
		return () => {
			stopColumnResize();
			clearCellHighlights();
		};
	});
</script>

<div class="flex-1 flex flex-col bg-white min-w-[320px] min-h-0">
	<RelationTrail
		trail={relationTrail}
		onActivate={(index) => onActivateRelationTrail?.(index)}
	/>
	<div class="h-11 px-3 border-b border-gray-200 bg-gray-50 shrink-0 flex items-center gap-2">
		{#if editable}
			<button
				type="button"
				onclick={startInsertRow}
				disabled={syncingChanges}
				class="h-7 px-2.5 rounded-md bg-emerald-500 text-white text-xs font-medium hover:bg-emerald-600 disabled:opacity-50 inline-flex items-center gap-1"
			>
				<Plus size={12} />Insert
			</button>
			<button
				type="button"
				onclick={() => queueDeleteRows(Array.from(selectedRows))}
				disabled={selectedRows.size === 0}
				class="h-7 px-2 rounded-md border border-gray-200 bg-white text-xs text-red-600 hover:bg-red-50 disabled:opacity-50 inline-flex items-center gap-1"
			>
				<Trash2 size={12} />Delete
			</button>
		{/if}
		<button
			type="button"
			onclick={rerunContextQuery}
			disabled={rerunning || loading}
			class="h-7 px-2 rounded-md border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-50 disabled:opacity-50 inline-flex items-center gap-1"
		>
			<Play size={12} />
			{rerunning ? 'Running' : 'Refresh'}
		</button>
		<div class="inline-flex items-center">
			<button
				type="button"
				onclick={() => (showFilterRow = !showFilterRow)}
				class={`h-7 px-2 rounded-md border text-xs inline-flex items-center gap-1 ${showFilterRow || hasActiveFilters ? 'border-emerald-400 bg-emerald-50 text-emerald-800' : 'border-gray-200 bg-white text-gray-700 hover:bg-gray-50'} ${hasActiveFilters ? 'rounded-r-none' : ''}`}
			>
				<Filter size={12} />Filter
				{#if hasActiveFilters}
					<span class="text-[10px] font-medium">({filterList.length})</span>
				{/if}
			</button>
			{#if hasActiveFilters}
				<button
					type="button"
					onclick={clearFilters}
					class="h-7 w-7 rounded-md rounded-l-none border border-l-0 border-emerald-400 bg-emerald-50 text-emerald-800 hover:bg-emerald-100 inline-flex items-center justify-center"
					title="Clear filters"
					aria-label="Clear filters"
				>
					<X size={12} />
				</button>
			{/if}
		</div>
		<div class="relative inline-flex items-center">
			<button
				type="button"
				onclick={() => (showSortMenu = !showSortMenu)}
				class={`h-7 px-2 rounded-md border text-xs inline-flex items-center gap-1 ${sort ? 'border-emerald-400 bg-emerald-50 text-emerald-800' : 'border-gray-200 bg-white text-gray-700 hover:bg-gray-50'} ${sort ? 'rounded-r-none' : ''}`}
			>
				<ArrowUpDown size={12} />Sort
			</button>
			{#if sort}
				<button
					type="button"
					onclick={clearSort}
					class="h-7 w-7 rounded-md rounded-l-none border border-l-0 border-emerald-400 bg-emerald-50 text-emerald-800 hover:bg-emerald-100 inline-flex items-center justify-center"
					title="Clear sort"
					aria-label="Clear sort"
				>
					<X size={12} />
				</button>
			{/if}
			{#if showSortMenu}
				<button type="button" class="fixed inset-0 z-30 cursor-default" aria-label="Close sort" onclick={() => (showSortMenu = false)}></button>
				<div class="absolute left-0 top-8 z-40 min-w-[176px] rounded-md border border-gray-200 bg-white py-1 shadow-[0_8px_24px_rgba(0,0,0,0.12)]">
					<button
						type="button"
						class="w-full px-3 py-1.5 text-left text-xs text-gray-500 hover:bg-gray-50 disabled:opacity-40"
						disabled={!sort}
						onclick={clearSort}
					>
						Clear sort
					</button>
					<div class="my-1 border-t border-gray-100"></div>
					{#each visibleColumns as column}
						<button
							type="button"
							class="w-full px-3 py-1.5 text-left text-xs text-gray-700 hover:bg-gray-50 inline-flex items-center justify-between gap-2"
							onclick={() => handleHeaderSort(column)}
						>
							<span class="truncate">{column}</span>
							{#if sort?.column === column}
								{#if sort.dir === 'asc'}<ArrowUp size={12} class="text-emerald-600" />{:else}<ArrowDown size={12} class="text-emerald-600" />{/if}
							{/if}
						</button>
					{/each}
				</div>
			{/if}
		</div>
		<div class="flex items-center gap-1 ml-1">
			<button
				type="button"
				class={`h-7 px-2 rounded-md border text-xs ${activeView === 'results' ? 'border-emerald-400 bg-white text-gray-900' : 'border-gray-200 bg-white text-gray-600 hover:bg-gray-50'}`}
				onclick={() => (activeView = 'results')}
			>
				Results
			</button>
			<button
				type="button"
				class={`h-7 px-2 rounded-md border text-xs ${activeView === 'messages' ? 'border-emerald-400 bg-white text-gray-900' : 'border-gray-200 bg-white text-gray-600 hover:bg-gray-50'}`}
				onclick={() => (activeView = 'messages')}
			>
				Messages
			</button>
			<button
				type="button"
				class={`h-7 px-2 rounded-md border text-xs ${activeView === 'explain' ? 'border-emerald-400 bg-white text-gray-900' : 'border-gray-200 bg-white text-gray-600 hover:bg-gray-50'}`}
				onclick={() => (activeView = 'explain')}
			>
				Explain
			</button>
		</div>
		<div class="flex-1"></div>
		<div class="flex items-center gap-2 text-xs text-gray-500">
			<button type="button" class="h-7 w-7 rounded-md border border-gray-200 bg-white hover:bg-gray-50 disabled:opacity-40 inline-flex items-center justify-center" disabled={page <= 1} onclick={() => goToPage(page - 1)} aria-label="Previous page">
				<ChevronLeft size={14} />
			</button>
			<span class="tabular-nums">{page} of {pageCount}</span>
			<button type="button" class="h-7 w-7 rounded-md border border-gray-200 bg-white hover:bg-gray-50 disabled:opacity-40 inline-flex items-center justify-center" disabled={page >= pageCount} onclick={() => goToPage(page + 1)} aria-label="Next page">
				<ChevronRight size={14} />
			</button>
			<select
				class="h-7 px-1.5 rounded-md border border-gray-200 bg-white text-xs text-gray-700 outline-none"
				value={String(pageSize)}
				onchange={(event) => setPageSize(Number((event.currentTarget as HTMLSelectElement).value) as PageSize)}
			>
				{#each PAGE_SIZE_OPTIONS as size}
					<option value={String(size)}>{size} rows</option>
				{/each}
			</select>
			<span class="tabular-nums text-gray-500">{displayTotal} rows</span>
			<span class="hidden sm:inline-flex items-center text-gray-400"><Timer size={12} class="mr-1" />{durationMs}ms</span>
			<button
				type="button"
				onclick={() => {
					userCollapsedPending = false;
					pendingPanelOpen = true;
				}}
				class={`h-7 px-2 rounded-md border text-xs inline-flex items-center gap-1 ${changeCount > 0 ? 'border-amber-300 bg-amber-50 text-amber-800' : 'border-gray-200 bg-white text-gray-600 hover:bg-gray-50'}`}
			>
				Changes
				{#if changeCount > 0}
					<span class="min-w-4 h-4 px-1 rounded-full bg-amber-500 text-white text-[10px] leading-4 text-center">{changeCount}</span>
				{/if}
			</button>
		</div>
	</div>

	{#if sqlError}
		<div
			class="px-4 py-2 text-xs text-red-600 border-b border-red-100 bg-red-50"
		>
			{sqlError}
		</div>
	{/if}
	{#if syncError}
		<div
			class="px-4 py-2 text-xs text-red-600 border-b border-red-100 bg-red-50"
		>
			{syncError}
		</div>
	{/if}

	<div class="flex-1 flex min-h-0">
	<div class="flex-1 overflow-auto bg-white min-h-0">
		{#if activeView === 'messages'}
			<div class="h-full p-4 text-xs text-gray-700 space-y-2">
				{#if sqlError || syncError}
					{#if sqlError}
						<div
							class="rounded border border-red-200 bg-red-50 px-3 py-2 text-red-700"
						>
							<div class="font-semibold mb-1">SQL Error</div>
							<div class="whitespace-pre-wrap">{sqlError}</div>
						</div>
					{/if}
					{#if syncError}
						<div
							class="rounded border border-red-200 bg-red-50 px-3 py-2 text-red-700"
						>
							<div class="font-semibold mb-1">Sync Error</div>
							<div class="whitespace-pre-wrap">{syncError}</div>
						</div>
					{/if}
				{:else}
					<div class="rounded border border-gray-200 bg-gray-50 px-3 py-2">
						Last query executed successfully in {durationMs}ms and returned {displayResult.rowCount}
						rows.
					</div>
				{/if}
			</div>
		{:else if activeView === 'explain'}
			<div class="h-full p-4 space-y-3">
				<div class="text-xs text-gray-600">
					Run `EXPLAIN` for the current query/result source.
				</div>
				<button
					onclick={runExplain}
					disabled={runningExplain || loading}
					class="h-8 px-3 rounded border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-100 disabled:opacity-60"
				>
					{runningExplain ? 'Running EXPLAIN...' : 'Run EXPLAIN'}
				</button>
				{#if refreshSql.trim().length > 0 || resultContext}
					<div class="rounded border border-gray-200 bg-gray-50 p-3">
						<div class="text-[11px] font-semibold text-gray-500 mb-1">
							Source SQL
						</div>
						<pre
							class="font-mono-code text-[11px] text-gray-700 whitespace-pre-wrap break-words">{refreshSql.trim()
								.length > 0
								? refreshSql
								: buildDefaultContextSql()}</pre>
					</div>
				{:else}
					<div class="text-xs text-gray-500">
						Run a query first to generate an explain plan.
					</div>
				{/if}
			</div>
		{:else if loading}
			<div class="min-w-max animate-pulse">
				<table
					class="min-w-full text-left border-collapse text-sm whitespace-nowrap"
				>
					<thead
						class="sticky top-0 bg-gray-50 border-b border-gray-200 shadow-sm z-10"
					>
						<tr>
							{#if editable}
								<th class="px-3 py-2 w-8 border-r border-gray-200"></th>
							{/if}
							<th class="px-4 py-2 w-12 border-r border-gray-200"></th>
							{#if visibleColumns.length > 0}
								{#each visibleColumns as _}
									<th
										class="px-4 py-2 border-r border-gray-200"
										style={`width:${getColumnWidth(_)}px;min-width:${getColumnWidth(_)}px;`}
									>
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
										<td
											class="px-4 py-1.5 border-r border-gray-100"
											style={`width:${getColumnWidth(_)}px;min-width:${getColumnWidth(_)}px;`}
										>
											<div
												class="h-3.5 w-full max-w-[180px] rounded bg-gray-200"
											></div>
										</td>
									{/each}
								{:else}
									{#each Array.from({ length: 6 }) as _}
										<td class="px-4 py-1.5 border-r border-gray-100">
											<div
												class="h-3.5 w-full max-w-[180px] rounded bg-gray-200"
											></div>
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
				<table
					class="min-w-full table-fixed text-left border-collapse text-sm"
				>
					<thead
						class="sticky top-0 bg-gray-50 border-b border-gray-200 shadow-sm z-10"
					>
						<tr>
							{#if editable}
								<th
									class="px-3 py-2 font-medium text-gray-500 border-r border-gray-200 text-xs w-8"
								>
									<input
										type="checkbox"
										onchange={toggleSelectAllVisible}
										checked={pageRows.length > 0 &&
											pageRows.every((row) =>
												selectedRows.has(
													String(row['_querycastle_ctid'] ?? ''),
												),
											)}
									/>
								</th>
							{/if}
							<th
								class="px-4 py-2 font-medium text-gray-500 w-12 border-r border-gray-200 text-xs"
								>#</th
							>
							{#each visibleColumns as column}
								{@const meta = metaFor(column)}
								<th
									class="px-4 py-2 font-medium text-gray-500 border-r border-gray-200 text-xs relative overflow-hidden"
									style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;max-width:${getColumnWidth(column)}px;`}
								>
									<button
										type="button"
										class="flex items-center pr-3 gap-1 min-w-0 w-full text-left hover:text-gray-800"
										onclick={() => handleHeaderSort(column)}
									>
										<ColumnTypeIcon {meta} />
										<span class="truncate">{column}</span>
										{#if sort?.column === column}
											{#if sort.dir === 'asc'}
												<ArrowUp size={11} class="shrink-0 text-emerald-600" />
											{:else}
												<ArrowDown size={11} class="shrink-0 text-emerald-600" />
											{/if}
										{/if}
									</button>
									<button
										type="button"
										class="absolute right-0 top-0 h-full w-2 !cursor-col-resize touch-none"
										style="cursor: col-resize"
										onpointerdown={(event) => beginColumnResize(event, column)}
										aria-label={`Resize ${column} column`}
									></button>
								</th>
							{/each}
						</tr>
						{#if showFilterRow}
							<tr class="bg-white">
								{#if editable}
									<th class="border-r border-gray-200"></th>
								{/if}
								<th class="border-r border-gray-200"></th>
								{#each visibleColumns as column}
									<th
										class="p-1 border-r border-gray-200 overflow-hidden"
										style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;max-width:${getColumnWidth(column)}px;`}
									>
										<input
											value={columnFilters[column] ?? ''}
											oninput={(event) => {
												const value = (event.currentTarget as HTMLInputElement).value;
												columnFilters = { ...columnFilters, [column]: value };
												scheduleFilterBrowse();
											}}
											placeholder="Contains…"
											class="w-full h-7 px-2 rounded border border-gray-200 bg-white text-[11px] font-normal text-gray-700 outline-none focus:border-emerald-500"
										/>
									</th>
								{/each}
							</tr>
						{/if}
					</thead>
					<tbody class="text-gray-700">
						{#if editable && pendingInserts.length > 0}
							{#each pendingInserts as insertRow (insertRow.id)}
								<tr class="border-b border-emerald-200 bg-emerald-50/40">
									<td class="px-3 border-r border-emerald-100 text-gray-400 text-xs">
										<button
											type="button"
											onclick={() => removePendingInsert(insertRow.id)}
											class="text-red-500 hover:text-red-700"
											aria-label="Remove pending insert">×</button
										>
									</td>
									<td class="px-4 border-r border-emerald-100 text-gray-500 text-xs">New</td>
									{#each visibleColumns as column}
										{@const meta = metaFor(column)}
										<td
											class="p-0 border-r border-emerald-100 overflow-hidden whitespace-nowrap"
											style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;max-width:${getColumnWidth(column)}px;`}
										>
											{#if meta?.isAuto || meta?.isPrimary}
												<div class="h-[28px] px-4 flex items-center text-[12px] italic text-gray-400">
													Automatic
												</div>
											{:else}
												<GridCellEditor
													kind={meta?.kind ?? 'text'}
													value={draftFromValue(column, insertRow.values[column])}
													nullable={!meta?.notNull}
													flush={true}
													placeholder={meta?.fk ? `Choose ${meta.fk.referencedTable}…` : ''}
													fkOptions={meta?.fk ? optionsForFk(meta.fk) : []}
													fkLoading={meta?.fk ? isFkLoading(meta.fk) : false}
													onChange={(next) => setInsertValue(insertRow.id, column, next)}
													onSearch={(query) => meta?.fk && scheduleFkSearch(meta.fk, query)}
												/>
											{/if}
										</td>
									{/each}
								</tr>
							{/each}
						{/if}
						{#each pageRows as row, rowIndex (String(row['_querycastle_ctid'] ?? `row-${rowIndex}`))}
							{@const rowId = String(row['_querycastle_ctid'] ?? '')}
							{@const isChecked = selectedRows.has(rowId)}
							{@const isActive = activeRowId === rowId}
							<tr
								class={`group h-8 max-h-8 ${isChecked ? 'relative z-[1] bg-emerald-500/15 [&>td]:border-t [&>td]:border-b [&>td]:border-t-emerald-500 [&>td]:border-b-emerald-500 [&>td:first-child]:border-l [&>td:first-child]:border-l-emerald-500 [&>td:last-child]:border-r-emerald-500' : isActive ? 'relative z-[1] bg-gray-100 [&>td]:border-t [&>td]:border-b [&>td]:border-t-gray-400 [&>td]:border-b-gray-400 [&>td:first-child]:border-l [&>td:first-child]:border-l-gray-400 [&>td:last-child]:border-r-gray-400' : 'border-b border-gray-100 hover:bg-gray-50'}`}
								oncontextmenu={(event) => openRowContextMenu(event, rowId, row)}
								onclick={() => {
									if (rowId) activeRowId = rowId;
								}}
							>
								{#if editable}
									<td class="px-3 py-0 border-r border-gray-100 overflow-hidden">
										<input
											type="checkbox"
											checked={selectedRows.has(rowId)}
											onchange={() => toggleRowSelected(rowId)}
										/>
									</td>
								{/if}
								<td
									class="px-4 py-0 text-gray-400 border-r border-gray-100 text-xs overflow-hidden whitespace-nowrap"
									>{(page - 1) * pageSize + rowIndex + 1}</td
								>
								{#each visibleColumns as column (column)}
									{@const currentValue = getRowValue(row, rowId, column)}
									{@const isEditing =
										editingCell &&
										editingCell.rowId === rowId &&
										editingCell.column === column}
									{@const isRecentlyUpdated = highlightCells.has(
										cellKey(rowId, column),
									)}
									{@const isFkColumn = fkColumns.has(column)}
									{@const meta = metaFor(column)}
									{@const canFollowFk =
										isFkColumn && isFollowableValue(currentValue)}
									{@const isPendingEdit =
										pendingUpdates.get(rowId)?.[column] !== undefined}
									<td
										class={`border-r border-gray-100 text-[12px] overflow-hidden whitespace-nowrap max-w-0 ${editable && !meta?.isAuto && !meta?.isPrimary ? 'cursor-pointer' : ''} ${isEditing ? 'p-0 outline outline-1 -outline-offset-1 outline-emerald-500 bg-white' : 'px-4 py-0'} ${isPendingEdit && !isEditing ? 'bg-amber-50' : ''} ${isRecentlyUpdated && !isEditing && !isPendingEdit ? 'bg-emerald-50/70' : ''}`}
										style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;max-width:${getColumnWidth(column)}px;`}
										onclick={(event) =>
											handleCellClick(
												event,
												rowId,
												column,
												currentValue,
											)}
										title={isEmptyCell(currentValue)
											? canFollowFk
												? 'Open related record (Alt+Click or icon)'
												: undefined
											: displayCellText(currentValue, meta)}
									>
										{#if isEditing}
											<!-- svelte-ignore a11y_click_events_have_key_events -->
											<!-- svelte-ignore a11y_no_static_element_interactions -->
											<div
												class="h-8 w-full overflow-hidden"
												onclick={(event) => event.stopPropagation()}
												onkeydown={(event) => event.stopPropagation()}
											>
											<GridCellEditor
												kind={meta?.kind ?? 'text'}
												value={editDraft}
												nullable={!meta?.notNull}
												flush={true}
												fkOptions={meta?.fk ? optionsForFk(meta.fk) : []}
												fkLoading={meta?.fk ? isFkLoading(meta.fk) : false}
												autofocus={meta?.kind !== 'fk'}
												startOpen={meta?.kind === 'fk'}
												onChange={(next) => (editDraft = next)}
												onCommit={commitEdit}
												onCancel={discardEdit}
												onSearch={(query) => meta?.fk && scheduleFkSearch(meta.fk, query)}
											/>
											</div>
										{:else}
											<div class="flex items-center gap-1 min-w-0 h-8 overflow-hidden">
												{#if isEmptyCell(currentValue)}
													<span class="truncate min-w-0 flex-1 italic text-gray-400">Empty</span>
												{:else if meta?.kind === 'boolean'}
													<span class={`inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-medium shrink-0 ${currentValue === true || currentValue === 'true' || currentValue === 1 || currentValue === '1' ? 'bg-emerald-50 text-emerald-700' : 'bg-gray-100 text-gray-600'}`}>{formatBooleanLabel(currentValue)}</span>
												{:else}
													<span class={`truncate min-w-0 flex-1 block overflow-hidden text-ellipsis ${isFkColumn ? 'font-mono text-[11px]' : ''}`}>{displayCellText(currentValue, meta)}</span>
												{/if}
												{#if canFollowFk}
													<button
														type="button"
														class="shrink-0 rounded p-0.5 text-emerald-600 opacity-0 group-hover:opacity-100 focus:opacity-100 hover:bg-emerald-50 hover:text-emerald-800"
														title="Open related record"
														aria-label="Follow foreign key"
														onclick={(event) => {
															event.stopPropagation();
															const fk = explorer
																? resolveOutgoingRelations(
																		explorer,
																		resultContext,
																		column,
																	)[0]
																: undefined;
															if (fk) startOutgoingFollow(fk, currentValue);
														}}
														onpointerdown={(event) => event.stopPropagation()}
													>
														<ArrowUpRight size={12} />
													</button>
												{/if}
											</div>
										{/if}
									</td>
								{/each}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
	<PendingChangesPane
		open={pendingPanelOpen}
		{changeCount}
		cards={pendingCards}
		sqlPreview={pendingSqlPreview}
		syncing={syncingChanges}
		error={syncError}
		onClose={() => {
			pendingPanelOpen = false;
			userCollapsedPending = true;
		}}
		onClear={resetDraftState}
		onCommit={() => void syncChanges()}
	/>
	</div>

	{#if rowContextMenu}
		{@const outgoing = outgoingActions(rowContextMenu.row, rowContextMenu.rowId)}
		{@const incoming = incomingActions(rowContextMenu.row, rowContextMenu.rowId)}
		<button
			class="fixed inset-0 z-40"
			aria-label="Close row menu"
			onclick={() => {
				rowContextMenu = null;
				relatedSubmenuOpen = false;
			}}
		></button>
		<div
			class="fixed z-50 min-w-[240px] bg-white rounded-md border border-gray-200 shadow-[0_8px_24px_rgba(0,0,0,0.12)] py-1"
			style={`left:${rowContextMenu?.x ?? 0}px;top:${rowContextMenu?.y ?? 0}px;`}
		>
			{#each outgoing as item}
				<button
					type="button"
					disabled={hasPendingChanges}
					onclick={() => startOutgoingFollow(item.fk, item.value)}
					class="w-full px-3 py-1.5 text-left text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50 inline-flex items-center gap-2"
					title={hasPendingChanges
						? 'Save or discard grid edits first'
						: undefined}
				>
					<ArrowUpRight size={14} class="shrink-0 text-emerald-600" />
					<span class="truncate"
						>Open {item.fk.referencedTable} where {item.fk.referencedColumn} =
						{formatFollowValue(item.value)}</span
					>
				</button>
			{/each}
			{#if incoming.length > 6}
				<div class="relative">
					<button
						type="button"
						class="w-full px-3 py-1.5 text-left text-sm text-gray-700 hover:bg-gray-50 inline-flex items-center justify-between gap-2"
						onmouseenter={() => (relatedSubmenuOpen = true)}
						onclick={() => (relatedSubmenuOpen = !relatedSubmenuOpen)}
					>
						Related rows
						<ChevronRight size={14} class="text-gray-400" />
					</button>
					{#if relatedSubmenuOpen}
						<div
							class="absolute left-full top-0 ml-0.5 min-w-[220px] max-h-72 overflow-auto bg-white rounded-md border border-gray-200 shadow-[0_8px_24px_rgba(0,0,0,0.12)] py-1"
						>
							{#each incoming as rel}
								<button
									type="button"
									disabled={hasPendingChanges}
									onclick={() => startIncomingFollow(rel, rel.value)}
									class="w-full px-3 py-1.5 text-left text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50 truncate"
									title={hasPendingChanges
										? 'Save or discard grid edits first'
										: undefined}
								>
									{incomingMenuLabel(rel, incoming)}
								</button>
							{/each}
						</div>
					{/if}
				</div>
			{:else}
				{#each incoming as rel}
					<button
						type="button"
						disabled={hasPendingChanges}
						onclick={() => startIncomingFollow(rel, rel.value)}
						class="w-full px-3 py-1.5 text-left text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50 truncate"
						title={hasPendingChanges
							? 'Save or discard grid edits first'
							: undefined}
					>
						{incomingMenuLabel(rel, incoming)}
					</button>
				{/each}
			{/if}
			{#if editable && (outgoing.length > 0 || incoming.length > 0)}
				<div class="my-1 border-t border-gray-100"></div>
			{/if}
			{#if editable}
				<button
					onclick={() =>
						rowContextMenu && queueDeleteRows([rowContextMenu.rowId])}
					class="w-full px-3 py-1.5 text-left text-sm text-red-600 hover:bg-red-50 inline-flex items-center gap-2"
				>
					<Trash2 size={14} />
					Delete Row
				</button>
			{/if}
		</div>
	{/if}
</div>



