import { format as formatSql } from 'sql-formatter';
import type {
	ApplyTableChangesResult,
	ConnectionInput,
	ConnectionStatus,
	DatabaseExplorer,
	ObjectDefinitionParams,
	TableChangesPayload,
} from '$lib/rpc';
import { rpc } from '$lib/rpc-client';
import type { QueryHistoryItem, SavedQueryItem } from '$lib/types';
import { dialectCapabilities, engineDisplayName } from '$lib/utils/dialect';
import { normalizeConnectionInput } from '$lib/utils/connection';
import { tryBuildEditableQuery } from '$lib/utils/editable-query';
import {
	removeSession,
	sessionIdOf,
	snapshotSession,
	upsertSession,
	type LiveWorkspace,
	type OpenSession,
} from '$lib/utils/open-session';
import {
	loadQueryFavoritesFromStorage,
	loadQueryHistoryFromStorage,
	loadQueryTabsFromStorage,
	loadSavedConnectionsFromStorage,
	persistJsonValue,
	toPersistedTabs,
} from '$lib/utils/persistence';
import {
	buildFollowSqlFromHop,
	buildFollowTabTitle,
	trailsEqual,
} from '$lib/utils/relation-sql';
import {
	collectExplorerIdentifiers,
	definitionTabTitle,
	isExplorerView,
} from '$lib/utils/schema-objects';
import { quoteCatalogIdentifiersInSql, quoteSqlIdentifier } from '$lib/utils/sql';
import {
	closeTabState,
	createDataTab as makeDataTab,
	createDiagramTab,
	createQueryTab,
	insertTabAfter,
	setSqlInReusableQueryTabState,
} from '$lib/utils/tabs';
import {
	QUERY_FAVORITES_KEY,
	QUERY_HISTORY_KEY,
	QUERY_TABS_KEY,
	SAVED_CONNECTIONS_KEY,
	clampResultsHeight,
	createDefaultTab,
	deriveFavoriteTitle,
	nowLabel,
	type RelationHop,
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

function disconnectedStatus(): ConnectionStatus {
	return {
		connected: false,
		databaseType: 'postgres',
		name: 'Disconnected',
		host: '',
		port: 5432,
		database: '',
		user: '',
		serverVersion: null,
		sessionId: '',
	};
}

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

export class Workspace {
	connectionStatus = $state<ConnectionStatus>(disconnectedStatus());
	explorerSearch = $state('');
	connectionSearch = $state('');
	tabs = $state<WorkspaceTab[]>([]);
	activeTabId = $state('');
	queryHistory = $state<QueryHistoryItem[]>([]);
	queryFavorites = $state<SavedQueryItem[]>([]);
	explorer = $state<DatabaseExplorer | null>(null);
	isExplorerLoading = $state(false);
	databases = $state<string[]>([]);
	savedConnections = $state<ConnectionInput[]>([]);
	showConnectionModal = $state(false);
	editingConnectionName = $state<string | null>(null);
	isTestingConnection = $state(false);
	isConnecting = $state(false);
	isRunningQuery = $state(false);
	testConnectionMessage = $state('');
	testConnectionOk = $state(false);
	globalError = $state('');
	connectingName = $state<string | null>(null);
	showRenameModal = $state(false);
	renameTarget = $state<{ schema: string; table: string } | null>(null);
	renameValue = $state('');
	tabContextMenu = $state<TabContextMenu>(null);
	queryDurationMs = $state(0);
	connectionInputMode = $state<'fields' | 'string'>('fields');
	connectionStringInput = $state('');
	sqlSplitContainer = $state<HTMLDivElement | null>(null);
	resultsPaneHeight = $state(320);
	resizingResults = $state(false);
	forceWorkspaceOnDisconnect = $state(false);
	showSearchPalette = $state(false);
	openSessions = $state<OpenSession[]>([]);
	activeSessionId = $state('');
	activeSessionInput = $state<ConnectionInput>(normalizeConnectionInput({}));
	connectionForm = $state<ConnectionInput>(normalizeConnectionInput({}));
	selectedSavedQueryId = $state('');
	selectedHistoryIndex = $state(0);

	readonly activeTab = $derived(
		this.tabs.find((tab) => tab.id === this.activeTabId) ?? null,
	);
	readonly activeConnectionKey = $derived(
		this.connectionStatus.connected
			? `${this.connectionStatus.databaseType}:${this.connectionStatus.host}:${this.connectionStatus.port}/${this.connectionStatus.database}/${this.connectionStatus.user}`
			: 'disconnected',
	);
	readonly favoritesForConnection = $derived(
		this.connectionStatus.connected
			? this.queryFavorites.filter(
					(item) => item.connectionKey === this.activeConnectionKey,
				)
			: this.queryFavorites,
	);
	readonly historyForConnection = $derived(
		this.connectionStatus.connected
			? this.queryHistory.filter(
					(item) =>
						!item.connectionKey || item.connectionKey === this.activeConnectionKey,
				)
			: this.queryHistory,
	);
	readonly selectedSavedQuery = $derived(
		this.favoritesForConnection.find((item) => item.id === this.selectedSavedQueryId) ??
			this.favoritesForConnection[0] ??
			null,
	);
	readonly selectedHistoryQuery = $derived(
		this.historyForConnection[this.selectedHistoryIndex] ??
			this.historyForConnection[0] ??
			null,
	);

	private moveListener: ((event: PointerEvent) => void) | null = null;
	private upListener: (() => void) | null = null;
	private tabsPersistTimer: ReturnType<typeof setTimeout> | null = null;

	private persistTabsSoon() {
		if (this.tabsPersistTimer) clearTimeout(this.tabsPersistTimer);
		this.tabsPersistTimer = setTimeout(() => {
			persistJsonValue(QUERY_TABS_KEY, toPersistedTabs(this.tabs));
		}, 400);
	}

	private persistTabsNow() {
		if (this.tabsPersistTimer) {
			clearTimeout(this.tabsPersistTimer);
			this.tabsPersistTimer = null;
		}
		persistJsonValue(QUERY_TABS_KEY, toPersistedTabs(this.tabs));
	}

	private liveWorkspace(): LiveWorkspace {
		return {
			connectionStatus: this.connectionStatus,
			input: this.activeSessionInput,
			explorer: this.explorer,
			databases: this.databases,
			tabs: this.tabs,
			activeTabId: this.activeTabId,
			explorerSearch: this.explorerSearch,
			isExplorerLoading: this.isExplorerLoading,
			globalError: this.globalError,
			queryDurationMs: this.queryDurationMs,
		};
	}

	private restoreSession(session: OpenSession) {
		this.connectionStatus = session.status;
		this.activeSessionInput = session.input;
		this.activeSessionId = session.id;
		this.explorer = session.explorer;
		this.databases = session.databases;
		this.tabs = session.tabs;
		this.activeTabId = session.activeTabId;
		this.explorerSearch = session.explorerSearch;
		this.isExplorerLoading = session.isExplorerLoading;
		this.globalError = session.globalError;
		this.queryDurationMs = session.queryDurationMs;
		this.tabContextMenu = null;
	}

	private resetWorkspaceToEmpty() {
		this.explorer = null;
		this.databases = [];
		this.tabs = [];
		this.activeTabId = '';
		this.explorerSearch = '';
		this.isExplorerLoading = false;
		this.globalError = '';
		this.queryDurationMs = 0;
		this.tabContextMenu = null;
	}

	private stashActiveSession() {
		const id = sessionIdOf(this.connectionStatus) || this.activeSessionId;
		if (!id || !this.connectionStatus.connected) return;
		this.openSessions = upsertSession(
			this.openSessions,
			snapshotSession(id, this.liveWorkspace()),
		);
	}

	private ensureTab() {
		if (this.tabs.length > 0) return;
		const nextTab = createDefaultTab();
		this.tabs = [nextTab];
		this.activeTabId = nextTab.id;
		this.persistTabsSoon();
	}

	getActiveSql() {
		if (!this.activeTab || this.activeTab.kind !== 'query') return '';
		return this.activeTab.sql;
	}

	addQueryTab(initialSql = '', title?: string) {
		const nextTab = createQueryTab(
			this.tabs.filter((tab) => tab.kind === 'query').length,
			initialSql,
			title,
		);
		this.tabs = [...this.tabs, nextTab];
		this.activeTabId = nextTab.id;
		this.persistTabsSoon();
	}

	setSqlInReusableQueryTab(sql: string) {
		const next = setSqlInReusableQueryTabState({
			tabs: this.tabs,
			activeTabId: this.activeTabId,
			sql,
		});
		this.tabs = next.tabs;
		this.activeTabId = next.activeTabId;
		this.persistTabsSoon();
	}

	setActiveSql(nextSql: string) {
		this.setSqlInReusableQueryTab(nextSql);
	}

	addDiagramTab() {
		const existing = this.tabs.find((tab) => tab.kind === 'diagram');
		if (existing) {
			this.activeTabId = existing.id;
			return;
		}
		const nextTab = createDiagramTab();
		this.tabs = [...this.tabs, nextTab];
		this.activeTabId = nextTab.id;
		this.persistTabsSoon();
	}

	addDataTab(
		title: string,
		sql: string,
		context: { schema: string; table: string } | null,
		options?: { relationTrail?: RelationHop[]; insertAfterActive?: boolean },
	) {
		const nextTab = makeDataTab({
			title,
			sql,
			context,
			relationTrail: options?.relationTrail ?? [],
		});
		if (options?.insertAfterActive) {
			const inserted = insertTabAfter({
				tabs: this.tabs,
				activeTabId: this.activeTabId,
				tab: nextTab,
			});
			this.tabs = inserted.tabs;
			this.activeTabId = inserted.activeTabId;
		} else {
			this.tabs = [...this.tabs, nextTab];
			this.activeTabId = nextTab.id;
		}
		this.persistTabsSoon();
		return nextTab.id;
	}

	closeTab(tabId: string) {
		const next = closeTabState({
			tabs: this.tabs,
			activeTabId: this.activeTabId,
			tabId,
		});
		this.tabs = next.tabs;
		this.activeTabId = next.activeTabId;
		this.tabContextMenu = null;
		this.persistTabsSoon();
	}

	closeAllTabs() {
		this.tabs = [];
		this.activeTabId = '';
		this.tabContextMenu = null;
		this.persistTabsSoon();
	}

	closeAllTabsBut(tabId: string) {
		const target = this.tabs.find((tab) => tab.id === tabId);
		if (!target) return;
		this.tabs = [target];
		this.activeTabId = target.id;
		this.tabContextMenu = null;
		this.persistTabsSoon();
	}

	selectTab(tabId: string) {
		this.activeTabId = tabId;
		this.tabContextMenu = null;
	}

	openTabContextMenu(event: MouseEvent, tabId: string) {
		event.preventDefault();
		this.tabContextMenu = { x: event.clientX, y: event.clientY, tabId };
	}

	upsertSavedConnection(connection: ConnectionInput) {
		const index = this.savedConnections.findIndex(
			(item) => item.name === connection.name,
		);
		if (index === -1) this.savedConnections = [connection, ...this.savedConnections];
		else {
			this.savedConnections[index] = connection;
			this.savedConnections = [...this.savedConnections];
		}
		persistJsonValue(SAVED_CONNECTIONS_KEY, this.savedConnections);
	}

	removeSavedConnection(name: string) {
		this.savedConnections = this.savedConnections.filter((item) => item.name !== name);
		persistJsonValue(SAVED_CONNECTIONS_KEY, this.savedConnections);
	}

	saveActiveQuery() {
		if (!this.connectionStatus.connected) {
			this.globalError = 'Connect to a database before saving queries.';
			return;
		}
		const sql = this.getActiveSql().trim();
		if (!sql) return;
		const duplicate = this.queryFavorites.find(
			(item) =>
				item.connectionKey === this.activeConnectionKey && item.sql.trim() === sql,
		);
		if (duplicate) {
			this.globalError = 'This query is already saved for the current connection.';
			return;
		}
		const next: SavedQueryItem = {
			id: crypto.randomUUID(),
			title: deriveFavoriteTitle(sql),
			sql,
			createdAt: Date.now(),
			connectionKey: this.activeConnectionKey,
		};
		this.queryFavorites = [next, ...this.queryFavorites].slice(0, 200);
		persistJsonValue(QUERY_FAVORITES_KEY, this.queryFavorites);
		this.globalError = '';
	}

	private pushHistory(item: QueryHistoryItem) {
		this.queryHistory = [item, ...this.queryHistory].slice(0, 100);
		persistJsonValue(QUERY_HISTORY_KEY, this.queryHistory);
	}

	loadExternalSqlFile(path: string, content: string) {
		this.forceWorkspaceOnDisconnect = true;
		const normalizedPath = path.replaceAll('\\', '/');
		const fileName = normalizedPath.split('/').pop() || 'opened.sql';
		const targetTab =
			(this.activeTab && this.activeTab.kind === 'query' ? this.activeTab : null) ??
			this.tabs.find((tab) => tab.kind === 'query') ??
			null;
		if (!targetTab) {
			this.addQueryTab(content);
			const newestTab = this.tabs[this.tabs.length - 1];
			if (newestTab) {
				this.tabs = this.tabs.map((tab) =>
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
			this.tabs = this.tabs.map((tab) =>
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
			this.activeTabId = targetTab.id;
		}
		this.globalError = '';
		this.persistTabsSoon();
	}

	async connectExternalSqliteFile(path: string) {
		const normalizedPath = path.replaceAll('\\', '/');
		const fileName = normalizedPath.split('/').pop() || 'opened.db';
		const connectionName =
			fileName.replace(/\.(sqlite|sqlite3|db)$/i, '') || 'sqlite_db';
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
		this.connectionForm = payload;
		this.connectionInputMode = 'fields';
		this.connectionStringInput = '';
		try {
			this.connectionStatus = await rpc.request.connect(payload);
			await this.loadDatabases();
			await this.loadExplorer({ clearBeforeLoad: true });
			this.showConnectionModal = false;
			this.editingConnectionName = null;
			this.globalError = '';
		} catch (error) {
			this.globalError = errorMessage(error);
			this.showConnectionModal = true;
		}
	}

	formatActiveQuery() {
		if (!this.activeTab || this.activeTab.kind !== 'query') return;
		const sql = this.activeTab.sql.trim();
		if (!sql) return;
		try {
			const language = dialectCapabilities(this.connectionStatus.databaseType)
				.formatLanguage;
			const nextSql = formatSql(sql, { language, identifierCase: 'preserve' });
			this.setActiveSql(nextSql);
			this.globalError = '';
		} catch (error) {
			this.globalError = errorMessage(error);
		}
	}

	deleteSavedQuery(id: string) {
		this.queryFavorites = this.queryFavorites.filter((item) => item.id !== id);
		persistJsonValue(QUERY_FAVORITES_KEY, this.queryFavorites);
	}

	clearSavedQueries() {
		this.queryFavorites = this.queryFavorites.filter(
			(item) => item.connectionKey !== this.activeConnectionKey,
		);
		persistJsonValue(QUERY_FAVORITES_KEY, this.queryFavorites);
		this.selectedSavedQueryId = '';
	}

	deleteHistoryItem(index: number) {
		let visibleCount = -1;
		for (let i = 0; i < this.queryHistory.length; i++) {
			const history = this.queryHistory[i];
			const isVisible = !this.connectionStatus.connected
				? true
				: !history.connectionKey || history.connectionKey === this.activeConnectionKey;
			if (isVisible) {
				visibleCount++;
				if (visibleCount === index) {
					this.queryHistory.splice(i, 1);
					this.queryHistory = [...this.queryHistory];
					persistJsonValue(QUERY_HISTORY_KEY, this.queryHistory);
					if (this.selectedHistoryIndex >= this.historyForConnection.length) {
						this.selectedHistoryIndex = Math.max(
							0,
							this.historyForConnection.length - 1,
						);
					}
					return;
				}
			}
		}
	}

	clearHistory() {
		const visibleSet = new Set(this.historyForConnection);
		this.queryHistory = this.queryHistory.filter((item) => !visibleSet.has(item));
		persistJsonValue(QUERY_HISTORY_KEY, this.queryHistory);
		this.selectedHistoryIndex = 0;
	}

	openSavedQuery(sql: string) {
		this.setSqlInReusableQueryTab(sql);
	}

	stopResultsResize() {
		this.resizingResults = false;
		if (this.moveListener) {
			window.removeEventListener('pointermove', this.moveListener);
			this.moveListener = null;
		}
		if (this.upListener) {
			window.removeEventListener('pointerup', this.upListener);
			window.removeEventListener('pointercancel', this.upListener);
			this.upListener = null;
		}
	}

	startResultsResize(event: PointerEvent) {
		if (!this.sqlSplitContainer) return;
		event.preventDefault();
		this.resizingResults = true;
		this.moveListener = (moveEvent: PointerEvent) => {
			if (!this.sqlSplitContainer) return;
			const rect = this.sqlSplitContainer.getBoundingClientRect();
			const total = this.sqlSplitContainer.clientHeight;
			const pointerOffset = moveEvent.clientY - rect.top;
			const nextHeight = total - pointerOffset;
			this.resultsPaneHeight = clampResultsHeight(nextHeight, total);
		};
		this.upListener = () => this.stopResultsResize();
		window.addEventListener('pointermove', this.moveListener);
		window.addEventListener('pointerup', this.upListener);
		window.addEventListener('pointercancel', this.upListener);
	}

	applyConnectionToForm(connection: ConnectionInput) {
		this.connectionForm = normalizeConnectionInput(connection);
		this.connectionInputMode =
			connection.databaseType !== 'sqlite' &&
			connection.useConnectionString &&
			connection.connectionString
				? 'string'
				: 'fields';
		this.connectionStringInput = connection.connectionString ?? '';
	}

	startEditConnection(connection: ConnectionInput) {
		this.applyConnectionToForm(connection);
		this.editingConnectionName = connection.name;
		this.showConnectionModal = true;
	}

	buildConnectionPayload(): ConnectionInput {
		if (
			this.connectionForm.databaseType !== 'sqlite' &&
			this.connectionInputMode === 'string'
		) {
			return {
				...this.connectionForm,
				useConnectionString: true,
				connectionString: this.connectionStringInput.trim(),
			};
		}
		return {
			...this.connectionForm,
			useConnectionString: false,
			connectionString: '',
		};
	}

	handlePaletteSelectTable(schema: string, table: string) {
		void this.handleTableAction('view_data', schema, table);
	}

	async loadExplorer(options?: { clearBeforeLoad?: boolean }) {
		if (!this.connectionStatus.connected) {
			this.explorer = null;
			this.isExplorerLoading = false;
			return;
		}
		if (options?.clearBeforeLoad) {
			this.explorer = null;
		}
		this.isExplorerLoading = true;
		try {
			this.explorer = await rpc.request.getDatabaseExplorer();
			this.globalError = '';
		} catch (error) {
			this.explorer = null;
			this.globalError = errorMessage(error);
		} finally {
			this.isExplorerLoading = false;
		}
	}

	async loadDatabases() {
		if (!this.connectionStatus.connected) {
			this.databases = [];
			return;
		}
		try {
			this.databases = await rpc.request.listDatabases();
		} catch {
			this.databases = this.connectionStatus.database
				? [this.connectionStatus.database]
				: [];
		}
	}

	async handleTestConnection() {
		this.isTestingConnection = true;
		this.testConnectionMessage = '';
		try {
			const engine = this.buildConnectionPayload().databaseType;
			const response = await rpc.request.testConnection(this.buildConnectionPayload());
			this.testConnectionOk = response.ok;
			this.testConnectionMessage = response.ok
				? `Connected successfully${response.serverVersion ? ` (${engineDisplayName(engine)} ${response.serverVersion})` : ''}`
				: response.message;
		} catch (error) {
			this.testConnectionOk = false;
			this.testConnectionMessage = errorMessage(error);
		} finally {
			this.isTestingConnection = false;
		}
	}

	openNewConnectionModal() {
		this.editingConnectionName = null;
		this.testConnectionMessage = '';
		this.testConnectionOk = false;
		this.showConnectionModal = true;
	}

	async switchOpenSession(id: string) {
		if (!id || id === this.activeSessionId) return;
		this.stashActiveSession();
		try {
			await rpc.request.switchSession(id);
		} catch (error) {
			this.globalError = errorMessage(error);
			return;
		}
		const next = this.openSessions.find((item) => item.id === id);
		if (next) this.restoreSession(next);
	}

	async closeOpenSession(id: string) {
		if (!id) return;
		if (id === this.activeSessionId || id === sessionIdOf(this.connectionStatus)) {
			this.stashActiveSession();
		}
		let nextStatus: ConnectionStatus;
		try {
			nextStatus = await rpc.request.disconnectSession(id);
		} catch (error) {
			this.globalError = errorMessage(error);
			return;
		}
		this.openSessions = removeSession(this.openSessions, id);
		if (id !== this.activeSessionId && id !== sessionIdOf(this.connectionStatus)) {
			return;
		}
		if (nextStatus.connected) {
			const nextId = sessionIdOf(nextStatus);
			const stored = this.openSessions.find((item) => item.id === nextId);
			if (stored) this.restoreSession(stored);
			else {
				this.connectionStatus = nextStatus;
				this.activeSessionId = nextId;
			}
			return;
		}
		this.openSessions = [];
		this.activeSessionId = '';
		this.connectionStatus = disconnectedStatus();
		this.resetWorkspaceToEmpty();
	}

	async handleConnect(saveConnection: boolean) {
		this.isConnecting = true;
		this.testConnectionMessage = '';
		try {
			const payload = this.buildConnectionPayload();
			if (
				saveConnection &&
				!this.editingConnectionName &&
				this.savedConnections.some((item) => item.name === payload.name)
			) {
				this.testConnectionOk = false;
				this.testConnectionMessage =
					'A connection with this name already exists. Choose a different name to save it.';
				return;
			}
			this.stashActiveSession();
			this.connectionStatus = await rpc.request.connect(payload);
			const id = sessionIdOf(this.connectionStatus) || crypto.randomUUID();
			this.connectionStatus = { ...this.connectionStatus, sessionId: id };
			if (saveConnection) {
				if (
					this.editingConnectionName &&
					this.editingConnectionName !== payload.name
				) {
					this.removeSavedConnection(this.editingConnectionName);
				}
				this.upsertSavedConnection(payload);
			}
			this.showConnectionModal = false;
			this.editingConnectionName = null;
			this.activeSessionInput = payload;
			this.activeSessionId = id;
			this.resetWorkspaceToEmpty();
			this.openSessions = upsertSession(
				this.openSessions,
				snapshotSession(id, this.liveWorkspace()),
			);
			await this.loadDatabases();
			await this.loadExplorer();
			this.stashActiveSession();
		} catch (error) {
			this.testConnectionOk = false;
			this.testConnectionMessage = errorMessage(error);
		} finally {
			this.isConnecting = false;
		}
	}

	async connectSaved(connection: ConnectionInput) {
		this.connectingName = connection.name;
		this.applyConnectionToForm(connection);
		try {
			await this.handleConnect(false);
		} finally {
			this.connectingName = null;
		}
	}

	async handleDisconnect() {
		const id = sessionIdOf(this.connectionStatus) || this.activeSessionId;
		if (id) {
			await this.closeOpenSession(id);
			return;
		}
		await rpc.request.disconnect();
		this.connectionStatus = disconnectedStatus();
		this.openSessions = [];
		this.activeSessionId = '';
		this.resetWorkspaceToEmpty();
		this.editingConnectionName = null;
	}

	async executeQuery(
		query: string,
		options?: {
			pushToHistory?: boolean;
			targetTabId?: string;
			context?: { schema: string; table: string } | null;
			historySql?: string;
		},
	) {
		this.isRunningQuery = true;
		const targetTabId = options?.targetTabId ?? this.activeTabId;
		try {
			const sql = quoteCatalogIdentifiersInSql(
				query,
				this.connectionStatus.databaseType,
				collectExplorerIdentifiers(this.explorer),
			);
			const queryResult = await rpc.request.runQuery({ sql });
			this.queryDurationMs = queryResult.durationMs;
			this.globalError = '';
			this.tabs = this.tabs.map((tab) =>
				tab.id === targetTabId
					? {
							...tab,
							result: queryResult,
							lastRunSql: sql,
							sqlError: '',
							resultContext: options?.context ?? tab.resultContext,
						}
					: tab,
			);
			const isDdl = /^\s*(create|drop|alter|truncate|rename)\b/i.test(query);
			const isDatabaseDdl = /^\s*(create|drop)\s+database\b/i.test(query);
			if (isDdl) {
				void this.loadExplorer().catch(() => {});
				if (isDatabaseDdl) void this.loadDatabases().catch(() => {});
			}
			if (options?.pushToHistory !== false) {
				this.pushHistory({
					time: nowLabel(),
					sql: options?.historySql ?? query,
					durationMs: queryResult.durationMs,
					success: true,
					connectionKey: this.activeConnectionKey,
				});
			}
		} catch (error) {
			const message = errorMessage(error);
			this.globalError = message;
			this.tabs = this.tabs.map((tab) =>
				tab.id === targetTabId ? { ...tab, sqlError: message } : tab,
			);
			if (options?.pushToHistory !== false) {
				this.pushHistory({
					time: nowLabel(),
					sql: options?.historySql ?? query,
					durationMs: 0,
					success: false,
					error: message,
					connectionKey: this.activeConnectionKey,
				});
			}
		} finally {
			this.isRunningQuery = false;
		}
	}

	async handleRunQuery(queryOverride?: string) {
		if (!this.connectionStatus.connected) {
			this.globalError = 'No active connection';
			return;
		}
		this.ensureTab();
		if (!this.activeTab || this.activeTab.kind !== 'query') return;
		const sqlToRun = (queryOverride ?? this.activeTab.sql).trim();
		if (!sqlToRun) {
			this.globalError = 'Query is empty';
			return;
		}
		const editablePlan = tryBuildEditableQuery({
			sql: sqlToRun,
			databaseType: this.connectionStatus.databaseType,
			explorer: this.explorer,
		});
		await this.executeQuery(editablePlan?.sql ?? sqlToRun, {
			pushToHistory: true,
			targetTabId: this.activeTab.id,
			context: editablePlan?.context ?? null,
			historySql: sqlToRun,
		});
	}

	async handleTableAction(action: TableAction, schema: string, table: string) {
		if (action === 'drop') {
			const isView = isExplorerView(this.explorer, schema, table);
			const ok = isView
				? confirm(`Drop view ${schema}.${table}?\nThis cannot be undone.`)
				: confirm(
						`Drop table ${schema}.${table} CASCADE?\nThis will also drop dependent objects and cannot be undone.`,
					);
			if (!ok) return;
		}
		if (action === 'truncate') {
			const ok = confirm(
				`Truncate table ${schema}.${table}? All rows will be deleted.`,
			);
			if (!ok) return;
		}
		const plan = buildTableActionPlan({
			action,
			databaseType: this.connectionStatus.databaseType,
			explorer: this.explorer,
			schema,
			table,
		});
		if (plan.kind === 'copy_name') {
			await navigator.clipboard.writeText(plan.text);
			return;
		}
		if (plan.kind === 'error') {
			this.globalError = plan.message;
			return;
		}
		if (plan.kind === 'rename') {
			this.renameTarget = { schema, table };
			this.renameValue = plan.value;
			this.showRenameModal = true;
			return;
		}
		if (plan.kind === 'editor_sql') {
			this.setActiveSql(plan.sql);
			this.globalError = plan.message;
			return;
		}
		if (plan.kind === 'editor_sql_clear_error') {
			this.setActiveSql(plan.sql);
			this.globalError = '';
			return;
		}
		if (action === 'drop' || action === 'truncate' || action === 'duplicate') {
			try {
				await rpc.request.runQuery({ sql: plan.query });
				this.globalError = '';
				await this.loadExplorer();
				if (action === 'drop' || action === 'duplicate') {
					await this.loadDatabases().catch(() => {});
				}
			} catch (error) {
				this.globalError = errorMessage(error);
			}
			return;
		}
		const tabId = this.addDataTab(plan.title, plan.query, plan.context);
		await this.executeQuery(plan.query, {
			targetTabId: tabId,
			pushToHistory: false,
			context: plan.context,
		});
	}

	async handleSchemaAction(action: SchemaAction, schema: string) {
		const plan = buildSchemaActionPlan({
			action,
			databaseType: this.connectionStatus.databaseType,
			schema,
		});
		if (plan.kind === 'copy') {
			await navigator.clipboard.writeText(plan.text);
			return;
		}
		const tabId = this.addDataTab(plan.title, plan.query, null);
		await this.executeQuery(plan.query, {
			targetTabId: tabId,
			pushToHistory: false,
			context: null,
		});
		this.globalError = '';
	}

	async submitRename() {
		if (!this.renameTarget) return;
		const nextName = this.renameValue.trim();
		if (!nextName) {
			this.globalError = 'New table name is required.';
			return;
		}
		this.showRenameModal = false;
		const sql = buildRenameTableSql({
			schema: this.renameTarget.schema,
			table: this.renameTarget.table,
			nextName,
		});
		try {
			await rpc.request.runQuery({ sql });
			this.globalError = '';
			await this.loadExplorer();
		} catch (error) {
			this.globalError = errorMessage(error);
		}
	}

	async openObjectDefinition(params: ObjectDefinitionParams) {
		try {
			const definition = await rpc.request.getObjectDefinition(params);
			this.addQueryTab(
				definition.sql,
				definitionTabTitle(params.kind, params.name, params.identityArgs) ||
					definition.title,
			);
			this.globalError = '';
		} catch (error) {
			this.globalError = errorMessage(error);
		}
	}

	async viewSequence(schema: string, name: string) {
		const safeSchema = quoteSqlIdentifier(
			this.connectionStatus.databaseType,
			schema,
		);
		const safeName = quoteSqlIdentifier(this.connectionStatus.databaseType, name);
		const sql = `select * from ${safeSchema}.${safeName};`;
		const tabId = this.addDataTab(name, sql, null);
		await this.executeQuery(sql, {
			targetTabId: tabId,
			pushToHistory: false,
			context: null,
		});
	}

	private findTabForTrailPrefix(
		prefix: RelationHop[],
		origin?: { schema: string; table: string },
	): WorkspaceTab | null {
		if (prefix.length === 0) {
			if (!origin) return null;
			const matches = this.tabs.filter(
				(tab) =>
					(tab.relationTrail?.length ?? 0) === 0 &&
					tab.resultContext?.schema === origin.schema &&
					tab.resultContext?.table === origin.table,
			);
			return matches.find((tab) => tab.kind === 'data') ?? matches[0] ?? null;
		}
		return this.tabs.find((tab) => trailsEqual(tab.relationTrail ?? [], prefix)) ?? null;
	}

	async openFollowTab(hop: RelationHop, previousTrail: RelationHop[]) {
		const sql = buildFollowSqlFromHop({
			databaseType: this.connectionStatus.databaseType,
			explorer: this.explorer,
			hop,
		});
		if (!sql) {
			this.globalError = 'Could not follow this relation.';
			return;
		}
		const tabId = this.addDataTab(
			buildFollowTabTitle(hop),
			sql,
			{ schema: hop.to.schema, table: hop.to.table },
			{
				relationTrail: [...previousTrail, hop],
				insertAfterActive: true,
			},
		);
		await this.executeQuery(sql, {
			targetTabId: tabId,
			pushToHistory: false,
			context: { schema: hop.to.schema, table: hop.to.table },
		});
	}

	async followRelation(hop: RelationHop) {
		await this.openFollowTab(hop, this.activeTab?.relationTrail ?? []);
	}

	async activateRelationTrail(crumbIndex: number) {
		const trail = this.activeTab?.relationTrail ?? [];
		if (trail.length === 0 || crumbIndex >= trail.length) return;
		if (crumbIndex === 0) {
			const origin = {
				schema: trail[0]!.from.schema,
				table: trail[0]!.from.table,
			};
			const existing = this.findTabForTrailPrefix([], origin);
			if (existing) {
				this.activeTabId = existing.id;
				return;
			}
			await this.handleTableAction('view_data', origin.schema, origin.table);
			return;
		}
		const prefix = trail.slice(0, crumbIndex);
		const existing = this.findTabForTrailPrefix(prefix);
		if (existing) {
			this.activeTabId = existing.id;
			return;
		}
		const hop = prefix[prefix.length - 1]!;
		await this.openFollowTab(hop, prefix.slice(0, -1));
	}

	async applyTableChanges(
		context: { schema: string; table: string },
		changes: TableChangesPayload,
	): Promise<ApplyTableChangesResult> {
		return rpc.request.applyTableChanges({
			schema: context.schema,
			table: context.table,
			changes,
		});
	}

	async handleDatabaseChange(database: string) {
		if (!database || database === this.connectionStatus.database) return;
		this.globalError = '';
		this.isExplorerLoading = true;
		this.explorer = null;
		try {
			this.connectionStatus = await rpc.request.selectDatabase({ database });
			this.activeSessionInput = { ...this.activeSessionInput, database };
			this.stashActiveSession();
			await this.loadDatabases();
			await this.loadExplorer({ clearBeforeLoad: true });
		} catch (error) {
			this.globalError = errorMessage(error);
			this.isExplorerLoading = false;
		}
	}

	async handleCreateDatabase(params: { name: string; encoding: string }) {
		if (!this.connectionStatus.connected) {
			this.globalError = 'No active connection';
			return;
		}
		if (!dialectCapabilities(this.connectionStatus.databaseType).canCreateDatabase) {
			this.globalError = 'Create database is only available for PostgreSQL connections.';
			return;
		}
		const name = params.name.trim();
		const encoding = params.encoding.trim().toUpperCase();
		if (!name) {
			this.globalError = 'Database name is required.';
			return;
		}
		const allowedEncodings = new Set(['UTF8', 'LATIN1', 'LATIN2', 'WIN1252']);
		if (!allowedEncodings.has(encoding)) {
			this.globalError = `Unsupported encoding: ${encoding}`;
			return;
		}
		try {
			await rpc.request.runQuery({
				sql: buildCreateDatabaseSql(name, encoding),
			});
			await this.loadDatabases();
			await this.handleDatabaseChange(name);
			if (this.connectionStatus.database === name) {
				this.globalError = '';
			}
		} catch (error) {
			this.globalError = errorMessage(error);
		}
	}

	async restoreDataTabResults() {
		if (!this.connectionStatus.connected) return;
		const dataTabs = this.tabs.filter(
			(tab) => tab.kind === 'data' && tab.sql.trim().length > 0,
		);
		for (const tab of dataTabs) {
			await this.executeQuery(tab.sql, {
				pushToHistory: false,
				targetTabId: tab.id,
				context: tab.resultContext,
			});
		}
	}

	init() {
		this.savedConnections = loadSavedConnectionsFromStorage({
			key: SAVED_CONNECTIONS_KEY,
			normalize: normalizeConnectionInput,
		});
		const restoredTabs = loadQueryTabsFromStorage(QUERY_TABS_KEY);
		if (restoredTabs.length > 0) {
			this.tabs = restoredTabs;
			this.activeTabId = restoredTabs[0].id;
		}
		this.queryFavorites = loadQueryFavoritesFromStorage(QUERY_FAVORITES_KEY);
		this.queryHistory = loadQueryHistoryFromStorage(QUERY_HISTORY_KEY);

		const onVisibility = () => {
			if (document.visibilityState === 'hidden') this.persistTabsNow();
		};
		document.addEventListener('visibilitychange', onVisibility);

		void (async () => {
			try {
				await initializeWorkspace({
					connectExternalSqliteFile: (path) => this.connectExternalSqliteFile(path),
					loadExternalSqlFile: (path, content) =>
						this.loadExternalSqlFile(path, content),
					setConnectionStatus: (status) => {
						this.connectionStatus = status;
						const id = sessionIdOf(status);
						if (status.connected && id && this.openSessions.length === 0) {
							this.activeSessionId = id;
							this.openSessions = [snapshotSession(id, this.liveWorkspace())];
						}
					},
					loadDatabases: () => this.loadDatabases(),
					loadExplorer: () => this.loadExplorer(),
					restoreDataTabResults: () => this.restoreDataTabResults(),
				});
			} catch (error) {
				console.error('Failed to initialize app', error);
			} finally {
				try {
					window.dispatchEvent(new CustomEvent('app:ready'));
				} catch {
					// ignore
				}
			}
		})();

		return () => {
			document.removeEventListener('visibilitychange', onVisibility);
			this.persistTabsNow();
			this.stopResultsResize();
		};
	}
}
