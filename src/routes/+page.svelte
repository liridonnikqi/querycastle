<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import type {
		ApplyTableChangesResult,
		ConnectionInput,
		ConnectionStatus,
		DatabaseExplorer,
		TableChangesPayload,
	} from '$lib/rpc';
	import { rpc } from '$lib/rpc-client';
	import type { QueryHistoryItem, SavedQueryItem } from '$lib/types';
import AppHeader from '$lib/components/workspace/AppHeader.svelte';
import DisconnectedWorkspace from '$lib/components/workspace/DisconnectedWorkspace.svelte';
import WorkspaceBody from '$lib/components/workspace/WorkspaceBody.svelte';
import WorkspaceModals from '$lib/components/workspace/WorkspaceModals.svelte';
import StatusBar from '$lib/components/workspace/StatusBar.svelte';
import SearchPalette from '$lib/components/workspace/SearchPalette.svelte';
	import { normalizeConnectionInput } from '$lib/utils/connection';
	import { tryBuildEditableQuery } from '$lib/utils/editable-query';
	import {
		loadQueryFavoritesFromStorage,
		loadQueryHistoryFromStorage,
		loadQueryTabsFromStorage,
		loadSavedConnectionsFromStorage,
		persistJsonValue,
		toPersistedTabs,
	} from '$lib/utils/persistence';
	import {
		closeTabState,
		createDataTab as makeDataTab,
		createQueryTab,
		setSqlInReusableQueryTabState,
	} from '$lib/utils/tabs';
	import {
		SAVED_CONNECTIONS_KEY,
		QUERY_TABS_KEY,
		QUERY_FAVORITES_KEY,
		QUERY_HISTORY_KEY,
		clampResultsHeight,
		createDefaultTab,
		deriveFavoriteTitle,
		nowLabel,
		type MainView,
		type SchemaAction,
		type TableAction,
		type TabContextMenu,
		type WorkspaceTab,
	} from '$lib/utils/workspace';
	import {
		buildCreateDatabaseSql,
		buildRenameTableSql,
		buildSchemaActionPlan,
		buildTableActionPlan,
	} from '$lib/utils/workspace-actions';
	import { initializeWorkspace } from '$lib/utils/workspace-init';
	import { format as formatSql } from 'sql-formatter';
	let connectionStatus = $state<ConnectionStatus>({
		connected: false,
		databaseType: 'postgres',
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
	let forceWorkspaceOnDisconnect = $state(false);
	let showSearchPalette = $state(false);
	let connectionForm = $state<ConnectionInput>({
		databaseType: 'postgres',
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
		return `${connectionStatus.databaseType}:${connectionStatus.host}:${connectionStatus.port}/${connectionStatus.database}/${connectionStatus.user}`;
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
	function ensureTab() {
		if (tabs.length > 0) return;
		const nextTab = createDefaultTab();
		tabs = [nextTab];
		activeTabId = nextTab.id;
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
		persistJsonValue(SAVED_CONNECTIONS_KEY, savedConnections);
	}
	function removeSavedConnection(name: string) {
		savedConnections = savedConnections.filter((item) => item.name !== name);
		persistJsonValue(SAVED_CONNECTIONS_KEY, savedConnections);
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
		persistJsonValue(QUERY_FAVORITES_KEY, queryFavorites);
		globalError = '';
	}
	function pushHistory(item: QueryHistoryItem) {
		queryHistory = [item, ...queryHistory].slice(0, 100);
		persistJsonValue(QUERY_HISTORY_KEY, queryHistory);
	}
	function getActiveSql() {
		if (!activeTab || activeTab.kind !== 'query') return '';
		return activeTab.sql;
	}
	function addQueryTab(initialSql = '') {
		const nextTab = createQueryTab(
			tabs.filter((tab) => tab.kind === 'query').length,
			initialSql,
		);
		tabs = [...tabs, nextTab];
		activeTabId = nextTab.id;
	}
	function setSqlInReusableQueryTab(sql: string) {
		const next = setSqlInReusableQueryTabState({ tabs, activeTabId, sql });
		tabs = next.tabs;
		activeTabId = next.activeTabId;
	}
	function addDataTab(
		title: string,
		sql: string,
		context: { schema: string; table: string } | null,
	) {
		const nextTab = makeDataTab({ title, sql, context });
		tabs = [...tabs, nextTab];
		activeTabId = nextTab.id;
		return nextTab.id;
	}
	function closeTab(tabId: string) {
		const next = closeTabState({ tabs, activeTabId, tabId });
		tabs = next.tabs;
		activeTabId = next.activeTabId;
	}
	function openTabContextMenu(event: MouseEvent, tabId: string) {
		event.preventDefault();
		tabContextMenu = { x: event.clientX, y: event.clientY, tabId };
	}
	function setActiveSql(nextSql: string) {
		setSqlInReusableQueryTab(nextSql);
	}
	function loadExternalSqlFile(path: string, content: string) {
		forceWorkspaceOnDisconnect = true;
		const normalizedPath = path.replaceAll('\\', '/');
		const fileName = normalizedPath.split('/').pop() || 'opened.sql';
		const targetTab =
			(activeTab && activeTab.kind === 'query' ? activeTab : null) ??
			tabs.find((tab) => tab.kind === 'query') ??
			null;
		if (!targetTab) {
			addQueryTab(content);
			const newestTab = tabs[tabs.length - 1];
			if (newestTab) {
				tabs = tabs.map((tab) =>
					tab.id === newestTab.id
						? {
								...tab,
								title: fileName,
								sqlError: '',
								resultContext: null,
								lastRunSql: '',
							}
						: tab,
					);
			}
		} else {
			tabs = tabs.map((tab) =>
				tab.id === targetTab.id
					? {
							...tab,
							title: fileName,
							sql: content,
							sqlError: '',
							resultContext: null,
							lastRunSql: '',
						}
					: tab,
			);
			activeTabId = targetTab.id;
		}
		mainView = 'sql';
		globalError = '';
	}
	async function connectExternalSqliteFile(path: string) {
		const normalizedPath = path.replaceAll('\\', '/');
		const fileName = normalizedPath.split('/').pop() || 'opened.db';
		const connectionName = fileName.replace(/\.(sqlite|sqlite3|db)$/i, '') || 'sqlite_db';
		const payload: ConnectionInput = {
			databaseType: 'sqlite',
			name: connectionName,
			host: '',
			port: 0,
			user: '',
			password: '',
			database: path,
			ssl: false,
			useConnectionString: false,
			connectionString: '',
		};
		connectionForm = payload;
		connectionInputMode = 'fields';
		connectionStringInput = '';
		try {
			connectionStatus = await rpc.request.connect(payload);
			await loadDatabases();
			await loadExplorer({ clearBeforeLoad: true });
			showConnectionModal = false;
			editingConnectionName = null;
			mainView = 'sql';
			globalError = '';
		} catch (error) {
			globalError = error instanceof Error ? error.message : String(error);
			showConnectionModal = true;
		}
	}
	function formatActiveQuery() {
		if (!activeTab || activeTab.kind !== 'query') return;
		const sql = activeTab.sql.trim();
		if (!sql) return;
		try {
			const language =
				connectionStatus.databaseType === 'mysql' ? 'mysql' : 'postgresql';
			const nextSql = formatSql(sql, { language });
			setActiveSql(nextSql);
			globalError = '';
		} catch (error) {
			globalError = error instanceof Error ? error.message : String(error);
		}
	}
	function deleteSavedQuery(id: string) {
		queryFavorites = queryFavorites.filter((item) => item.id !== id);
		persistJsonValue(QUERY_FAVORITES_KEY, queryFavorites);
	}
	function openSavedQuery(sql: string) {
		setSqlInReusableQueryTab(sql);
		mainView = 'sql';
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
		connectionForm = normalizeConnectionInput(connection);
		connectionInputMode =
			connection.databaseType !== 'sqlite' &&
			connection.useConnectionString &&
			connection.connectionString
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
		if (connectionForm.databaseType !== 'sqlite' && connectionInputMode === 'string') {
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
	function handlePaletteSelectTable(schema: string, table: string) {
		if (!expandedSchemas.has(schema)) toggleSchema(schema);
		const key = `${schema}.${table}`;
		if (!expandedTables.has(key)) toggleTable(schema, table);
		void handleTableAction('view_data', schema, table);
	}
	function handlePaletteSelectSchema(schema: string) {
		if (!expandedSchemas.has(schema)) toggleSchema(schema);
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
			const engine = buildConnectionPayload().databaseType;
			const engineLabel = engine === 'mysql' ? 'MySQL' : engine === 'sqlite' ? 'SQLite' : 'PostgreSQL';
			const response = await rpc.request.testConnection(
				buildConnectionPayload(),
			);
			testConnectionOk = response.ok;
			testConnectionMessage = response.ok
				? `Connected successfully${response.serverVersion ? ` (${engineLabel} ${response.serverVersion})` : ''}`
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
			if (
				saveConnection &&
				!editingConnectionName &&
				savedConnections.some((item) => item.name === payload.name)
			) {
				testConnectionOk = false;
				testConnectionMessage =
					'A connection with this name already exists. Choose a different name to save it.';
				return;
			}
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
		const editablePlan = tryBuildEditableQuery({
			sql: sqlToRun,
			databaseType: connectionStatus.databaseType,
			explorer,
		});
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
		const plan = buildTableActionPlan({
			action,
			databaseType: connectionStatus.databaseType,
			explorer,
			schema,
			table,
		});
		if (plan.kind === 'copy_name') {
			await navigator.clipboard.writeText(plan.text);
			return;
		}
		if (plan.kind === 'error') {
			globalError = plan.message;
			return;
		}
		if (plan.kind === 'rename') {
			renameTarget = { schema, table };
			renameValue = plan.value;
			showRenameModal = true;
			return;
		}
		if (plan.kind === 'editor_sql') {
			setActiveSql(plan.sql);
			globalError = plan.message;
			return;
		}
		if (plan.kind === 'editor_sql_clear_error') {
			setActiveSql(plan.sql);
			globalError = '';
			return;
		}
		const tabId = addDataTab(plan.title, plan.query, plan.context);
		await executeQuery(plan.query, {
			targetTabId: tabId,
			pushToHistory: false,
			context: plan.context,
		});
	}
	async function handleSchemaAction(action: SchemaAction, schema: string) {
		const plan = buildSchemaActionPlan({
			action,
			databaseType: connectionStatus.databaseType,
			schema,
		});
		if (plan.kind === 'copy') {
			await navigator.clipboard.writeText(plan.text);
			return;
		}
		const tabId = addDataTab(plan.title, plan.query, null);
		await executeQuery(plan.query, {
			targetTabId: tabId,
			pushToHistory: false,
			context: null,
		});
		globalError = '';
	}
	async function submitRename() {
		if (!renameTarget) return;
		const nextName = renameValue.trim();
		if (!nextName) {
			globalError = 'New table name is required.';
			return;
		}
		showRenameModal = false;
		const sql = buildRenameTableSql({
			schema: renameTarget.schema,
			table: renameTarget.table,
			nextName,
		});
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
	async function handleCreateDatabase(params: { name: string; encoding: string }) {
		if (!connectionStatus.connected) {
			globalError = 'No active connection';
			return;
		}
		if (connectionStatus.databaseType !== 'postgres') {
			globalError = 'Create database is only available for PostgreSQL connections.';
			return;
		}
		const name = params.name.trim();
		const encoding = params.encoding.trim().toUpperCase();
		if (!name) {
			globalError = 'Database name is required.';
			return;
		}
		const allowedEncodings = new Set(['UTF8', 'LATIN1', 'LATIN2', 'WIN1252']);
		if (!allowedEncodings.has(encoding)) {
			globalError = `Unsupported encoding: ${encoding}`;
			return;
		}
		try {
			await rpc.request.runQuery({
				sql: buildCreateDatabaseSql(name, encoding),
			});
			await loadDatabases();
			await handleDatabaseChange(name);
			if (connectionStatus.database === name) {
				globalError = '';
			}
		} catch (error) {
			globalError = error instanceof Error ? error.message : String(error);
		}
	}
	async function refreshDatabasesAndExplorer() {
		await loadDatabases();
		await loadExplorer();
	}
	async function refreshExplorerTables() {
		await loadExplorer();
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
		savedConnections = loadSavedConnectionsFromStorage({
			key: SAVED_CONNECTIONS_KEY,
			normalize: normalizeConnectionInput,
		});
		const restoredTabs = loadQueryTabsFromStorage(QUERY_TABS_KEY);
		if (restoredTabs.length > 0) {
			tabs = restoredTabs;
			activeTabId = restoredTabs[0].id;
		}
		queryFavorites = loadQueryFavoritesFromStorage(QUERY_FAVORITES_KEY);
		queryHistory = loadQueryHistoryFromStorage(QUERY_HISTORY_KEY);
		void (async () => {
			try {
				await initializeWorkspace({
					connectExternalSqliteFile,
					loadExternalSqlFile,
					setConnectionStatus: (status) => (connectionStatus = status),
					loadDatabases,
					loadExplorer: () => loadExplorer(),
					restoreDataTabResults,
				});
			} catch (error) {
				console.error('Failed to initialize app', error);
			}
		})();
		const onWindowResize = () => {
			if (!sqlSplitContainer) return;
			const total = sqlSplitContainer.clientHeight;
			resultsPaneHeight = clampResultsHeight(resultsPaneHeight, total);
		};

		const isEditableTarget = (target: EventTarget | null) => {
			if (!(target instanceof HTMLElement)) return false;
			// allow Ctrl+T/W inside CodeMirror to still work (editor is contenteditable but we want shortcuts)
			if (target.closest('.cm-editor, .cm-content')) return false;
			if (target.isContentEditable) return true;
			const tag = target.tagName.toLowerCase();
			return tag === 'input' || tag === 'textarea' || tag === 'select';
		};

		const onGlobalShortcuts = (event: KeyboardEvent) => {
			if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
			const key = event.key.toLowerCase();
			if (key === 'k') {
				event.preventDefault();
				showSearchPalette = !showSearchPalette;
				return;
			}
			if (isEditableTarget(event.target)) return;

			if (key === 'n') {
				event.preventDefault();
				addQueryTab();
				mainView = 'sql';
				return;
			}

			if (key === 'x') {
				event.preventDefault();
				if (activeTabId) closeTab(activeTabId);
			}
		};

		const onContextMenu = (e: MouseEvent) => e.preventDefault();
		window.addEventListener('contextmenu', onContextMenu);
		window.addEventListener('resize', onWindowResize);
		window.addEventListener('keydown', onGlobalShortcuts);
		return () => {
			window.removeEventListener('contextmenu', onContextMenu);
			window.removeEventListener('resize', onWindowResize);
			window.removeEventListener('keydown', onGlobalShortcuts);
		};
	});
	onDestroy(() => {
		stopResultsResize();
	});
	$effect(() => {
		persistJsonValue(QUERY_TABS_KEY, toPersistedTabs(tabs));
	});
</script>
<main
	class="h-screen w-full flex flex-col bg-gray-50 overflow-hidden text-sm text-gray-800 antialiased"
>
	<AppHeader
		{connectionStatus}
		onCreateConnection={startCreateConnection}
		onDisconnect={handleDisconnect}
	/>
	{#if !connectionStatus.connected && !forceWorkspaceOnDisconnect}
		<DisconnectedWorkspace
			{savedConnections}
			{connectingName}
			{connectionSearch}
			onConnect={connectSaved}
			onCreate={startCreateConnection}
			onEdit={startEditConnection}
			onDelete={removeSavedConnection}
		/>
	{:else}
		<WorkspaceBody
			{mainView}
			{connectionStatus}
			{explorer}
			isExplorerLoading={isExplorerLoading}
			{databases}
			{explorerSearch}
			{expandedSchemas}
			{expandedTables}
			{tabs}
			{activeTabId}
			{tabContextMenu}
			{activeTab}
			{resizingResults}
			{resultsPaneHeight}
			{isRunningQuery}
			{sqlCompletions}
			{globalError}
			{queryDurationMs}
			{favoritesForConnection}
			{selectedSavedQueryId}
			{selectedSavedQuery}
			{historyForConnection}
			{selectedHistoryIndex}
			{selectedHistoryQuery}
			onSelectView={(view) => (mainView = view)}
			onChangeDatabase={handleDatabaseChange}
			onSearchChange={(value) => (explorerSearch = value)}
			onToggleSchema={toggleSchema}
			onToggleTable={toggleTable}
			onRefreshDatabases={refreshDatabasesAndExplorer}
			onRefreshTables={refreshExplorerTables}
			onCreateDatabase={handleCreateDatabase}
			onTableAction={handleTableAction}
			onSchemaAction={handleSchemaAction}
			onFollowForeignKey={followForeignKey}
			onSelectTab={(tabId) => {
				activeTabId = tabId;
				tabContextMenu = null;
			}}
			onOpenTabContextMenu={openTabContextMenu}
			onCloseTab={(tabId) => {
				closeTab(tabId);
				tabContextMenu = null;
			}}
			onAddTab={() => addQueryTab()}
			onCloseContextMenu={() => (tabContextMenu = null)}
			onCloseAllTabs={() => {
				tabs = [];
				activeTabId = '';
				tabContextMenu = null;
			}}
			onCloseAllTabsBut={(tabId) => {
				const target = tabs.find((tab) => tab.id === tabId);
				if (!target) return;
				tabs = [target];
				activeTabId = target.id;
				tabContextMenu = null;
			}}
			onOpenDefaultQueryTab={() => addQueryTab('SELECT 1;')}
			onSetActiveSql={setActiveSql}
			onRunQuery={handleRunQuery}
			onSaveQuery={saveActiveQuery}
			onFormatQuery={formatActiveQuery}
			onRunSqlForTab={(query, targetTabId, context) =>
				executeQuery(query, { pushToHistory: false, targetTabId, context })}
			onApplyTableChanges={applyTableChanges}
			onStartResultsResize={startResultsResize}
			onSetSplitContainer={(el) => (sqlSplitContainer = el)}
			onSelectSavedQuery={(id) => (selectedSavedQueryId = id)}
			onDeleteSavedQuery={deleteSavedQuery}
			onOpenSavedQuery={openSavedQuery}
			onSelectHistory={(index) => (selectedHistoryIndex = index)}
		/>
	{/if}
	<WorkspaceModals
		{showConnectionModal}
		{editingConnectionName}
		{connectionInputMode}
		{connectionForm}
		{connectionStringInput}
		{testConnectionMessage}
		{testConnectionOk}
		{isTestingConnection}
		{isConnecting}
		{showRenameModal}
		{renameTarget}
		{renameValue}
		onCloseConnectionModal={() => {
			showConnectionModal = false;
			editingConnectionName = null;
		}}
		onModeChange={(mode) => {
			connectionInputMode = connectionForm.databaseType === 'sqlite' ? 'fields' : mode;
		}}
		onConnectionFormChange={(next) => (connectionForm = next)}
		onConnectionStringChange={(value) => (connectionStringInput = value)}
		onTestConnection={handleTestConnection}
		onSaveAndConnect={() => handleConnect(true)}
		onRenameValueChange={(value) => (renameValue = value)}
		onCloseRenameModal={() => (showRenameModal = false)}
		onSubmitRename={submitRename}
	/>
	<StatusBar />
	<SearchPalette
		open={showSearchPalette}
		searchQuery={explorerSearch}
		{explorer}
		onSearchChange={(v) => (explorerSearch = v)}
		onClose={() => (showSearchPalette = false)}
		onSelectTable={handlePaletteSelectTable}
		onSelectSchema={handlePaletteSelectSchema}
	/>
</main>
