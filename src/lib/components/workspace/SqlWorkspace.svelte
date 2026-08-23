<script lang="ts">
	import ExplorerSidebar from '$lib/components/explorer/ExplorerSidebar.svelte';
	import QueryTabsBar from '$lib/components/workspace/QueryTabsBar.svelte';
	import ResultsPane from '$lib/components/query/ResultsPane.svelte';
	import SqlEditor from '$lib/components/query/SqlEditor.svelte';
	import type {
		ApplyTableChangesResult,
		ConnectionStatus,
		DatabaseExplorer,
		QueryResultPayload,
		TableChangesPayload,
	} from '$lib/rpc';
	import type { SchemaAction, TableAction, TabContextMenu, WorkspaceTab } from '$lib/utils/workspace';

	let {
		connectionStatus,
		explorer,
		loadingExplorer,
		databases,
		explorerSearch,
		expandedSchemas,
		expandedTables,
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
	}: {
		connectionStatus: ConnectionStatus;
		explorer: DatabaseExplorer | null;
		loadingExplorer: boolean;
		databases: string[];
		explorerSearch: string;
		expandedSchemas: Set<string>;
		expandedTables: Set<string>;
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
	} = $props();

	let splitContainer = $state<HTMLDivElement | null>(null);

	$effect(() => {
		onSetSplitContainer(splitContainer);
	});
</script>

<ExplorerSidebar
	{connectionStatus}
	{explorer}
	{loadingExplorer}
	{databases}
	searchQuery={explorerSearch}
	{expandedSchemas}
	{expandedTables}
	{onChangeDatabase}
	{onSearchChange}
	{onToggleSchema}
	{onToggleTable}
	{onRefreshDatabases}
	{onRefreshTables}
	{onCreateDatabase}
	{onTableAction}
	{onSchemaAction}
	{onFollowForeignKey}
/>

<section class="flex-1 relative flex flex-col min-w-0 min-h-0 bg-white border-l border-gray-100">
	<QueryTabsBar
		{tabs}
		{activeTabId}
		{tabContextMenu}
		{onSelectTab}
		onOpenContextMenu={onOpenTabContextMenu}
		{onCloseTab}
		{onAddTab}
		{onCloseContextMenu}
		{onCloseAllTabs}
		{onCloseAllTabsBut}
	/>

	{#if activeTab?.kind === 'query'}
		<div
			bind:this={splitContainer}
			class={`flex-1 flex flex-col min-h-0 bg-white ${resizingResults ? 'select-none cursor-row-resize' : ''}`}
		>
			<SqlEditor
				value={activeTab.sql}
				onChange={onSetActiveSql}
				onRun={onRunQuery}
				onSaveQuery={onSaveQuery}
				onFormatQuery={onFormatQuery}
				running={isRunningQuery}
				disabled={!connectionStatus.connected}
				completions={sqlCompletions}
			/>

			<button
				type="button"
				aria-label="Resize results panel"
				onpointerdown={onStartResultsResize}
				class="h-1.5 bg-gray-100 border-y border-gray-200 hover:bg-emerald-400/50 cursor-row-resize transition-colors shrink-0 z-20 relative flex items-center justify-center"
			>
				<div class="w-8 h-0.5 bg-gray-300 rounded-full pointer-events-none"></div>
			</button>

			<div style={`height:${resultsPaneHeight}px;`} class="flex flex-col bg-white shrink-0 min-h-0">
				<ResultsPane
					result={activeTab.result as QueryResultPayload}
					sqlError={activeTab.sqlError || globalError}
					databaseType={connectionStatus.databaseType}
					resultContext={activeTab.resultContext}
					loading={isRunningQuery}
					refreshSql={activeTab.lastRunSql}
					onRunSql={(query) =>
						onRunSqlForTab(query, activeTab.id, activeTab.resultContext)}
					{onApplyTableChanges}
					durationMs={activeTab.result.durationMs || queryDurationMs}
				/>
			</div>
		</div>
	{:else if activeTab}
		<div class="flex-1 min-w-0 min-h-0 flex flex-col">
			<ResultsPane
				result={activeTab.result as QueryResultPayload}
				sqlError={activeTab.sqlError || globalError}
				databaseType={connectionStatus.databaseType}
				resultContext={activeTab.resultContext}
				loading={isRunningQuery}
				refreshSql={activeTab.lastRunSql}
				onRunSql={(query) => onRunSqlForTab(query, activeTab.id, activeTab.resultContext)}
				{onApplyTableChanges}
				durationMs={activeTab.result.durationMs || queryDurationMs}
			/>
		</div>
	{:else}
		<div class="flex-1 flex items-center justify-center p-6 bg-white">
			<div class="w-full max-w-sm rounded-lg border border-gray-200 bg-white p-5">
				<div class="text-sm font-medium text-gray-900 mb-3">Quick Shortcuts</div>
				<div class="space-y-0 divide-y divide-gray-100 -mx-5 px-5">
					<div class="flex items-center justify-between py-2 text-xs">
						<span class="text-gray-600">Run Query</span><span class="px-2 py-0.5 rounded border border-gray-200 bg-gray-50 font-mono text-[11px]">Ctrl+Enter</span>
					</div>
					<div class="flex items-center justify-between py-2 text-xs">
						<span class="text-gray-600">Save Query</span><span class="px-2 py-0.5 rounded border border-gray-200 bg-gray-50 font-mono text-[11px]">Ctrl+S</span>
					</div>
					<div class="flex items-center justify-between py-2 text-xs">
						<span class="text-gray-600">Format SQL</span><span class="px-2 py-0.5 rounded border border-gray-200 bg-gray-50 font-mono text-[11px]">Shift+Alt+F</span>
					</div>
					<div class="flex items-center justify-between py-2 text-xs">
						<span class="text-gray-600">New Query Tab</span><span class="px-2 py-0.5 rounded border border-gray-200 bg-gray-50 font-mono text-[11px]">Ctrl+N</span>
					</div>
					<div class="flex items-center justify-between py-2 text-xs">
						<span class="text-gray-600">Close Active Tab</span><span class="px-2 py-0.5 rounded border border-gray-200 bg-gray-50 font-mono text-[11px]">Ctrl+X</span>
					</div>
					<div class="flex items-center justify-between py-2 text-xs">
						<span class="text-gray-600">Search</span><span class="px-2 py-0.5 rounded border border-gray-200 bg-gray-50 font-mono text-[11px]">Ctrl+K</span>
					</div>
				</div>
				<div class="mt-4">
					<button
						onclick={onOpenDefaultQueryTab}
						class="h-8 px-4 rounded-md bg-white border border-gray-200 text-gray-700 text-xs hover:bg-gray-50"
						>New Query Tab</button
					>
				</div>
			</div>
		</div>
	{/if}
</section>
