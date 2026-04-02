<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import type {
		ApplyTableChangesResult,
		ConnectionInput,
		ConnectionStatus,
		DatabaseExplorer,
		QueryResultPayload,
		TableChangesPayload,
	} from './lib/rpc';
	import { rpc } from './lib/rpc-client';
	import type { QueryHistoryItem, SavedQueryItem } from './lib/types';
	import SqlEditor from './components/SqlEditor.svelte';
	import ExplorerSidebar from './components/ExplorerSidebar.svelte';
	import ResultsPane from './components/ResultsPane.svelte';
	import ConnectionHub from './components/ConnectionHub.svelte';
	import ConnectionModal from './components/ConnectionModal.svelte';
	import { format as formatSql } from 'sql-formatter';
	import {
		Database,
		Plus,
		Unplug,
		Home,
		FolderKanban,
		FileCode2,
		ChevronRight,
		Star,
		Trash2,
	} from '@lucide/svelte';

	const SAVED_CONNECTIONS_KEY = 'querycastle.savedConnections.v1';
	const QUERY_TABS_KEY = 'querycastle.queryTabs.v2';
	const QUERY_FAVORITES_KEY = 'querycastle.queryFavorites.v1';
	const QUERY_HISTORY_KEY = 'querycastle.queryHistory.v1';

	type TabKind = 'query' | 'data';
	type MainView = 'sql' | 'saved_queries' | 'last_queries';
	type WorkspaceTab = {
		id: string;
		title: string;
		kind: TabKind;
		sql: string;
		lastRunSql: string;
		result: QueryResultPayload;
		sqlError: string;
		resultContext: { schema: string; table: string } | null;
	};
	type TabContextMenu = { x: number; y: number; tabId: string } | null;
	type EditableQueryPlan = {
		sql: string;
		context: { schema: string; table: string };
	};

	type TableAction =
		| 'view_data'
		| 'view_structure'
		| 'export_file'
		| 'import_file'
		| 'copy_name'
		| 'hide'
		| 'sql_create'
		| 'rename'
		| 'drop'
		| 'truncate'
		| 'duplicate';
	type SchemaAction = 'copy_name' | 'copy_quoted_name' | 'sql_list_tables';

	function emptyResult(): QueryResultPayload {
		return { columns: [], rows: [], rowCount: 0, durationMs: 0 };
	}

	const DEFAULT_TAB: WorkspaceTab = {
		id: crypto.randomUUID(),
		title: 'Query 1',
		kind: 'query',
		sql: 'SELECT 1;',
		lastRunSql: '',
		result: emptyResult(),
		sqlError: '',
		resultContext: null,
	};

	let connectionStatus = $state<ConnectionStatus>({
		connected: false,
		name: 'Disconnected',
		host: '',
		port: 5432,
		database: '',
		user: '',
		serverVersion: null,
	});

	let explorerSearch = $state('');
	let connectionSearch = $state('');
	let tabs = $state<WorkspaceTab[]>([]);
	let activeTabId = $state('');
	let queryHistory = $state<QueryHistoryItem[]>([]);
	let queryFavorites = $state<SavedQueryItem[]>([]);
	let explorer = $state<DatabaseExplorer | null>(null);
	let isExplorerLoading = $state(false);
	let databases = $state<string[]>([]);
	let expandedSchemas = $state(new Set<string>());
	let expandedTables = $state(new Set<string>());
	let savedConnections = $state<ConnectionInput[]>([]);

	let showConnectionModal = $state(false);
	let editingConnectionName = $state<string | null>(null);
	let isTestingConnection = $state(false);
	let isConnecting = $state(false);
	let isRunningQuery = $state(false);
	let testConnectionMessage = $state('');
	let testConnectionOk = $state(false);
	let globalError = $state('');
	let connectingName = $state<string | null>(null);
	let showRenameModal = $state(false);
	let renameTarget = $state<{ schema: string; table: string } | null>(null);
	let renameValue = $state('');
	let tabContextMenu = $state<TabContextMenu>(null);

	let queryDurationMs = $state(0);
	let connectionInputMode = $state<'fields' | 'string'>('fields');
	let connectionStringInput = $state('');
	let mainView = $state<MainView>('sql');
	let sqlSplitContainer = $state<HTMLDivElement | null>(null);
	let resultsPaneHeight = $state(320);
	let resizingResults = $state(false);

	let connectionForm = $state<ConnectionInput>({
		name: 'local_pg',
		host: 'localhost',
		port: 5432,
		user: 'postgres',
		password: '',
		database: 'postgres',
		ssl: false,
	});

	let activeTab = $derived.by(
		() => tabs.find((tab) => tab.id === activeTabId) ?? null,
	);
	let activeConnectionKey = $derived.by(() => {
		if (!connectionStatus.connected) return 'disconnected';
		return `${connectionStatus.host}:${connectionStatus.port}/${connectionStatus.database}/${connectionStatus.user}`;
	});
	let sqlCompletions = $derived.by(() => {
		if (!explorer) return [];
		const items = new Set<string>();
		for (const schema of explorer.schemas) {
			items.add(schema.name);
			for (const table of schema.tables) {
				items.add(table.name);
				items.add(`${schema.name}.${table.name}`);
				for (const column of table.columns) items.add(column.name);
			}
		}
		return Array.from(items);
	});
	let favoritesForConnection = $derived.by(() =>
		connectionStatus.connected
			? queryFavorites.filter(
					(item) => item.connectionKey === activeConnectionKey,
				)
			: queryFavorites,
	);
	let historyForConnection = $derived.by(() =>
		connectionStatus.connected
			? queryHistory.filter(
					(item) =>
						!item.connectionKey || item.connectionKey === activeConnectionKey,
				)
			: queryHistory,
	);
	let selectedSavedQueryId = $state('');
	let selectedHistoryIndex = $state(0);
	let selectedSavedQuery = $derived.by(
		() =>
			favoritesForConnection.find((item) => item.id === selectedSavedQueryId) ??
			favoritesForConnection[0] ??
			null,
	);
	let selectedHistoryQuery = $derived.by(
		() =>
			historyForConnection[selectedHistoryIndex] ??
			historyForConnection[0] ??
			null,
	);
	let moveListener: ((event: PointerEvent) => void) | null = null;
	let upListener: (() => void) | null = null;

	$effect(() => {
		if (mainView !== 'saved_queries') return;
		if (favoritesForConnection.length === 0) {
			selectedSavedQueryId = '';
			return;
		}
		if (
			!favoritesForConnection.some((item) => item.id === selectedSavedQueryId)
		) {
			selectedSavedQueryId = favoritesForConnection[0].id;
		}
	});

	$effect(() => {
		if (mainView !== 'last_queries') return;
		if (historyForConnection.length === 0) {
			selectedHistoryIndex = 0;
			return;
		}
		if (selectedHistoryIndex >= historyForConnection.length) {
			selectedHistoryIndex = 0;
		}
	});

	function quoteIdent(value: string) {
		return `"${value.replaceAll('"', '""')}"`;
	}

	function unquoteIdent(value: string) {
		const trimmed = value.trim();
		if (trimmed.startsWith('"') && trimmed.endsWith('"')) {
			return trimmed.slice(1, -1).replaceAll('""', '"');
		}
		return trimmed.toLowerCase();
	}

	function resolveTableSchema(tableName: string): string | null {
		if (!explorer) return null;
		const exactMatches: string[] = [];
		const lowerMatches: string[] = [];
		for (const schema of explorer.schemas) {
			for (const table of schema.tables) {
				if (table.name === tableName) exactMatches.push(schema.name);
				if (table.name.toLowerCase() === tableName.toLowerCase())
					lowerMatches.push(schema.name);
			}
		}
		if (exactMatches.length === 1) return exactMatches[0];
		if (exactMatches.length > 1)
			return exactMatches.includes('public') ? 'public' : null;
		if (lowerMatches.length === 1) return lowerMatches[0];
		if (lowerMatches.length > 1)
			return lowerMatches.includes('public') ? 'public' : null;
		return null;
	}

	function resolvePreferredOrderColumn(
		schema: string,
		table: string,
	): string | null {
		const tableMeta = explorer?.schemas
			.find((item) => item.name === schema)
			?.tables.find((item) => item.name === table);
		return tableMeta?.columns[0]?.name ?? null;
	}

	function tryBuildEditableQuery(sql: string): EditableQueryPlan | null {
		const cleaned = sql.trim().replace(/;+\s*$/, '');
		if (!/^select\b/i.test(cleaned)) return null;

		const selectMatch = cleaned.match(
			/^\s*select\s+([\s\S]+?)\s+from\s+([\s\S]+)$/i,
		);
		if (!selectMatch) return null;

		const selectPart = selectMatch[1];
		const fromAndTail = selectMatch[2];
		if (/\bdistinct\b/i.test(selectPart)) return null;
		if (
			/\b(with|join|group\s+by|having|union|intersect|except)\b/i.test(
				fromAndTail,
			)
		)
			return null;

		const tableMatch = fromAndTail.match(
			/^\s*((?:"(?:[^"]|"")+"|[A-Za-z_][A-Za-z0-9_$]*)(?:\s*\.\s*(?:"(?:[^"]|"")+"|[A-Za-z_][A-Za-z0-9_$]*))?)([\s\S]*)$/s,
		);
		if (!tableMatch) return null;

		const tableRef = tableMatch[1];
		const tail = tableMatch[2] ?? '';
		if (/\bfrom\b/i.test(tail)) return null;
		if (/^\s*,/.test(tail)) return null;

		const qualifiedIdMatch = tableRef.match(
			/^\s*(?:"((?:[^"]|"")*)"|([A-Za-z_][A-Za-z0-9_$]*))\s*\.\s*(?:"((?:[^"]|"")*)"|([A-Za-z_][A-Za-z0-9_$]*))\s*$/s,
		);
		const unqualifiedIdMatch = tableRef.match(
			/^\s*(?:"((?:[^"]|"")*)"|([A-Za-z_][A-Za-z0-9_$]*))\s*$/s,
		);

		let contextSchema = '';
		let contextTable = '';
		if (qualifiedIdMatch) {
			const rawSchema = qualifiedIdMatch[1]
				? `"${qualifiedIdMatch[1]}"`
				: (qualifiedIdMatch[2] ?? '');
			const rawTable = qualifiedIdMatch[3]
				? `"${qualifiedIdMatch[3]}"`
				: (qualifiedIdMatch[4] ?? '');
			if (!rawSchema || !rawTable) return null;
			contextSchema = unquoteIdent(rawSchema);
			contextTable = unquoteIdent(rawTable);
		} else if (unqualifiedIdMatch) {
			const rawTable = unqualifiedIdMatch[1]
				? `"${unqualifiedIdMatch[1]}"`
				: (unqualifiedIdMatch[2] ?? '');
			const tableName = unquoteIdent(rawTable);
			const resolvedSchema = resolveTableSchema(tableName);
			if (!resolvedSchema) return null;
			contextSchema = resolvedSchema;
			contextTable = tableName;
		} else {
			return null;
		}

		let effectiveTail = tail;
		if (!/\border\s+by\b/i.test(effectiveTail)) {
			const preferredOrderColumn = resolvePreferredOrderColumn(
				contextSchema,
				contextTable,
			);
			const orderByClause = preferredOrderColumn
				? ` order by ${quoteIdent(preferredOrderColumn)} asc nulls last`
				: ' order by ctid asc';
			const limitLikeMatch = effectiveTail.match(/\b(limit|offset|fetch)\b/i);
			if (limitLikeMatch && limitLikeMatch.index !== undefined) {
				const insertAt = limitLikeMatch.index;
				effectiveTail = `${effectiveTail.slice(0, insertAt)}${orderByClause} ${effectiveTail.slice(insertAt)}`;
			} else {
				effectiveTail = `${effectiveTail}${orderByClause}`;
			}
		}

		return {
			sql: `select ctid::text as _querycastle_ctid, ${selectPart} from ${tableRef}${effectiveTail};`,
			context: {
				schema: contextSchema,
				table: contextTable,
			},
		};
	}

	function ensureTab() {
		if (tabs.length > 0) return;
		const nextTab = { ...DEFAULT_TAB, id: crypto.randomUUID() };
		tabs = [nextTab];
		activeTabId = nextTab.id;
	}

	function loadSavedConnections() {
		try {
			const raw = localStorage.getItem(SAVED_CONNECTIONS_KEY);
			savedConnections = raw ? (JSON.parse(raw) as ConnectionInput[]) : [];
		} catch {
			savedConnections = [];
		}
	}

	function loadQueryTabs() {
		try {
			const raw = localStorage.getItem(QUERY_TABS_KEY);
			if (!raw) return;
			const parsed = JSON.parse(raw) as Array<Partial<WorkspaceTab>>;
			if (!Array.isArray(parsed)) return;
			const restored = parsed
				.filter(
					(item) =>
						typeof item.id === 'string' && typeof item.title === 'string',
				)
				.map((item) => ({
					id: item.id!,
					title: item.title!,
					kind: (item.kind === 'data' ? 'data' : 'query') as TabKind,
					sql: typeof item.sql === 'string' ? item.sql : '',
					lastRunSql: '',
					result: emptyResult(),
					sqlError: '',
					resultContext: item.resultContext ?? null,
				}));
			if (restored.length === 0) return;
			tabs = restored;
			activeTabId = restored[0].id;
		} catch {
			// ignore
		}
	}

	function loadQueryFavorites() {
		try {
			const raw = localStorage.getItem(QUERY_FAVORITES_KEY);
			if (!raw) {
				queryFavorites = [];
				return;
			}
			const parsed = JSON.parse(raw) as Array<Partial<SavedQueryItem>>;
			if (!Array.isArray(parsed)) {
				queryFavorites = [];
				return;
			}
			queryFavorites = parsed
				.filter(
					(item) =>
						typeof item.id === 'string' &&
						typeof item.sql === 'string' &&
						typeof item.connectionKey === 'string',
				)
				.map((item) => ({
					id: item.id!,
					title:
						typeof item.title === 'string' && item.title.trim().length > 0
							? item.title
							: 'Saved Query',
					sql: item.sql!,
					createdAt:
						typeof item.createdAt === 'number' ? item.createdAt : Date.now(),
					connectionKey: item.connectionKey!,
				}));
		} catch {
			queryFavorites = [];
		}
	}

	function loadQueryHistory() {
		try {
			const raw = localStorage.getItem(QUERY_HISTORY_KEY);
			if (!raw) {
				queryHistory = [];
				return;
			}
			const parsed = JSON.parse(raw) as Array<Partial<QueryHistoryItem>>;
			if (!Array.isArray(parsed)) {
				queryHistory = [];
				return;
			}
			queryHistory = parsed
				.filter(
					(item) =>
						typeof item.time === 'string' &&
						typeof item.sql === 'string' &&
						typeof item.durationMs === 'number' &&
						typeof item.success === 'boolean',
				)
				.map((item) => ({
					time: item.time!,
					sql: item.sql!,
					durationMs: item.durationMs!,
					success: item.success!,
					error: typeof item.error === 'string' ? item.error : undefined,
					connectionKey:
						typeof item.connectionKey === 'string'
							? item.connectionKey
							: undefined,
				}));
		} catch {
			queryHistory = [];
		}
	}

	function persistSavedConnections() {
		localStorage.setItem(
			SAVED_CONNECTIONS_KEY,
			JSON.stringify(savedConnections),
		);
	}

	function upsertSavedConnection(connection: ConnectionInput) {
		const index = savedConnections.findIndex(
			(item) => item.name === connection.name,
		);
		if (index === -1) savedConnections = [connection, ...savedConnections];
		else {
			savedConnections[index] = connection;
			savedConnections = [...savedConnections];
		}
		persistSavedConnections();
	}

	function removeSavedConnection(name: string) {
		savedConnections = savedConnections.filter((item) => item.name !== name);
		persistSavedConnections();
	}

	function persistQueryFavorites() {
		localStorage.setItem(QUERY_FAVORITES_KEY, JSON.stringify(queryFavorites));
	}

	function persistQueryHistory() {
		localStorage.setItem(QUERY_HISTORY_KEY, JSON.stringify(queryHistory));
	}

	function deriveFavoriteTitle(sql: string) {
		const firstLine = sql
			.split('\n')
			.map((line) => line.trim())
			.find((line) => line.length > 0);
		if (!firstLine) return 'Saved Query';
		return firstLine.length > 56 ? `${firstLine.slice(0, 56)}...` : firstLine;
	}

	function saveActiveQuery() {
		if (!connectionStatus.connected) {
			globalError = 'Connect to a database before saving queries.';
			return;
		}
		const sql = getActiveSql().trim();
		if (!sql) return;

		const duplicate = queryFavorites.find(
			(item) =>
				item.connectionKey === activeConnectionKey && item.sql.trim() === sql,
		);
		if (duplicate) {
			globalError = 'This query is already saved for the current connection.';
			return;
		}

		const next: SavedQueryItem = {
			id: crypto.randomUUID(),
			title: deriveFavoriteTitle(sql),
			sql,
			createdAt: Date.now(),
			connectionKey: activeConnectionKey,
		};
		queryFavorites = [next, ...queryFavorites].slice(0, 200);
		persistQueryFavorites();
		globalError = '';
	}

	function nowLabel() {
		return new Date().toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit',
		});
	}

	function pushHistory(item: QueryHistoryItem) {
		queryHistory = [item, ...queryHistory].slice(0, 100);
		persistQueryHistory();
	}

	function getActiveSql() {
		if (!activeTab || activeTab.kind !== 'query') return '';
		return activeTab.sql;
	}

	function addQueryTab(initialSql = '') {
		const nextTab: WorkspaceTab = {
			id: crypto.randomUUID(),
			title: `Query ${tabs.filter((t) => t.kind === 'query').length + 1}`,
			kind: 'query',
			sql: initialSql,
			lastRunSql: '',
			result: emptyResult(),
			sqlError: '',
			resultContext: null,
		};
		tabs = [...tabs, nextTab];
		activeTabId = nextTab.id;
	}

	function setSqlInReusableQueryTab(sql: string) {
		const targetTab =
			(activeTab && activeTab.kind === 'query' ? activeTab : null) ??
			tabs.find((tab) => tab.kind === 'query') ??
			null;
		if (!targetTab) {
			addQueryTab(sql);
			return;
		}
		tabs = tabs.map((tab) =>
			tab.id === targetTab.id ? { ...tab, sql, sqlError: '' } : tab,
		);
		activeTabId = targetTab.id;
	}

	function addDataTab(
		title: string,
		sql: string,
		context: { schema: string; table: string } | null,
	) {
		const nextTab: WorkspaceTab = {
			id: crypto.randomUUID(),
			title,
			kind: 'data',
			sql,
			lastRunSql: '',
			result: emptyResult(),
			sqlError: '',
			resultContext: context,
		};
		tabs = [...tabs, nextTab];
		activeTabId = nextTab.id;
		return nextTab.id;
	}

	function closeTab(tabId: string) {
		const index = tabs.findIndex((tab) => tab.id === tabId);
		if (index === -1) return;
		const nextTabs = tabs.filter((tab) => tab.id !== tabId);
		tabs = nextTabs;
		if (activeTabId === tabId) {
			if (nextTabs.length === 0) {
				activeTabId = '';
				return;
			}
			const nextIndex = Math.max(0, index - 1);
			activeTabId = nextTabs[nextIndex]?.id ?? nextTabs[0]?.id ?? '';
		}
	}

	function openTabContextMenu(event: MouseEvent, tabId: string) {
		event.preventDefault();
		tabContextMenu = { x: event.clientX, y: event.clientY, tabId };
	}

	function closeAllTabs() {
		tabs = [];
		activeTabId = '';
		tabContextMenu = null;
	}

	function closeAllTabsBut(tabId: string) {
		const target = tabs.find((tab) => tab.id === tabId);
		if (!target) return;
		tabs = [target];
		activeTabId = target.id;
		tabContextMenu = null;
	}

	function setActiveSql(nextSql: string) {
		setSqlInReusableQueryTab(nextSql);
	}

	function formatActiveQuery() {
		if (!activeTab || activeTab.kind !== 'query') return;
		const sql = activeTab.sql.trim();
		if (!sql) return;
		try {
			const nextSql = formatSql(sql, { language: 'postgresql' });
			setActiveSql(nextSql);
			globalError = '';
		} catch (error) {
			globalError = error instanceof Error ? error.message : String(error);
		}
	}

	function deleteSavedQuery(id: string) {
		queryFavorites = queryFavorites.filter((item) => item.id !== id);
		persistQueryFavorites();
	}

	function openSavedQuery(sql: string) {
		setSqlInReusableQueryTab(sql);
		mainView = 'sql';
	}

	function clampResultsHeight(height: number, total: number) {
		const minHeight = 140;
		const maxHeight = Math.max(minHeight, total - 180);
		return Math.min(maxHeight, Math.max(minHeight, height));
	}

	function stopResultsResize() {
		resizingResults = false;
		if (moveListener) {
			window.removeEventListener('pointermove', moveListener);
			moveListener = null;
		}
		if (upListener) {
			window.removeEventListener('pointerup', upListener);
			window.removeEventListener('pointercancel', upListener);
			upListener = null;
		}
	}

	function startResultsResize(event: PointerEvent) {
		if (!sqlSplitContainer) return;
		event.preventDefault();
		resizingResults = true;
		moveListener = (moveEvent: PointerEvent) => {
			if (!sqlSplitContainer) return;
			const rect = sqlSplitContainer.getBoundingClientRect();
			const total = sqlSplitContainer.clientHeight;
			const pointerOffset = moveEvent.clientY - rect.top;
			const nextHeight = total - pointerOffset;
			resultsPaneHeight = clampResultsHeight(nextHeight, total);
		};
		upListener = () => stopResultsResize();
		window.addEventListener('pointermove', moveListener);
		window.addEventListener('pointerup', upListener);
		window.addEventListener('pointercancel', upListener);
	}

	function applyConnectionToForm(connection: ConnectionInput) {
		connectionForm = { ...connection };
		connectionInputMode =
			connection.useConnectionString && connection.connectionString
				? 'string'
				: 'fields';
		connectionStringInput = connection.connectionString ?? '';
	}

	function startCreateConnection() {
		editingConnectionName = null;
		showConnectionModal = true;
	}

	function startEditConnection(connection: ConnectionInput) {
		applyConnectionToForm(connection);
		editingConnectionName = connection.name;
		showConnectionModal = true;
	}

	function buildConnectionPayload(): ConnectionInput {
		if (connectionInputMode === 'string') {
			return {
				...connectionForm,
				useConnectionString: true,
				connectionString: connectionStringInput.trim(),
			};
		}
		return {
			...connectionForm,
			useConnectionString: false,
			connectionString: '',
		};
	}

	function toggleSchema(schemaName: string) {
		const next = new Set(expandedSchemas);
		if (next.has(schemaName)) next.delete(schemaName);
		else next.add(schemaName);
		expandedSchemas = next;
	}

	function toggleTable(schemaName: string, tableName: string) {
		const key = `${schemaName}.${tableName}`;
		const next = new Set(expandedTables);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		expandedTables = next;
	}

	async function loadExplorer(options?: { clearBeforeLoad?: boolean }) {
		if (!connectionStatus.connected) {
			explorer = null;
			expandedSchemas = new Set();
			expandedTables = new Set();
			isExplorerLoading = false;
			return;
		}
		if (options?.clearBeforeLoad) {
			explorer = null;
			expandedSchemas = new Set();
			expandedTables = new Set();
		}
		isExplorerLoading = true;
		try {
			const nextExplorer = await rpc.request.getDatabaseExplorer();
			explorer = nextExplorer;
			globalError = '';
			if (nextExplorer.schemas.length > 0 && expandedSchemas.size === 0) {
				expandedSchemas = new Set([nextExplorer.schemas[0].name]);
			}
		} catch (error) {
			explorer = null;
			globalError = error instanceof Error ? error.message : String(error);
		} finally {
			isExplorerLoading = false;
		}
	}

	async function loadDatabases() {
		if (!connectionStatus.connected) {
			databases = [];
			return;
		}
		try {
			databases = await rpc.request.listDatabases();
		} catch {
			databases = connectionStatus.database ? [connectionStatus.database] : [];
		}
	}

	async function handleTestConnection() {
		isTestingConnection = true;
		testConnectionMessage = '';
		try {
			const response = await rpc.request.testConnection(
				buildConnectionPayload(),
			);
			testConnectionOk = response.ok;
			testConnectionMessage = response.ok
				? `Connected successfully${response.serverVersion ? ` (PostgreSQL ${response.serverVersion})` : ''}`
				: response.message;
		} catch (error) {
			testConnectionOk = false;
			testConnectionMessage =
				error instanceof Error ? error.message : String(error);
		} finally {
			isTestingConnection = false;
		}
	}

	async function handleConnect(saveConnection: boolean) {
		isConnecting = true;
		testConnectionMessage = '';
		try {
			const payload = buildConnectionPayload();
			connectionStatus = await rpc.request.connect(payload);
			if (saveConnection) {
				if (editingConnectionName && editingConnectionName !== payload.name)
					removeSavedConnection(editingConnectionName);
				upsertSavedConnection(payload);
			}
			showConnectionModal = false;
			editingConnectionName = null;
			mainView = 'sql';
			await loadDatabases();
			await loadExplorer();
		} catch (error) {
			testConnectionOk = false;
			testConnectionMessage =
				error instanceof Error ? error.message : String(error);
		} finally {
			isConnecting = false;
		}
	}

	async function connectSaved(connection: ConnectionInput) {
		connectingName = connection.name;
		applyConnectionToForm(connection);
		try {
			await handleConnect(false);
		} finally {
			connectingName = null;
		}
	}

	async function handleDisconnect() {
		await rpc.request.disconnect();
		connectionStatus = await rpc.request.connectionStatus();
		explorer = null;
		databases = [];
		tabs = [];
		activeTabId = '';
		mainView = 'sql';
		editingConnectionName = null;
		globalError = '';
	}

	async function executeQuery(
		query: string,
		options?: {
			pushToHistory?: boolean;
			targetTabId?: string;
			context?: { schema: string; table: string } | null;
			historySql?: string;
		},
	) {
		isRunningQuery = true;
		const targetTabId = options?.targetTabId ?? activeTabId;
		try {
			const queryResult = await rpc.request.runQuery({ sql: query });
			queryDurationMs = queryResult.durationMs;
			globalError = '';
			tabs = tabs.map((tab) =>
				tab.id === targetTabId
					? {
							...tab,
							result: queryResult,
							lastRunSql: query,
							sqlError: '',
							resultContext: options?.context ?? tab.resultContext,
						}
					: tab,
			);
			if (options?.pushToHistory !== false) {
				pushHistory({
					time: nowLabel(),
					sql: options?.historySql ?? query,
					durationMs: queryResult.durationMs,
					success: true,
					connectionKey: activeConnectionKey,
				});
			}
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			globalError = message;
			tabs = tabs.map((tab) =>
				tab.id === targetTabId ? { ...tab, sqlError: message } : tab,
			);
			if (options?.pushToHistory !== false) {
				pushHistory({
					time: nowLabel(),
					sql: options?.historySql ?? query,
					durationMs: 0,
					success: false,
					error: message,
					connectionKey: activeConnectionKey,
				});
			}
		} finally {
			isRunningQuery = false;
		}
	}

	async function handleRunQuery(queryOverride?: string) {
		if (!connectionStatus.connected) {
			globalError = 'No active connection';
			return;
		}
		ensureTab();
		if (!activeTab || activeTab.kind !== 'query') return;
		const sqlToRun = (queryOverride ?? activeTab.sql).trim();
		if (!sqlToRun) {
			globalError = 'Query is empty';
			return;
		}

		const editablePlan = tryBuildEditableQuery(sqlToRun);
		const queryToRun = editablePlan?.sql ?? sqlToRun;
		const context = editablePlan?.context ?? null;
		await executeQuery(queryToRun, {
			pushToHistory: true,
			targetTabId: activeTab.id,
			context,
			historySql: sqlToRun,
		});
	}

	async function handleTableAction(
		action: TableAction,
		schema: string,
		table: string,
	) {
		const safeSchema = quoteIdent(schema);
		const safeTable = quoteIdent(table);
		if (action === 'copy_name') {
			await navigator.clipboard.writeText(`${schema}.${table}`);
			return;
		}
		if (action === 'hide') {
			globalError = `Hide is not implemented yet for ${schema}.${table}`;
			return;
		}
		if (action === 'import_file') {
			globalError = 'Import from file is not implemented yet.';
			return;
		}
		if (action === 'rename') {
			renameTarget = { schema, table };
			renameValue = table;
			showRenameModal = true;
			return;
		}

		if (action === 'drop') {
			setActiveSql(`drop table ${safeSchema}.${safeTable};`);
			globalError =
				'Drop statement inserted into editor. Review before running.';
			return;
		}
		if (action === 'truncate') {
			setActiveSql(`truncate table ${safeSchema}.${safeTable};`);
			globalError =
				'Truncate statement inserted into editor. Review before running.';
			return;
		}
		if (action === 'duplicate') {
			setActiveSql(
				`create table ${safeSchema}.${safeTable}_copy as select * from ${safeSchema}.${safeTable};`,
			);
			globalError =
				'Duplicate statement inserted into editor. Review before running.';
			return;
		}
		if (action === 'sql_create') {
			setActiveSql(
				`-- Table definition helper\nselect column_name, data_type, is_nullable\nfrom information_schema.columns\nwhere table_schema = '${schema.replaceAll("'", "''")}'\n  and table_name = '${table.replaceAll("'", "''")}'\norder by ordinal_position;`,
			);
			globalError = '';
			return;
		}

		let query = '';
		let title = `${table}`;
		let context: { schema: string; table: string } | null = null;
		if (action === 'view_data') {
			const firstOrderColumn = explorer?.schemas
				.find((item) => item.name === schema)
				?.tables.find((item) => item.name === table)?.columns[0]?.name;
			const orderByClause = firstOrderColumn
				? ` order by ${quoteIdent(firstOrderColumn)} asc nulls last`
				: '';
			query = `select ctid::text as _querycastle_ctid, * from ${safeSchema}.${safeTable}${orderByClause} limit 100;`;
			title = `${table} [all]`;
			context = { schema, table };
		}
		if (action === 'view_structure') {
			query = `select column_name, data_type, is_nullable from information_schema.columns where table_schema = '${schema.replaceAll("'", "''")}' and table_name = '${table.replaceAll("'", "''")}' order by ordinal_position;`;
			title = `${table} [structure]`;
		}
		if (action === 'export_file') {
			query = `select * from ${safeSchema}.${safeTable} limit 1000;`;
			title = `${table} [export]`;
		}
		const tabId = addDataTab(title, query, context);
		await executeQuery(query, {
			targetTabId: tabId,
			pushToHistory: false,
			context,
		});
	}

	async function handleSchemaAction(action: SchemaAction, schema: string) {
		if (action === 'copy_name') {
			await navigator.clipboard.writeText(schema);
			return;
		}
		if (action === 'copy_quoted_name') {
			await navigator.clipboard.writeText(quoteIdent(schema));
			return;
		}
		if (action === 'sql_list_tables') {
			const query = `select tablename as table_name\nfrom pg_catalog.pg_tables\nwhere schemaname = '${schema.replaceAll("'", "''")}'\norder by tablename;`;
			const tabId = addDataTab(`${schema} [tables]`, query, null);
			await executeQuery(query, {
				targetTabId: tabId,
				pushToHistory: false,
				context: null,
			});
			globalError = '';
		}
	}

	async function submitRename() {
		if (!renameTarget) return;
		const nextName = renameValue.trim();
		if (!nextName) {
			globalError = 'New table name is required.';
			return;
		}
		showRenameModal = false;
		const sql = `alter table ${quoteIdent(renameTarget.schema)}.${quoteIdent(renameTarget.table)} rename to ${quoteIdent(nextName)};`;
		if (!activeTab || activeTab.kind !== 'query') addQueryTab(sql);
		else setActiveSql(sql);
	}

	async function followForeignKey(schema: string, table: string) {
		await handleTableAction('view_data', schema, table);
	}

	async function applyTableChanges(
		context: { schema: string; table: string },
		changes: TableChangesPayload,
	): Promise<ApplyTableChangesResult> {
		return rpc.request.applyTableChanges({
			schema: context.schema,
			table: context.table,
			changes,
		});
	}

	async function handleDatabaseChange(database: string) {
		if (!database || database === connectionStatus.database) return;
		globalError = '';
		isExplorerLoading = true;
		explorer = null;
		expandedSchemas = new Set();
		expandedTables = new Set();
		try {
			connectionStatus = await rpc.request.selectDatabase({ database });
			await loadDatabases();
			await loadExplorer({ clearBeforeLoad: true });
		} catch (error) {
			globalError = error instanceof Error ? error.message : String(error);
			isExplorerLoading = false;
		}
	}

	async function restoreDataTabResults() {
		if (!connectionStatus.connected) return;
		const dataTabs = tabs.filter(
			(tab) => tab.kind === 'data' && tab.sql.trim().length > 0,
		);
		for (const tab of dataTabs) {
			await executeQuery(tab.sql, {
				pushToHistory: false,
				targetTabId: tab.id,
				context: tab.resultContext,
			});
		}
	}

	onMount(() => {
		loadSavedConnections();
		loadQueryTabs();
		loadQueryFavorites();
		loadQueryHistory();
		void (async () => {
			try {
				connectionStatus = await rpc.request.connectionStatus();
				await loadDatabases();
				await loadExplorer();
				await restoreDataTabResults();
			} catch (error) {
				console.error('Failed to initialize app', error);
			}
		})();

		const onWindowResize = () => {
			if (!sqlSplitContainer) return;
			const total = sqlSplitContainer.clientHeight;
			resultsPaneHeight = clampResultsHeight(resultsPaneHeight, total);
		};
		window.addEventListener('resize', onWindowResize);
		return () => {
			window.removeEventListener('resize', onWindowResize);
		};
	});

	onDestroy(() => {
		stopResultsResize();
	});

	$effect(() => {
		const persisted = tabs.map((tab) => ({
			id: tab.id,
			title: tab.title,
			kind: tab.kind,
			sql: tab.sql,
			resultContext: tab.resultContext,
		}));
		localStorage.setItem(QUERY_TABS_KEY, JSON.stringify(persisted));
	});
</script>

<main
	class="h-screen w-full flex flex-col bg-gray-50 overflow-hidden text-sm text-gray-800 antialiased"
>
	<header
		class="flex items-center h-12 bg-[#1c1c1e] text-gray-300 px-4 shrink-0 shadow-sm z-30 relative"
	>
		<div class="flex items-center space-x-3 min-w-0">
			<div class="bg-emerald-500 rounded p-1 flex items-center justify-center">
				<Database size={16} class="text-[#1c1c1e]" />
			</div>
			<div class="flex items-center text-sm min-w-0">
				<span class="text-gray-400 cursor-default hover:text-gray-200 truncate"
					>{connectionStatus.connected
						? connectionStatus.host
						: 'Disconnected'}</span
				>
				<ChevronRight size={14} class="mx-1 text-gray-600 shrink-0" />
				<span
					class="font-medium text-gray-100 cursor-default flex items-center truncate"
				>
					{connectionStatus.connected
						? connectionStatus.database
						: 'Database IDE'}
				</span>
			</div>
			<div
				class=" bg-emerald-900/40 border border-emerald-800/50 text-emerald-400 text-xs px-2 py-0.5 rounded shadow-sm"
			>
				Demo
			</div>
		</div>
		<div class="flex-1"></div>
		<div class="flex items-center gap-2">
			<button
				onclick={startCreateConnection}
				class="h-7 px-2 rounded border border-white/15 text-xs text-gray-300 hover:text-white hover:bg-white/10 inline-flex items-center gap-1"
			>
				<Plus size={12} />
				Connection
			</button>
			{#if connectionStatus.connected}
				<button
					onclick={handleDisconnect}
					class="h-7 px-2 rounded border border-white/15 text-xs text-gray-300 hover:text-white hover:bg-white/10 inline-flex items-center gap-1"
				>
					<Unplug size={12} />
					Disconnect
				</button>
			{/if}
		</div>
	</header>

	{#if !connectionStatus.connected}
		<div class="flex flex-1 overflow-hidden">
			<aside
				class="w-14 bg-white border-r border-gray-200 flex flex-col items-center py-4 space-y-3 shrink-0 z-20 shadow-[1px_0_5px_rgba(0,0,0,0.02)]"
			>
				<button
					class="text-gray-900 bg-gray-100 p-1.5 rounded-md shadow-sm border border-gray-200/50 flex items-center justify-center"
					><Home size={20} /></button
				>
				<button
					class="text-gray-400 p-1.5 rounded-md flex items-center justify-center"
					><Star size={20} /></button
				>
				<button
					class="text-gray-400 p-1.5 rounded-md flex items-center justify-center"
					><FolderKanban size={20} /></button
				>
			</aside>
			<section class="flex-1 overflow-auto border-l border-gray-100 bg-white">
				<ConnectionHub
					{savedConnections}
					onConnect={connectSaved}
					onCreate={startCreateConnection}
					onEdit={startEditConnection}
					onDelete={removeSavedConnection}
					{connectingName}
					searchQuery={connectionSearch}
				/>
			</section>
		</div>
	{:else}
		<div class="flex flex-1 overflow-hidden">
			<aside
				class="w-14 bg-white border-r border-gray-200 flex flex-col items-center py-4 space-y-6 shrink-0 z-20 shadow-[1px_0_5px_rgba(0,0,0,0.02)]"
			>
				<button
					onclick={() => (mainView = 'sql')}
					class={`transition-colors p-1.5 rounded-md flex items-center justify-center ${mainView === 'sql' ? 'text-gray-900 bg-gray-100 shadow-sm border border-gray-200/50' : 'text-gray-400 hover:text-gray-900 hover:bg-gray-100'}`}
				>
					<Home size={20} />
				</button>
				<button
					onclick={() => (mainView = 'saved_queries')}
					class={`transition-colors p-1.5 rounded-md flex items-center justify-center ${mainView === 'saved_queries' ? 'text-gray-900 bg-gray-100 shadow-sm border border-gray-200/50' : 'text-gray-400 hover:text-gray-900 hover:bg-gray-100'}`}
					><Star size={20} /></button
				>
				<button
					onclick={() => (mainView = 'last_queries')}
					class={`transition-colors p-1.5 rounded-md flex items-center justify-center ${mainView === 'last_queries' ? 'text-gray-900 bg-gray-100 shadow-sm border border-gray-200/50' : 'text-gray-400 hover:text-gray-900 hover:bg-gray-100'}`}
					><FolderKanban size={20} /></button
				>
			</aside>

			{#if mainView === 'sql'}
				<ExplorerSidebar
					{connectionStatus}
					{explorer}
					loadingExplorer={isExplorerLoading}
					{databases}
					searchQuery={explorerSearch}
					{expandedSchemas}
					{expandedTables}
					onChangeDatabase={handleDatabaseChange}
					onSearchChange={(value) => (explorerSearch = value)}
					onToggleSchema={toggleSchema}
					onToggleTable={toggleTable}
					onRefresh={loadExplorer}
					onTableAction={handleTableAction}
					onSchemaAction={handleSchemaAction}
					onFollowForeignKey={followForeignKey}
				/>

				<section
					class="flex-1 relative flex flex-col min-w-0 min-h-0 bg-white border-l border-gray-100"
				>
					<div
						class="flex items-center bg-gray-50/80 border-b border-gray-200 overflow-x-auto hide-scrollbar shrink-0"
					>
						{#each tabs as tab}
							<div
								class={`flex items-center min-w-0 max-w-72 px-4 py-2 border-r border-gray-200 border-t-2 text-sm font-medium relative z-10 -mb-[1px] ${tab.id === activeTabId ? 'bg-white border-t-emerald-500 text-gray-800' : 'border-t-transparent text-gray-500 hover:bg-gray-100/50 hover:text-gray-700'}`}
							>
								<button
									onclick={() => {
										activeTabId = tab.id;
										tabContextMenu = null;
									}}
									oncontextmenu={(event) => openTabContextMenu(event, tab.id)}
									class="inline-flex items-center space-x-2 min-w-0 flex-1"
								>
									<FileCode2
										size={16}
										class={`shrink-0 ${tab.id === activeTabId ? 'text-emerald-500' : ''}`}
									/>
									<span class="truncate">{tab.title}</span>
								</button>
								<button
									onclick={() => {
										closeTab(tab.id);
										tabContextMenu = null;
									}}
									class="text-gray-400 hover:text-gray-600 ml-2 shrink-0"
									aria-label={`Close ${tab.title}`}>x</button
								>
							</div>
						{/each}
						<button
							onclick={() => addQueryTab()}
							class="px-3 py-2 text-gray-400 hover:text-gray-600 transition-colors"
							>+</button
						>
					</div>
					{#if tabContextMenu}
						<button
							class="fixed inset-0 z-40"
							aria-label="Close tab menu"
							onclick={() => (tabContextMenu = null)}
						></button>
						<div
							class="fixed z-50 min-w-[180px] bg-white rounded-md border border-gray-200 shadow-[0_8px_24px_rgba(0,0,0,0.12)] py-1"
							style={`left:${tabContextMenu.x}px;top:${tabContextMenu.y}px;`}
						>
							<button
								onclick={closeAllTabs}
								class="w-full px-3 py-1.5 text-left text-sm text-gray-700 hover:bg-gray-50"
							>
								Close all
							</button>
							<button
								onclick={() => closeAllTabsBut(tabContextMenu?.tabId ?? '')}
								class="w-full px-3 py-1.5 text-left text-sm text-gray-700 hover:bg-gray-50"
							>
								Close all but this
							</button>
						</div>
					{/if}

					{#if activeTab?.kind === 'query'}
						<div
							bind:this={sqlSplitContainer}
							class={`flex-1 flex flex-col min-h-0 bg-white ${resizingResults ? 'select-none cursor-row-resize' : ''}`}
						>
							<SqlEditor
								value={activeTab.sql}
								onChange={setActiveSql}
								onRun={handleRunQuery}
								onSaveQuery={saveActiveQuery}
								onFormatQuery={formatActiveQuery}
								running={isRunningQuery}
								disabled={!connectionStatus.connected}
								completions={sqlCompletions}
							/>

							<button
								type="button"
								aria-label="Resize results panel"
								onpointerdown={startResultsResize}
								class="h-1.5 bg-gray-100 border-y border-gray-200 hover:bg-emerald-400/50 cursor-row-resize transition-colors shrink-0 z-20 relative flex items-center justify-center"
							>
								<div
									class="w-8 h-0.5 bg-gray-300 rounded-full pointer-events-none"
								></div>
							</button>

							<div
								style={`height:${resultsPaneHeight}px;`}
								class="flex flex-col bg-white shrink-0 min-h-0"
							>
								<ResultsPane
									result={activeTab.result}
									sqlError={activeTab.sqlError || globalError}
									resultContext={activeTab.resultContext}
									loading={isRunningQuery}
									refreshSql={activeTab.lastRunSql}
									onRunSql={(query) =>
										executeQuery(query, {
											pushToHistory: false,
											targetTabId: activeTab.id,
											context: activeTab.resultContext,
										})}
									onApplyTableChanges={applyTableChanges}
									durationMs={activeTab.result.durationMs || queryDurationMs}
								/>
							</div>
						</div>
					{:else if activeTab}
						<div class="flex-1 min-w-0 min-h-0 flex flex-col">
							<ResultsPane
								result={activeTab.result}
								sqlError={activeTab.sqlError || globalError}
								resultContext={activeTab.resultContext}
								loading={isRunningQuery}
								refreshSql={activeTab.lastRunSql}
								onRunSql={(query) =>
									executeQuery(query, {
										pushToHistory: false,
										targetTabId: activeTab.id,
										context: activeTab.resultContext,
									})}
								onApplyTableChanges={applyTableChanges}
								durationMs={activeTab.result.durationMs || queryDurationMs}
							/>
						</div>
					{:else}
						<div class="flex-1 flex items-center justify-center p-6 bg-gray-50">
							<div
								class="w-full max-w-md rounded-xl border border-gray-200 bg-white p-5"
							>
								<div class="text-sm font-semibold text-gray-900 mb-3">
									Quick Shortcuts
								</div>
								<div class="space-y-2 text-xs text-gray-600">
									<div class="flex items-center justify-between">
										<span>Run Query</span><span
											class="px-2 py-0.5 rounded border border-gray-200 bg-gray-50 font-mono-code"
											>Ctrl+Enter</span
										>
									</div>
									<div class="flex items-center justify-between">
										<span>Save Query</span><span
											class="px-2 py-0.5 rounded border border-gray-200 bg-gray-50 font-mono-code"
											>Ctrl+S</span
										>
									</div>
									<div class="flex items-center justify-between">
										<span>Format SQL</span><span
											class="px-2 py-0.5 rounded border border-gray-200 bg-gray-50 font-mono-code"
											>Shift+Alt+F</span
										>
									</div>
									<div class="flex items-center justify-between">
										<span>New Query Tab</span><span
											class="px-2 py-0.5 rounded border border-gray-200 bg-gray-50 font-mono-code"
											>+</span
										>
									</div>
								</div>
								<div class="mt-4">
									<button
										onclick={() => addQueryTab('SELECT 1;')}
										class="h-8 px-3 rounded-md border border-emerald-500 bg-emerald-500 text-white text-xs hover:bg-emerald-600 hover:border-emerald-600"
										>Open Query Tab</button
									>
								</div>
							</div>
						</div>
					{/if}
				</section>
			{:else}
				<section
					class="flex-1 min-w-0 flex border-l border-gray-100 bg-gray-50"
				>
					<aside
						class="w-[260px] border-r border-gray-200 bg-white flex flex-col shrink-0"
					>
						<div
							class="h-11 px-4 border-b border-gray-200 flex items-center text-xs font-semibold tracking-[0.08em] text-gray-500 uppercase"
						>
							Explorer
						</div>
						<div class="p-4 text-sm overflow-auto">
							{#if mainView === 'saved_queries'}
								<div class="space-y-3">
									<div>
										<div class="text-gray-700 font-medium mb-2">
											Saved Queries
										</div>
										<div class="space-y-1">
											{#if favoritesForConnection.length === 0}
												<div class="text-xs text-gray-500">
													No saved queries
												</div>
											{:else}
												{#each favoritesForConnection as item}
													<div
														class={`w-full px-2 py-1.5 rounded-md flex items-center gap-2 min-w-0 ${selectedSavedQueryId === item.id ? 'bg-gray-100 text-gray-900' : 'text-gray-700 hover:bg-gray-50'}`}
													>
														<button
															onclick={() => (selectedSavedQueryId = item.id)}
															class="min-w-0 flex-1 text-left inline-flex items-center gap-2"
														>
															<FileCode2
																size={14}
																class="shrink-0 text-emerald-500"
															/>
															<span class="truncate">{item.title}</span>
														</button>
														<button
															onclick={() => deleteSavedQuery(item.id)}
															class="shrink-0 text-gray-400 hover:text-red-600"
															aria-label={`Delete ${item.title}`}
															title="Delete saved query"
														>
															<Trash2 size={14} />
														</button>
													</div>
												{/each}
											{/if}
										</div>
									</div>
								</div>
							{:else if mainView === 'last_queries'}
								<div class="space-y-2">
									<div class="text-gray-700 font-medium mb-2">Last Queries</div>
									<div class="space-y-1">
										{#if historyForConnection.length === 0}
											<div class="text-xs text-gray-500">No query history</div>
										{:else}
											{#each historyForConnection as item, index}
												<button
													onclick={() => (selectedHistoryIndex = index)}
													class={`w-full text-left px-2 py-1.5 rounded-md min-w-0 ${selectedHistoryIndex === index ? 'bg-gray-100 text-gray-900' : 'text-gray-700 hover:bg-gray-50'}`}
												>
													<div class="truncate font-mono-code text-xs">
														{item.sql}
													</div>
													<div class="mt-1 text-[10px] text-gray-500">
														{item.time} • {item.durationMs}ms
													</div>
												</button>
											{/each}
										{/if}
									</div>
								</div>
							{/if}
						</div>
					</aside>

					<div class="flex-1 min-w-0 flex flex-col">
						{#if mainView === 'saved_queries'}
							<div
								class="h-11 px-6 border-b border-gray-200 bg-white flex items-center justify-between"
							>
								<div class="inline-flex items-center gap-2 min-w-0">
									<FileCode2 size={15} class="text-emerald-500 shrink-0" />
									<span class="text-sm font-semibold text-gray-900 truncate"
										>{selectedSavedQuery?.title ?? 'saved_query.sql'}</span
									>
								</div>
								<div class="flex items-center gap-2">
									<button
										onclick={() => {
											if (selectedSavedQuery)
												deleteSavedQuery(selectedSavedQuery.id);
										}}
										disabled={!selectedSavedQuery}
										class="h-8 px-3 rounded-md border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-50 disabled:opacity-60"
									>
										Remove
									</button>
									<button
										onclick={() => {
											if (selectedSavedQuery)
												openSavedQuery(selectedSavedQuery.sql);
										}}
										disabled={!selectedSavedQuery}
										class="h-8 px-3 rounded-md border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-50 disabled:opacity-60"
									>
										Open in Editor
									</button>
								</div>
							</div>
							<div class="flex-1 overflow-auto p-6">
								{#if selectedSavedQuery}
									<div class="rounded-xl border border-gray-200 bg-white p-4">
										<pre
											class="font-mono-code text-sm leading-7 text-gray-700 whitespace-pre-wrap">{selectedSavedQuery.sql}</pre>
									</div>
								{:else}
									<div
										class="h-full flex items-center justify-center text-sm text-gray-500"
									>
										No saved queries yet for this connection.
									</div>
								{/if}
							</div>
						{:else if mainView === 'last_queries'}
							<div
								class="h-11 px-6 border-b border-gray-200 bg-white flex items-center"
							>
								<div class="inline-flex items-center gap-2 min-w-0">
									<FileCode2 size={15} class="text-emerald-500 shrink-0" />
									<span class="text-sm font-semibold text-gray-900 truncate"
										>last_queries.sql</span
									>
								</div>
							</div>
							<div class="flex-1 overflow-auto p-6">
								{#if selectedHistoryQuery}
									<div class="rounded-xl border border-gray-200 bg-white p-4">
										<div class="text-xs text-gray-500 mb-3">
											{selectedHistoryQuery.time} • {selectedHistoryQuery.durationMs}ms
											• {selectedHistoryQuery.success ? 'Success' : 'Error'}
										</div>
										<pre
											class="font-mono-code text-sm leading-7 text-gray-700 whitespace-pre-wrap">{selectedHistoryQuery.sql}</pre>
										{#if selectedHistoryQuery.error}
											<div class="mt-3 text-xs text-red-600">
												{selectedHistoryQuery.error}
											</div>
										{/if}
									</div>
								{:else}
									<div
										class="h-full flex items-center justify-center text-sm text-gray-500"
									>
										No query history for this connection.
									</div>
								{/if}
							</div>
						{/if}
					</div>
				</section>
			{/if}
		</div>
	{/if}

	<ConnectionModal
		visible={showConnectionModal}
		editing={editingConnectionName !== null}
		mode={connectionInputMode}
		{connectionForm}
		{connectionStringInput}
		{testConnectionMessage}
		{testConnectionOk}
		{isTestingConnection}
		{isConnecting}
		onClose={() => {
			showConnectionModal = false;
			editingConnectionName = null;
		}}
		onModeChange={(mode) => (connectionInputMode = mode)}
		onConnectionFormChange={(next) => (connectionForm = next)}
		onConnectionStringChange={(value) => (connectionStringInput = value)}
		onTest={handleTestConnection}
		onSaveAndConnect={() => handleConnect(true)}
	/>

	{#if showRenameModal && renameTarget}
		<div
			class="fixed inset-0 z-70 bg-black/55 backdrop-blur-[1px] flex items-center justify-center p-4"
		>
			<div
				class="w-full max-w-md rounded-xl border border-gray-200 bg-white shadow-[0_24px_60px_rgba(16,37,70,0.26)]"
			>
				<div
					class="h-10 px-4 border-b border-gray-200 flex items-center justify-between bg-gray-50"
				>
					<h3 class="text-sm font-semibold text-gray-900">Rename Table</h3>
					<button
						onclick={() => (showRenameModal = false)}
						class="text-gray-500 hover:text-gray-900">x</button
					>
				</div>
				<div class="p-4 space-y-3">
					<div class="text-xs text-gray-500">
						Current table: <span class="text-gray-900"
							>{renameTarget.schema}.{renameTarget.table}</span
						>
					</div>
					<input
						bind:value={renameValue}
						placeholder="New table name"
						class="ui-input w-full h-9 text-sm px-2"
					/>
				</div>
				<div
					class="h-12 px-4 border-t border-gray-200 flex items-center justify-end gap-2 bg-gray-50"
				>
					<button
						onclick={() => (showRenameModal = false)}
						class="btn-secondary h-8 px-3 rounded-md text-xs">Cancel</button
					>
					<button
						onclick={submitRename}
						class="btn-primary h-8 px-3 rounded-md text-xs font-medium"
						>Rename</button
					>
				</div>
			</div>
		</div>
	{/if}
</main>
