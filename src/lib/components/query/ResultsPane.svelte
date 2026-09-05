<script lang="ts">
	import {
		ArrowDown,
		ArrowUp,
		ArrowUpDown,
		ArrowUpRight,
		ChevronLeft,
		ChevronRight,
		Copy,
		Filter,
		PanelRight,
		Play,
		Plus,
		Table2,
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
	import { buildTableSelect } from '$lib/utils/table-select';
	import { copyTextToClipboard } from '$lib/utils/clipboard';
	import type { RelationHop } from '$lib/utils/workspace';
	import RelationTrail from '$lib/components/query/RelationTrail.svelte';
	import ColumnTypeIcon from '$lib/components/query/ColumnTypeIcon.svelte';
	import GridCellEditor from '$lib/components/query/GridCellEditor.svelte';
	import PendingChangesPane from '$lib/components/query/PendingChangesPane.svelte';
	import RowInspector from '$lib/components/query/RowInspector.svelte';
	import { gridChrome } from '$lib/stores/grid-chrome.svelte';
	import {
		buildPendingChangeCards,
		buildPendingSqlPreview,
		buildRowInspectSql,
		formatApplyResultMessage,
		pendingChangeCount,
	} from '$lib/utils/pending-changes';
	import { toast } from '$lib/stores/toast.svelte';
	import { fly } from 'svelte/transition';
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
		resultKey = '',
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
		resultKey?: string;
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
	let inspectorOpen = $state(false);
	let cellRange = $state<{
		r0: number;
		r1: number;
		c0: number;
		c1: number;
	} | null>(null);
	let rangeDragging = $state(false);
	let rangeAnchor = $state<{ r: number; c: number } | null>(null);
	let rangeCapture: { el: HTMLElement; pointerId: number } | null = null;
	let gridScrollEl = $state<HTMLDivElement | null>(null);
	let gridViewportH = $state(0);
	let gridViewportW = $state(0);
	const GRID_ROW_PX = 32;
	const GRID_HEADER_PX = 32;
	const FILLER_COL_PX = 120;
	const EMPTY_SHEET_COLUMNS = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
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
	let visibleRows = $derived(displayResult.rows);
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
	let pkColumns = $derived(columnMetas.filter((column) => column.isPrimary).map((column) => column.name));
	let pendingSqlPreview = $derived.by(() => {
		if (!resultContext) return '';
		return buildPendingSqlPreview({
			databaseType,
			schema: resultContext.schema,
			table: resultContext.table,
			updates: Array.from(pendingUpdates.entries()).map(([ctid, values]) => ({
				ctid,
				values,
			})),
			deletes: Array.from(pendingDeletes),
			inserts: pendingInserts.map((row) => row.values),
			rows: displayResult.rows,
			deletedSnapshots,
			pkColumns,
		});
	});
	let browseSourceKey = $derived.by(() => {
		const contextKey = resultContext
			? `${resultContext.schema}.${resultContext.table}`
			: '';
		const trailKey = relationTrail
			.map(
				(hop) =>
					`${hop.direction}:${hop.to.schema}.${hop.to.table}:${String(hop.from.value)}`,
			)
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
		canServerBrowse
			? visibleRows
			: filterSortRows(visibleRows, filterList, effectiveSort),
	);
	let displayTotal = $derived(
		canServerBrowse ? totalRowCount : clientPreparedRows.length,
	);
	let pageCount = $derived.by(() => totalPages(displayTotal, pageSize));
	let pageRows = $derived.by(() => {
		if (canServerBrowse) return visibleRows;
		const start = (page - 1) * pageSize;
		return clientPreparedRows.slice(start, start + pageSize);
	});
	let inspectorRow = $derived.by(() => {
		if (pageRows.length === 0) return null;
		if (!activeRowId) return pageRows[0] ?? null;
		return (
			pageRows.find(
				(row) => String(row['_querycastle_ctid'] ?? '') === activeRowId,
			) ??
			pageRows[0] ??
			null
		);
	});
	let inspectorValues = $derived.by(() => {
		if (!inspectorRow || !activeRowId) return {} as Record<string, unknown>;
		return { ...inspectorRow, ...(pendingUpdates.get(activeRowId) ?? {}) };
	});
	let inspectorSql = $derived.by(() => {
		if (!resultContext || !activeRowId) return '';
		return buildRowInspectSql({
			databaseType,
			schema: resultContext.schema,
			table: resultContext.table,
			ctid: activeRowId,
			row: inspectorRow ?? undefined,
			pkColumns,
		});
	});
	let inspectorLabel = $derived.by(() => {
		if (pageRows.length === 0) return 'Row';
		let index = activeRowId
			? pageRows.findIndex(
					(row) => String(row['_querycastle_ctid'] ?? '') === activeRowId,
				)
			: 0;
		if (index < 0) index = 0;
		return `Row #${(page - 1) * pageSize + index + 1}`;
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

	function buildExternalResultSignature(
		context: { schema: string; table: string } | null,
		payload: QueryResultPayload,
		key: string,
	): string {
		if (key) return key;
		const first = payload.rows[0];
		const last = payload.rows[payload.rows.length - 1];
		return [
			context ? `${context.schema}.${context.table}` : '',
			payload.columns.join('\0'),
			String(payload.rowCount),
			String(payload.durationMs),
			String(payload.rows.length),
			first ? String(first['_querycastle_ctid'] ?? '') : '',
			last ? String(last['_querycastle_ctid'] ?? '') : '',
		].join('|');
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
		deletedSnapshots = new Map();
		keepDraftsOnNextResult = false;
		pendingPanelOpen = false;
		userCollapsedPending = false;
	}

	function buildDefaultContextSql() {
		if (!resultContext) return '';
		const firstVisibleColumn = visibleColumns[0];
		return (
			buildTableSelect({
				databaseType,
				explorer,
				schema: resultContext.schema,
				table: resultContext.table,
				orderClause: firstVisibleColumn
					? ` order by ${quoteSqlIdentifier(databaseType, firstVisibleColumn)} asc${databaseType === 'mysql' ? '' : ' nulls last'}`
					: '',
				limit: 100,
			}) ?? ''
		);
	}

	$effect(() => {
		const signature = buildExternalResultSignature(
			resultContext,
			result,
			resultKey,
		);
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

	function fkCacheKey(fk: {
		referencedSchema: string;
		referencedTable: string;
		referencedColumn: string;
	}) {
		return `${fk.referencedSchema}.${fk.referencedTable}.${fk.referencedColumn}`;
	}

	async function ensureFkOptions(
		fk: {
			referencedSchema: string;
			referencedTable: string;
			referencedColumn: string;
			column: string;
		},
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

	function isFkLoading(fk: {
		referencedSchema: string;
		referencedTable: string;
		referencedColumn: string;
	}) {
		const prefix = fkCacheKey(fk);
		for (const key of fkLoadingKeys) {
			if (key === prefix || key.startsWith(`${prefix}::`)) return true;
		}
		return false;
	}

	function optionsForFk(
		fk: {
			referencedSchema: string;
			referencedTable: string;
			referencedColumn: string;
		} | null,
	) {
		if (!fk) return [];
		return fkOptionCache.get(fkCacheKey(fk)) ?? [];
	}

	function draftFromValue(column: string, value: unknown): string {
		const meta = metaFor(column);
		if (isEmptyCell(value)) return '';
		if (meta?.kind === 'boolean') {
			if (value === true || value === 'true' || value === 1 || value === '1')
				return 'true';
			if (value === false || value === 'false' || value === 0 || value === '0')
				return 'false';
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
		const pending = pendingUpdates.get(rowId);
		if (pending && Object.prototype.hasOwnProperty.call(pending, column)) {
			return pending[column];
		}
		return row[column];
	}

	function getRowValueByName(
		row: Record<string, unknown>,
		rowId: string,
		column: string,
	): unknown {
		if (
			Object.prototype.hasOwnProperty.call(row, column) ||
			pendingUpdates.get(rowId)?.[column] !== undefined
		) {
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
		if (!resultContext || !onFollowRelation || !isFollowableValue(value))
			return;
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
		if (!resultContext || !onFollowRelation || !isFollowableValue(value))
			return;
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
			for (const fk of resolveOutgoingRelations(
				explorer,
				resultContext,
				column,
			)) {
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
		const duplicates =
			all.filter((item) => item.table === rel.table).length > 1;
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
		if (rowId) {
			activeRowId = rowId;
		}
		if (editingCell?.rowId === rowId && editingCell?.column === column) return;
		const fks = explorer
			? resolveOutgoingRelations(explorer, resultContext, column)
			: [];
		const canFollow = fks.length > 0 && isFollowableValue(currentValue);
		const modifierClick = event.altKey || event.metaKey || event.ctrlKey;
		if (canFollow && (modifierClick || !editable)) {
			event.preventDefault();
			startOutgoingFollow(fks[0]!, currentValue);
			return;
		}
		if (inspectorOpen) return;
		beginEdit(rowId, column, currentValue);
	}

	function toggleInspector() {
		if (inspectorOpen) {
			inspectorOpen = false;
			return;
		}
		if (!activeRowId) {
			const first = pageRows[0];
			if (first) activeRowId = String(first['_querycastle_ctid'] ?? '');
		}
		if (!activeRowId) return;
		editingCell = null;
		inspectorOpen = true;
	}

	function inspectRow(rowId: string) {
		activeRowId = rowId;
		editingCell = null;
		inspectorOpen = true;
		rowContextMenu = null;
		relatedSubmenuOpen = false;
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
		const column = columnResizeState.column;
		const prevWidth = getColumnWidth(column);
		setColumnWidth(
			column,
			columnResizeState.startWidth + (event.clientX - columnResizeState.startX),
		);
		const dw = getColumnWidth(column) - prevWidth;
		if (dw === 0 || !cellRange) return;
		const overlay = document.getElementById('qc-range-overlay');
		if (!overlay || overlay.hidden) return;
		const colIndex = visibleColumns.indexOf(column);
		if (colIndex < 0) return;
		if (colIndex < cellRange.c0) {
			overlay.style.left = `${(parseFloat(overlay.style.left) || 0) + dw}px`;
		} else if (colIndex <= cellRange.c1) {
			overlay.style.width = `${(parseFloat(overlay.style.width) || 0) + dw}px`;
		}
	}

	function stopColumnResize() {
		if (!columnResizeState) return;
		columnResizeState = null;
		requestAnimationFrame(updateRangeOverlay);
	}

	function beginEdit(rowId: string, column: string, currentValue: unknown) {
		if (inspectorOpen) return;
		if (!editable || column === '_querycastle_ctid') return;
		if (pendingDeletes.has(rowId)) return;
		const meta = metaFor(column);
		if (meta?.isAuto || meta?.isPrimary) return;
		editingCell = { rowId, column };
		editDraft = draftFromValue(column, currentValue);
		if (meta?.fk) void ensureFkOptions(meta.fk);
	}

	function applyCellValue(rowId: string, column: string, nextValue: unknown) {
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
	}

	function commitEdit() {
		if (!editingCell) return;
		applyCellValue(
			editingCell.rowId,
			editingCell.column,
			coerceValue(editDraft, editingCell.column),
		);
		editingCell = null;
		editDraft = '';
	}

	function setInspectorField(column: string, raw: string) {
		if (!activeRowId) return;
		applyCellValue(activeRowId, column, coerceValue(raw, column));
	}

	function discardEdit() {
		editingCell = null;
		editDraft = '';
	}

	function beginCellRange(
		rowIndex: number,
		colIndex: number,
		event: PointerEvent,
	) {
		if ((event.target as HTMLElement).closest('input,button,textarea,select'))
			return;
		rangeAnchor = { r: rowIndex, c: colIndex };
		cellRange = { r0: rowIndex, r1: rowIndex, c0: colIndex, c1: colIndex };
		rangeDragging = true;
		// NOTE: pointer capture is intentionally NOT taken here. Capturing on
		// plain mousedown steals focus from cell editors opened by the same
		// click (e.g. double-click to edit). Capture starts in
		// extendCellRange once an actual drag is underway.
	}

	function extendCellRange(rowIndex: number, colIndex: number, event?: PointerEvent) {
		if (!rangeDragging || !rangeAnchor) return;
		const next = {
			r0: Math.min(rangeAnchor.r, rowIndex),
			r1: Math.max(rangeAnchor.r, rowIndex),
			c0: Math.min(rangeAnchor.c, colIndex),
			c1: Math.max(rangeAnchor.c, colIndex),
		};
		const grew =
			next.r0 !== cellRange?.r0 ||
			next.r1 !== cellRange?.r1 ||
			next.c0 !== cellRange?.c0 ||
			next.c1 !== cellRange?.c1;
		cellRange = next;
		if (grew && event) {
			const target = event.currentTarget as HTMLElement | null;
			try {
				target?.setPointerCapture?.(event.pointerId);
				if (target) rangeCapture = { el: target, pointerId: event.pointerId };
			} catch {
				// Capture is best-effort; range tracking works without it.
			}
		}
	}

	function endCellRange() {
		rangeDragging = false;
		if (rangeCapture) {
			try {
				rangeCapture.el.releasePointerCapture?.(rangeCapture.pointerId);
			} catch {
				// Capture already released; nothing to do.
			}
			rangeCapture = null;
		}
	}

	async function copyGridText(text: string) {
		const ok = await copyTextToClipboard(text);
		if (!ok) toast.error('Copy failed. The document is not focused.');
	}

	function copyCellRange() {
		if (!cellRange) return;
		const cols = visibleColumns.slice(cellRange.c0, cellRange.c1 + 1);
		const rows = pageRows.slice(cellRange.r0, cellRange.r1 + 1);
		const lines = [
			cols.join('\t'),
			...rows.map((row) => {
				const rowId = String(row['_querycastle_ctid'] ?? '');
				return cols
					.map((column) => String(getRowValue(row, rowId, column) ?? ''))
					.join('\t');
			}),
		];
		void copyGridText(lines.join('\n'));
	}

	function copySelectedRows() {
		const rows = pageRows.filter((row) =>
			selectedRows.has(String(row['_querycastle_ctid'] ?? '')),
		);
		if (rows.length === 0) return;
		const lines = [
			visibleColumns.join('\t'),
			...rows.map((row) => {
				const rowId = String(row['_querycastle_ctid'] ?? '');
				return visibleColumns
					.map((column) => String(getRowValue(row, rowId, column) ?? ''))
					.join('\t');
			}),
		];
		void copyGridText(lines.join('\n'));
	}

	function updateRangeOverlay() {
		const overlay = document.getElementById('qc-range-overlay');
		const scroll = gridScrollEl;
		if (!overlay || !scroll || !cellRange) {
			if (overlay) overlay.hidden = true;
			return;
		}
		const start = scroll.querySelector(
			`td.grid-cell[data-r="${cellRange.r0}"][data-c="${cellRange.c0}"]`,
		) as HTMLElement | null;
		const end = scroll.querySelector(
			`td.grid-cell[data-r="${cellRange.r1}"][data-c="${cellRange.c1}"]`,
		) as HTMLElement | null;
		if (!start || !end) {
			overlay.hidden = true;
			return;
		}
		const sRect = start.getBoundingClientRect();
		const eRect = end.getBoundingClientRect();
		const cRect = scroll.getBoundingClientRect();
		overlay.style.top = `${sRect.top - cRect.top + scroll.scrollTop}px`;
		overlay.style.left = `${sRect.left - cRect.left + scroll.scrollLeft}px`;
		overlay.style.width = `${eRect.right - sRect.left}px`;
		overlay.style.height = `${eRect.bottom - sRect.top}px`;
		overlay.hidden = false;
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
				toast.error(`New row is missing ${missing.join(', ')}.`);
				pendingPanelOpen = true;
				return;
			}
		}
		syncingChanges = true;
		try {
			const payload: TableChangesPayload = {
				updates: Array.from(pendingUpdates.entries()).map(([ctid, values]) => ({
					ctid,
					values,
				})),
				deletes: Array.from(pendingDeletes),
				inserts: pendingInserts
					.map((row) =>
						valuesForInsert(
							Object.fromEntries(
								Object.entries(row.values).map(([key, value]) => [
									key,
									isEmptyCell(value) ? '' : String(value),
								]),
							),
							columnMetas,
							sampleValue,
						),
					)
					.filter((row) => Object.keys(row).length > 0),
			};
			const applyResult = await onApplyTableChanges(resultContext, payload);
			toast.success(formatApplyResultMessage(applyResult));
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
			toast.error(error instanceof Error ? error.message : String(error));
		} finally {
			syncingChanges = false;
		}
	}

	function scheduleFkSearch(
		fk: {
			referencedSchema: string;
			referencedTable: string;
			referencedColumn: string;
			column: string;
		},
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

	$effect(() => {
		gridChrome.changeCount = changeCount;
	});

	$effect(() => {
		if (gridChrome.pendingOpenNonce > 0) {
			pendingPanelOpen = true;
			userCollapsedPending = false;
		}
	});

	let inspectSourceKey = $state('');
	$effect(() => {
		if (browseSourceKey === inspectSourceKey) return;
		inspectSourceKey = browseSourceKey;
		const first = pageRows[0];
		if (!first) {
			activeRowId = null;
			inspectorOpen = false;
			return;
		}
		activeRowId = String(first['_querycastle_ctid'] ?? '');
	});

	$effect(() => {
		cellRange;
		pageRows;
		requestAnimationFrame(updateRangeOverlay);
	});

	$effect(() => {
		const onKey = (event: KeyboardEvent) => {
			if (event.key === 'Escape') {
				// GridCellEditor stops keydown propagation while focused, so
				// this only runs when focus is elsewhere: dismiss open menus
				// and commit any orphaned cell edit (same as blur).
				if (rowContextMenu) rowContextMenu = null;
				relatedSubmenuOpen = false;
				if (showSortMenu) showSortMenu = false;
				if (editingCell) commitEdit();
				return;
			}
			if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'c')
				return;
			const target = event.target as HTMLElement | null;
			if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
				return;
			if (cellRange) {
				event.preventDefault();
				copyCellRange();
			} else if (selectedRows.size > 0) {
				event.preventDefault();
				copySelectedRows();
			}
		};
		const onUp = () => endCellRange();
		window.addEventListener('keydown', onKey);
		window.addEventListener('pointerup', onUp);
		return () => {
			window.removeEventListener('keydown', onKey);
			window.removeEventListener('pointerup', onUp);
		};
	});

	$effect(() => {
		const el = gridScrollEl;
		if (!el) return;
		const sync = () => {
			gridViewportH = el.clientHeight;
			gridViewportW = el.clientWidth;
		};
		sync();
		const observer = new ResizeObserver(sync);
		observer.observe(el);
		return () => observer.disconnect();
	});

	let sheetColumns = $derived(
		visibleColumns.length > 0 ? visibleColumns : EMPTY_SHEET_COLUMNS,
	);
	let fillerRowCount = $derived.by(() => {
		const headerH = GRID_HEADER_PX + (showFilterRow ? GRID_ROW_PX : 0);
		const usedRows =
			activeView === 'results' ? pageRows.length + pendingInserts.length : 0;
		const remaining = gridViewportH - headerH - usedRows * GRID_ROW_PX;
		return Math.max(16, Math.ceil(Math.max(0, remaining) / GRID_ROW_PX) + 4);
	});
	let fillerColCount = $derived.by(() => {
		const used =
			(editable ? 32 : 0) +
			sheetColumns.reduce((sum, column) => sum + getColumnWidth(column), 0);
		const remaining = Math.max(0, gridViewportW - used);
		return Math.floor(remaining / FILLER_COL_PX);
	});
</script>

<div class="results-pane flex-1 flex flex-col bg-qc-grid min-w-[320px] min-h-0">
	{#snippet fillerHeader()}
		{#each Array.from({ length: fillerColCount }) as _, i (`fill-h-${i}`)}
			<th
				class="grid-filler"
				style={`width:${FILLER_COL_PX}px;min-width:${FILLER_COL_PX}px;max-width:${FILLER_COL_PX}px;`}
				aria-hidden="true"
			></th>
		{/each}
		<th class="grid-filler grid-filler-tail" aria-hidden="true"></th>
	{/snippet}
	{#snippet fillerCells()}
		{#each Array.from({ length: fillerColCount }) as _, i (`fill-c-${i}`)}
			<td
				class="grid-filler"
				style={`width:${FILLER_COL_PX}px;min-width:${FILLER_COL_PX}px;max-width:${FILLER_COL_PX}px;`}
				aria-hidden="true"
			></td>
		{/each}
		<td class="grid-filler grid-filler-tail" aria-hidden="true"></td>
	{/snippet}
	<RelationTrail
		trail={relationTrail}
		onActivate={(index) => onActivateRelationTrail?.(index)}
	/>
	<div
		class="h-10 px-2 border-b border-qc-border bg-qc-panel shrink-0 flex items-center gap-0.5"
	>
		{#if editable}
			{#if selectedRows.size > 0}
				<button
					type="button"
					onclick={() => queueDeleteRows(Array.from(selectedRows))}
					class="btn-danger h-6 w-[72px] px-2 text-[12px] font-medium inline-flex items-center justify-center gap-1 shrink-0"
					title={`Delete ${selectedRows.size} row${selectedRows.size === 1 ? '' : 's'}`}
				>
					<Trash2 size={12} />Delete
				</button>
			{:else}
				<button
					type="button"
					onclick={startInsertRow}
					disabled={syncingChanges}
					class="btn-primary h-6 w-[72px] px-2 text-[12px] font-medium disabled:opacity-50 inline-flex items-center justify-center gap-1 shrink-0"
				>
					<Plus size={12} />Insert
				</button>
			{/if}
		{/if}
		<button
			type="button"
			onclick={rerunContextQuery}
			disabled={rerunning || loading}
			class="toolbar-icon disabled:opacity-50"
			title={rerunning ? 'Running' : 'Refresh'}
			aria-label="Refresh results"
		>
			<Play size={14} />
		</button>
		<button
			type="button"
			onclick={() => (showFilterRow = !showFilterRow)}
			class={`toolbar-icon ${showFilterRow || hasActiveFilters ? 'is-on' : ''}`}
			title="Filter"
			aria-label="Filter"
			aria-pressed={showFilterRow || hasActiveFilters}
		>
			<Filter size={14} />
		</button>
		{#if hasActiveFilters}
			<button
				type="button"
				onclick={clearFilters}
				class="toolbar-icon"
				title="Clear filters"
				aria-label="Clear filters"
			>
				<X size={13} />
			</button>
		{/if}
		<div class="relative shrink-0 inline-flex items-center">
			<button
				type="button"
				onclick={() => (showSortMenu = !showSortMenu)}
				class={`toolbar-icon ${sort || showSortMenu ? 'is-on' : ''}`}
				title="Sort"
				aria-label="Sort"
				aria-pressed={Boolean(sort) || showSortMenu}
			>
				<ArrowUpDown size={14} />
			</button>
			{#if sort}
				<button
					type="button"
					onclick={clearSort}
					class="toolbar-icon"
					title="Clear sort"
					aria-label="Clear sort"
				>
					<X size={13} />
				</button>
			{/if}
			{#if showSortMenu}
				<button
					type="button"
					class="fixed inset-0 z-30 cursor-default"
					aria-label="Close sort"
					onclick={() => (showSortMenu = false)}
				></button>
				<div
					class="absolute left-0 top-full mt-1 z-40 min-w-[176px] rounded-md border border-qc-border bg-qc-elevated py-1 shadow-[0_8px_24px_rgba(0,0,0,0.24)]"
				>
					<button
						type="button"
						class="w-full px-3 py-1.5 text-left text-xs text-qc-muted hover:bg-qc-hover disabled:opacity-40"
						disabled={!sort}
						onclick={clearSort}
					>
						Clear sort
					</button>
					<div class="my-1 border-t border-qc-border-subtle"></div>
					{#each visibleColumns as column}
						<button
							type="button"
							class="w-full px-3 py-1.5 text-left text-xs text-qc-fg hover:bg-qc-hover inline-flex items-center justify-between gap-2"
							onclick={() => handleHeaderSort(column)}
						>
							<span class="truncate">{column}</span>
							{#if sort?.column === column}
								{#if sort.dir === 'asc'}<ArrowUp
										size={12}
										class="text-qc-cell"
									/>{:else}<ArrowDown size={12} class="text-qc-cell" />{/if}
							{/if}
						</button>
					{/each}
				</div>
			{/if}
		</div>
		{#if selectedRows.size > 0}
			<button
				type="button"
				onclick={copySelectedRows}
				class="toolbar-icon"
				title="Copy selected rows"
				aria-label="Copy selected rows"
			>
				<Copy size={14} />
			</button>
			<button
				type="button"
				onclick={() => (selectedRows = new Set())}
				class="h-7 px-1.5 text-[11px] text-qc-muted hover:text-qc-subtle inline-flex items-center gap-1 shrink-0"
				title="Clear selection"
			>
				{selectedRows.size} selected
				<X size={11} />
			</button>
		{/if}
		<div class="flex-1 min-w-2"></div>
		<div class="flex items-center gap-0.5 text-[12px] shrink-0">
			<button
				type="button"
				class={`h-7 px-2 rounded ${activeView === 'results' ? 'text-qc-fg' : 'text-qc-muted hover:text-qc-subtle'}`}
				onclick={() => (activeView = 'results')}
			>
				Results
			</button>
			<button
				type="button"
				class={`h-7 px-2 rounded ${activeView === 'messages' ? 'text-qc-fg' : 'text-qc-muted hover:text-qc-subtle'}`}
				onclick={() => (activeView = 'messages')}
			>
				Messages
			</button>
			<button
				type="button"
				class={`h-7 px-2 rounded ${activeView === 'explain' ? 'text-qc-fg' : 'text-qc-muted hover:text-qc-subtle'}`}
				onclick={() => (activeView = 'explain')}
			>
				Explain
			</button>
		</div>
		<div class="w-px h-4 bg-qc-border mx-1 shrink-0"></div>
		<div class="flex items-center gap-1 text-[12px] text-qc-muted shrink-0">
			<button
				type="button"
				class="w-6 h-6 rounded flex items-center justify-center hover:bg-qc-hover hover:text-qc-subtle disabled:opacity-40"
				disabled={page <= 1}
				onclick={() => goToPage(page - 1)}
				aria-label="Previous page"
			>
				<ChevronLeft size={14} />
			</button>
			<span class="tabular-nums text-qc-subtle">{page}/{pageCount}</span>
			<button
				type="button"
				class="w-6 h-6 rounded flex items-center justify-center hover:bg-qc-hover hover:text-qc-subtle disabled:opacity-40"
				disabled={page >= pageCount}
				onclick={() => goToPage(page + 1)}
				aria-label="Next page"
			>
				<ChevronRight size={14} />
			</button>
			<select
				class="h-6 rounded border border-qc-border bg-qc-elevated text-[11px] text-qc-subtle px-1 outline-none"
				value={String(pageSize)}
				onchange={(event) =>
					setPageSize(
						Number(
							(event.currentTarget as HTMLSelectElement).value,
						) as PageSize,
					)}
			>
				{#each PAGE_SIZE_OPTIONS as size}
					<option value={String(size)}>{size}</option>
				{/each}
			</select>
			<button
				type="button"
				class={`toolbar-icon ${inspectorOpen ? 'is-on' : ''}`}
				onclick={toggleInspector}
				title={inspectorOpen ? 'Hide row inspector' : 'Show row inspector'}
				aria-label={inspectorOpen ? 'Hide row inspector' : 'Show row inspector'}
				aria-pressed={inspectorOpen}
			>
				<PanelRight size={14} />
			</button>
			<span
				class="inline-flex items-center gap-1 tabular-nums"
				title={`${displayTotal} rows`}
			>
				<Table2 size={12} />
				{displayTotal}
			</span>
			<span
				class="hidden sm:inline-flex items-center gap-1 tabular-nums"
				title={`Ran in ${durationMs}ms`}
			>
				<Timer size={12} />
				{durationMs}ms
			</span>
		</div>
	</div>

	{#if sqlError && activeView !== 'messages'}
		<div
			class="px-4 py-2 text-xs text-qc-danger border-b border-qc-danger/20 bg-qc-danger/10"
		>
			{sqlError}
		</div>
	{/if}

	<div class="flex-1 flex min-h-0">
		<div
			class="flex-1 overflow-auto bg-qc-grid min-h-0 relative"
			bind:this={gridScrollEl}
			onscroll={updateRangeOverlay}
		>
			<div id="qc-range-overlay" class="grid-range-overlay" hidden></div>
			{#if activeView === 'messages'}
				<div
					class="h-full p-4 text-xs text-qc-subtle space-y-2 excel-grid bg-qc-grid"
				>
					{#if sqlError}
						<div
							class="rounded border border-qc-danger/30 bg-qc-danger/10 px-3 py-2 text-qc-danger"
						>
							<div class="font-semibold mb-1">SQL Error</div>
							<div class="whitespace-pre-wrap">{sqlError}</div>
						</div>
					{:else}
						<div
							class="rounded border border-qc-border bg-qc-elevated px-3 py-2"
						>
							Last query executed successfully in {durationMs}ms and returned {displayResult.rowCount}
							rows.
						</div>
					{/if}
				</div>
			{:else if activeView === 'explain'}
				<div class="h-full p-4 space-y-3 bg-qc-grid">
					<div class="text-xs text-qc-muted">
						Run `EXPLAIN` for the current query/result source.
					</div>
					<button
						onclick={runExplain}
						disabled={runningExplain || loading}
						class="h-8 px-3 rounded border border-qc-border bg-qc-elevated text-xs text-qc-subtle hover:bg-qc-hover disabled:opacity-60"
					>
						{runningExplain ? 'Running EXPLAIN...' : 'Run EXPLAIN'}
					</button>
					{#if refreshSql.trim().length > 0 || resultContext}
						<div class="rounded border border-qc-border bg-qc-elevated p-3">
							<div class="text-[11px] font-semibold text-qc-muted mb-1">
								Source SQL
							</div>
							<pre
								class="font-mono text-[11px] text-qc-data whitespace-pre-wrap break-words">{refreshSql.trim()
									.length > 0
									? refreshSql
									: buildDefaultContextSql()}</pre>
						</div>
					{:else}
						<div class="text-xs text-qc-muted">
							Run a query first to generate an explain plan.
						</div>
					{/if}
				</div>
			{:else if loading}
				<div class="min-w-full min-h-full">
					<table
						class="excel-grid min-w-full text-left text-sm whitespace-nowrap"
					>
						<thead class="bg-qc-grid z-10">
							<tr>
								{#if editable}
									<th class="qc-select-col"></th>
								{/if}
								{#each sheetColumns as column}
									<th
										class="px-4 py-2"
										style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;`}
									>
										<div class="h-3.5 w-24 rounded bg-qc-hover"></div>
									</th>
								{/each}
								{@render fillerHeader()}
							</tr>
						</thead>
						<tbody>
							{#each skeletonRows as _}
								<tr class="h-8">
									{#if editable}
										<td class="qc-select-col">
											<div class="mx-auto h-3.5 w-3.5 rounded bg-qc-hover"></div>
										</td>
									{/if}
									{#each sheetColumns as column}
										<td
											class="px-4 py-1.5"
											style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;`}
										>
											<div
												class="h-3.5 w-full max-w-[180px] rounded bg-qc-hover"
											></div>
										</td>
									{/each}
									{@render fillerCells()}
								</tr>
							{/each}
							{#each Array.from({ length: fillerRowCount }) as _, i (i)}
								<tr class="h-8 max-h-8 pointer-events-none">
									{#if editable}
										<td class="qc-select-col"></td>
									{/if}
									{#each sheetColumns as column}
										<td
											style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;max-width:${getColumnWidth(column)}px;`}
										></td>
									{/each}
									{@render fillerCells()}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{:else}
				<div class="min-w-full min-h-full">
					<table class="excel-grid min-w-full table-fixed text-left text-sm">
						<colgroup>
							{#if editable}
								<col class="qc-select-col" />
							{/if}
							{#each sheetColumns as column}
								<col
									style={`width:${getColumnWidth(column)}px`}
								/>
							{/each}
						</colgroup>
						<thead class="bg-qc-grid z-10">
							<tr>
								{#if editable}
									<th class="qc-select-col">
										<div
											class="flex h-8 w-full items-center justify-center"
										>
											<input
												type="checkbox"
												class="qc-check"
												onchange={toggleSelectAllVisible}
												checked={pageRows.length > 0 &&
													pageRows.every((row) =>
														selectedRows.has(
															String(row['_querycastle_ctid'] ?? ''),
														),
													)}
											/>
										</div>
									</th>
								{/if}
								{#each sheetColumns as column}
									{@const meta = metaFor(column)}
									<th
										class="px-2.5 py-1.5 font-medium text-qc-subtle text-[11px] relative overflow-hidden"
										style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;max-width:${getColumnWidth(column)}px;`}
									>
										<button
											type="button"
											class="flex items-center pr-3 gap-1 min-w-0 w-full text-left hover:text-qc-fg"
											onclick={() => handleHeaderSort(column)}
										>
											{#if visibleColumns.includes(column)}
												<span class="truncate">{column}</span>
												<ColumnTypeIcon {meta} />
												{#if sort?.column === column}
													{#if sort.dir === 'asc'}
														<ArrowUp size={11} class="shrink-0 text-qc-cell" />
													{:else}
														<ArrowDown
															size={11}
															class="shrink-0 text-qc-cell"
														/>
													{/if}
												{/if}
											{:else}
												<span class="truncate text-qc-muted">{column}</span>
											{/if}
										</button>
										<button
											type="button"
											class="absolute right-0 top-0 h-full w-2 !cursor-col-resize touch-none"
											style="cursor: col-resize"
											onpointerdown={(event) =>
												beginColumnResize(event, column)}
											aria-label={`Resize ${column} column`}
										></button>
									</th>
								{/each}
								{@render fillerHeader()}
							</tr>
							{#if showFilterRow}
								<tr class="bg-qc-grid">
									{#if editable}
										<th class="qc-select-col"></th>
									{/if}
									{#each sheetColumns as column}
										<th
											class="p-1 overflow-hidden"
											style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;max-width:${getColumnWidth(column)}px;`}
										>
											{#if visibleColumns.includes(column)}
												<input
													value={columnFilters[column] ?? ''}
													oninput={(event) => {
														const value = (
															event.currentTarget as HTMLInputElement
														).value;
														columnFilters = {
															...columnFilters,
															[column]: value,
														};
														scheduleFilterBrowse();
													}}
													placeholder="Contains…"
													class="w-full h-7 px-2 rounded border border-qc-border bg-qc-bg text-[11px] font-normal text-qc-fg outline-none focus:border-qc-focus-border"
												/>
											{/if}
										</th>
									{/each}
									{@render fillerHeader()}
								</tr>
							{/if}
						</thead>
						<tbody class="font-mono text-[12px] tabular-nums text-qc-data">
							{#each editable ? pendingInserts : [] as insertRow (insertRow.id)}
								<tr
									class="row-pending-insert h-8 max-h-8"
									in:fly|local={{ y: -8, duration: 220 }}
									out:fly|local={{ y: -8, duration: 180 }}
								>
									<td class="qc-select-col">
										<div
											class="flex h-8 w-full items-center justify-center"
										>
											<button
												type="button"
												onclick={() =>
													removePendingInsert(insertRow.id)}
												class="flex h-6 w-6 items-center justify-center rounded text-qc-muted hover:bg-qc-hover hover:text-qc-danger"
												aria-label="Remove pending insert"
											>
												<X size={12} />
											</button>
										</div>
									</td>
									{#each visibleColumns as column}
										{@const meta = metaFor(column)}
										<td
											class="p-0 overflow-hidden whitespace-nowrap"
											style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;max-width:${getColumnWidth(column)}px;`}
										>
											{#if meta?.isAuto || meta?.isPrimary}
												<div
													class="h-[28px] px-4 flex items-center text-[12px] italic text-qc-muted"
												>
													Automatic
												</div>
											{:else}
												<GridCellEditor
													kind={meta?.kind ?? 'text'}
													value={draftFromValue(
														column,
														insertRow.values[column],
													)}
													nullable={!meta?.notNull}
													flush={true}
													placeholder={meta?.fk
														? `Choose ${meta.fk.referencedTable}…`
														: ''}
													fkOptions={meta?.fk ? optionsForFk(meta.fk) : []}
													fkLoading={meta?.fk ? isFkLoading(meta.fk) : false}
													onChange={(next) =>
														setInsertValue(insertRow.id, column, next)}
													onSearch={(query) =>
														meta?.fk && scheduleFkSearch(meta.fk, query)}
												/>
											{/if}
										</td>
									{/each}
									{@render fillerCells()}
								</tr>
							{/each}
							{#each pageRows as row, rowIndex (String(row['_querycastle_ctid'] ?? `row-${rowIndex}`))}
								{@const rowId = String(row['_querycastle_ctid'] ?? '')}
								{@const isChecked = selectedRows.has(rowId)}
								{@const isActive = activeRowId === rowId}
								{@const isPendingDelete = pendingDeletes.has(rowId)}
								<tr
									class={`group table-row h-8 max-h-8 transition-colors duration-200 ${isPendingDelete ? 'row-pending-delete' : isChecked ? 'row-selected' : isActive ? 'row-current' : ''}`}
									out:fly|local={{ y: -6, duration: 180 }}
									oncontextmenu={(event) =>
										openRowContextMenu(event, rowId, row)}
									onclick={() => {
										if (rowId) activeRowId = rowId;
									}}
								>
									{#if editable}
										<td class="qc-select-col">
											<div
												class="flex h-8 w-full items-center justify-center"
											>
												<input
													type="checkbox"
													class="qc-check"
													checked={selectedRows.has(rowId)}
													onchange={() => toggleRowSelected(rowId)}
												/>
											</div>
										</td>
									{/if}
									{#each visibleColumns as column, colIndex (column)}
										{@const currentValue = getRowValue(row, rowId, column)}
										{@const isEditing =
											editingCell &&
											editingCell.rowId === rowId &&
											editingCell.column === column}
										{@const isFkColumn = fkColumns.has(column)}
										{@const meta = metaFor(column)}
										{@const canFollowFk =
											isFkColumn && isFollowableValue(currentValue)}
										{@const isPendingEdit =
											pendingUpdates.get(rowId)?.[column] !== undefined}
										<td
											class={`grid-cell text-[12px] overflow-hidden whitespace-nowrap max-w-0 font-mono tabular-nums text-qc-data ${editable && !meta?.isAuto && !meta?.isPrimary && !isPendingDelete ? 'cursor-cell' : ''} ${isEditing ? 'p-0 outline outline-1 -outline-offset-1 outline-qc-cell bg-qc-bg' : 'px-2.5 py-0'} ${isPendingEdit && !isEditing && !isPendingDelete ? 'cell-dirty' : ''} ${meta?.kind === 'number' ? 'text-right' : ''}`}
											style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;max-width:${getColumnWidth(column)}px;`}
											data-r={rowIndex}
											data-c={colIndex}
											onpointerdown={(event) =>
												beginCellRange(rowIndex, colIndex, event)}
											onpointerenter={(event) =>
												extendCellRange(rowIndex, colIndex, event)}
											onclick={(event) =>
												handleCellClick(event, rowId, column, currentValue)}
											title={isPendingEdit
												? `${displayCellText(row[column], meta)} → ${displayCellText(currentValue, meta)}`
												: isEmptyCell(currentValue)
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
														onSearch={(query) =>
															meta?.fk && scheduleFkSearch(meta.fk, query)}
													/>
												</div>
											{:else}
												<div
													class="flex items-center gap-1 min-w-0 h-8 overflow-hidden"
												>
													{#if isEmptyCell(currentValue)}
														<span
															class="truncate min-w-0 flex-1 italic text-qc-muted"
															>Empty</span
														>
													{:else if meta?.kind === 'boolean'}
														<span
															class={`inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-medium shrink-0 ${currentValue === true || currentValue === 'true' || currentValue === 1 || currentValue === '1' ? 'bg-qc-hover text-qc-fg' : 'bg-qc-elevated text-qc-muted'}`}
															>{formatBooleanLabel(currentValue)}</span
														>
													{:else}
														<span
															class={`truncate min-w-0 flex-1 block overflow-hidden text-ellipsis ${isFkColumn ? 'font-mono text-[11px]' : ''} ${meta?.kind === 'number' ? 'text-right' : ''}`}
															>{displayCellText(currentValue, meta)}</span
														>
													{/if}
													{#if canFollowFk}
														<button
															type="button"
															class="shrink-0 rounded p-0.5 text-qc-cell opacity-0 group-hover:opacity-100 focus:opacity-100 hover:bg-qc-hover"
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
									{@render fillerCells()}
								</tr>
							{/each}
							{#each Array.from( { length: fillerRowCount }, ) as _, i (`fill-${i}`)}
								<tr class="h-8 max-h-8 pointer-events-none">
									{#if editable}
										<td class="qc-select-col"></td>
									{/if}
									{#each sheetColumns as column}
										<td
											style={`width:${getColumnWidth(column)}px;min-width:${getColumnWidth(column)}px;max-width:${getColumnWidth(column)}px;`}
										></td>
									{/each}
									{@render fillerCells()}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</div>
		<RowInspector
			open={inspectorOpen && !!inspectorRow}
			rowLabel={inspectorLabel}
			columns={visibleColumns}
			values={inspectorValues}
			metas={columnMetas}
			sqlPreview={inspectorSql}
			{editable}
			onClose={() => (inspectorOpen = false)}
			onFieldChange={setInspectorField}
			onDelete={() => {
				if (activeRowId) queueDeleteRows([activeRowId]);
			}}
		/>
		<PendingChangesPane
			open={pendingPanelOpen && (changeCount > 0 || syncingChanges)}
			{changeCount}
			cards={pendingCards}
			sqlPreview={pendingSqlPreview}
			syncing={syncingChanges}
			onClose={() => {
				pendingPanelOpen = false;
				userCollapsedPending = true;
			}}
			onClear={resetDraftState}
			onCommit={() => void syncChanges()}
		/>
	</div>

	{#if rowContextMenu}
		{@const outgoing = outgoingActions(
			rowContextMenu.row,
			rowContextMenu.rowId,
		)}
		{@const incoming = incomingActions(
			rowContextMenu.row,
			rowContextMenu.rowId,
		)}
		<button
			class="fixed inset-0 z-40"
			aria-label="Close row menu"
			onclick={() => {
				rowContextMenu = null;
				relatedSubmenuOpen = false;
			}}
		></button>
		<div
			class="ctx-menu fixed z-50"
			style={`left:${rowContextMenu?.x ?? 0}px;top:${rowContextMenu?.y ?? 0}px;`}
		>
			<button
				type="button"
				onclick={() => inspectRow(rowContextMenu?.rowId ?? '')}
				class="ctx-item"
			>
				<PanelRight size={12} class="shrink-0 text-qc-muted" />
				Inspect row
			</button>
			<div class="ctx-separator"></div>
			{#each outgoing as item}
				<button
					type="button"
					disabled={hasPendingChanges}
					onclick={() => startOutgoingFollow(item.fk, item.value)}
					class="ctx-item"
					title={hasPendingChanges
						? 'Save or discard grid edits first'
						: undefined}
				>
					<ArrowUpRight size={12} class="shrink-0 text-qc-cell" />
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
						class="ctx-item justify-between"
						onmouseenter={() => (relatedSubmenuOpen = true)}
						onclick={() => (relatedSubmenuOpen = !relatedSubmenuOpen)}
					>
						Related rows
						<ChevronRight size={12} class="text-qc-muted" />
					</button>
					{#if relatedSubmenuOpen}
						<div
							class="ctx-menu absolute left-full top-0 ml-0.5 max-h-72 overflow-auto"
						>
							{#each incoming as rel}
								<button
									type="button"
									disabled={hasPendingChanges}
									onclick={() => startIncomingFollow(rel, rel.value)}
									class="ctx-item"
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
						class="ctx-item"
						title={hasPendingChanges
							? 'Save or discard grid edits first'
							: undefined}
					>
						{incomingMenuLabel(rel, incoming)}
					</button>
				{/each}
			{/if}
			{#if editable && (outgoing.length > 0 || incoming.length > 0)}
				<div class="ctx-separator"></div>
			{/if}
			{#if editable}
				<button
					onclick={() =>
						rowContextMenu && queueDeleteRows([rowContextMenu.rowId])}
					class="ctx-item ctx-item-danger"
				>
					<Trash2 size={12} />
					Delete Row
				</button>
			{/if}
		</div>
	{/if}
</div>
