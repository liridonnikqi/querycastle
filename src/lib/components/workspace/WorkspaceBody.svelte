<script lang="ts">
	import LibraryView from '$lib/components/workspace/LibraryView.svelte';
	import MainNav from '$lib/components/workspace/MainNav.svelte';
	import SqlWorkspace from '$lib/components/workspace/SqlWorkspace.svelte';
	import type {
		ApplyTableChangesResult,
		ConnectionStatus,
		DatabaseExplorer,
		TableChangesPayload,
	} from '$lib/rpc';
	import type { QueryHistoryItem, SavedQueryItem } from '$lib/types';
	import type { MainView, SchemaAction, TableAction, TabContextMenu, WorkspaceTab } from '$lib/utils/workspace';

	let {
		mainView,
		connectionStatus,
		explorer,
		isExplorerLoading,
		databases,
		explorerSearch,
		expandedSchemas,
		expandedTables,
		tabs,
		activeTabId,
		tabContextMenu,
		activeTab,
		resizingResults,
		resultsPaneHeight,
		isRunningQuery,
		sqlCompletions,
		globalError,
		queryDurationMs,
		favoritesForConnection,
		selectedSavedQueryId,
		selectedSavedQuery,
		historyForConnection,
		selectedHistoryIndex,
		selectedHistoryQuery,
		onSelectView,
		onChangeDatabase,
		onSearchChange,
		onToggleSchema,
		onToggleTable,
		onRefreshDatabases,
		onRefreshTables,
		onCreateDatabase,
		onTableAction,
		onSchemaAction,
		onFollowForeignKey,
		onSelectTab,
		onOpenTabContextMenu,
		onCloseTab,
		onAddTab,
		onCloseContextMenu,
		onCloseAllTabs,
		onCloseAllTabsBut,
		onOpenDefaultQueryTab,
		onSetActiveSql,
		onRunQuery,
		onSaveQuery,
		onFormatQuery,
		onRunSqlForTab,
		onApplyTableChanges,
		onStartResultsResize,
		onSetSplitContainer,
		onSelectSavedQuery,
		onDeleteSavedQuery,
		onOpenSavedQuery,
		onSelectHistory,
	}: {
		mainView: MainView;
		connectionStatus: ConnectionStatus;
		explorer: DatabaseExplorer | null;
		isExplorerLoading: boolean;
		databases: string[];
		explorerSearch: string;
		expandedSchemas: Set<string>;
		expandedTables: Set<string>;
		tabs: WorkspaceTab[];
		activeTabId: string;
		tabContextMenu: TabContextMenu;
		activeTab: WorkspaceTab | null;
		resizingResults: boolean;
		resultsPaneHeight: number;
		isRunningQuery: boolean;
		sqlCompletions: string[];
		globalError: string;
		queryDurationMs: number;
		favoritesForConnection: SavedQueryItem[];
		selectedSavedQueryId: string;
		selectedSavedQuery: SavedQueryItem | null;
		historyForConnection: QueryHistoryItem[];
		selectedHistoryIndex: number;
		selectedHistoryQuery: QueryHistoryItem | null;
		onSelectView: (view: MainView) => void;
		onChangeDatabase: (database: string) => void;
		onSearchChange: (value: string) => void;
		onToggleSchema: (schema: string) => void;
		onToggleTable: (schema: string, table: string) => void;
		onRefreshDatabases: () => void | Promise<void>;
		onRefreshTables: () => void | Promise<void>;
		onCreateDatabase: (params: { name: string; encoding: string }) => void | Promise<void>;
		onTableAction: (action: TableAction, schema: string, table: string) => void | Promise<void>;
		onSchemaAction: (action: SchemaAction, schema: string) => void | Promise<void>;
		onFollowForeignKey: (schema: string, table: string) => void | Promise<void>;
		onSelectTab: (tabId: string) => void;
		onOpenTabContextMenu: (event: MouseEvent, tabId: string) => void;
		onCloseTab: (tabId: string) => void;
		onAddTab: () => void;
		onCloseContextMenu: () => void;
		onCloseAllTabs: () => void;
		onCloseAllTabsBut: (tabId: string) => void;
		onOpenDefaultQueryTab: () => void;
		onSetActiveSql: (sql: string) => void;
		onRunQuery: (queryOverride?: string) => void | Promise<void>;
		onSaveQuery: () => void;
		onFormatQuery: () => void;
		onRunSqlForTab: (
			query: string,
			targetTabId: string,
			context: { schema: string; table: string } | null,
		) => Promise<void>;
		onApplyTableChanges: (
			context: { schema: string; table: string },
			changes: TableChangesPayload,
		) => Promise<ApplyTableChangesResult>;
		onStartResultsResize: (event: PointerEvent) => void;
		onSetSplitContainer: (el: HTMLDivElement | null) => void;
		onSelectSavedQuery: (id: string) => void;
		onDeleteSavedQuery: (id: string) => void;
		onOpenSavedQuery: (sql: string) => void;
		onSelectHistory: (index: number) => void;
	} = $props();
</script>

<div class="flex flex-1 overflow-hidden">
	<MainNav {mainView} onSelectView={onSelectView} />

	{#if mainView === 'sql'}
		<SqlWorkspace
			{connectionStatus}
			explorer={explorer}
			loadingExplorer={isExplorerLoading}
			{databases}
			{explorerSearch}
			{expandedSchemas}
			{expandedTables}
			onChangeDatabase={onChangeDatabase}
			onSearchChange={onSearchChange}
			onToggleSchema={onToggleSchema}
			onToggleTable={onToggleTable}
			onRefreshDatabases={onRefreshDatabases}
			onRefreshTables={onRefreshTables}
			onCreateDatabase={onCreateDatabase}
			onTableAction={onTableAction}
			onSchemaAction={onSchemaAction}
			onFollowForeignKey={onFollowForeignKey}
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
			onSelectTab={onSelectTab}
			onOpenTabContextMenu={onOpenTabContextMenu}
			onCloseTab={onCloseTab}
			onAddTab={onAddTab}
			onCloseContextMenu={onCloseContextMenu}
			onCloseAllTabs={onCloseAllTabs}
			onCloseAllTabsBut={onCloseAllTabsBut}
			onOpenDefaultQueryTab={onOpenDefaultQueryTab}
			onSetActiveSql={onSetActiveSql}
			onRunQuery={onRunQuery}
			onSaveQuery={onSaveQuery}
			onFormatQuery={onFormatQuery}
			onRunSqlForTab={onRunSqlForTab}
			onApplyTableChanges={onApplyTableChanges}
			onStartResultsResize={onStartResultsResize}
			onSetSplitContainer={onSetSplitContainer}
		/>
	{:else}
		<LibraryView
			{mainView}
			{favoritesForConnection}
			{selectedSavedQueryId}
			{selectedSavedQuery}
			{historyForConnection}
			{selectedHistoryIndex}
			{selectedHistoryQuery}
			onSelectSavedQuery={onSelectSavedQuery}
			onDeleteSavedQuery={onDeleteSavedQuery}
			onOpenSavedQuery={onOpenSavedQuery}
			onSelectHistory={onSelectHistory}
		/>
	{/if}
</div>
