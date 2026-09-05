<script lang="ts">
	import ExplorerSidebar from '$lib/components/explorer/ExplorerSidebar.svelte';
	import QueryTabsBar from '$lib/components/workspace/QueryTabsBar.svelte';
	import DiagramView from '$lib/components/workspace/DiagramView.svelte';
	import ResultsPane from '$lib/components/query/ResultsPane.svelte';
	import SqlEditor from '$lib/components/query/SqlEditor.svelte';
	import type { Workspace } from '$lib/workspace/controller.svelte';

	let { workspace }: { workspace: Workspace } = $props();

	let splitContainer = $state<HTMLDivElement | null>(null);

	const activeTab = $derived(workspace.activeTab);
	const showQueryResults = $derived(
		Boolean(
			activeTab?.kind === 'query' &&
				(workspace.isRunningQuery ||
					(activeTab.lastRunSql?.trim().length ?? 0) > 0 ||
					(activeTab.sqlError?.trim().length ?? 0) > 0 ||
					(activeTab.result.columns.length ?? 0) > 0),
		),
	);

	$effect(() => {
		workspace.sqlSplitContainer = splitContainer;
	});
</script>

<ExplorerSidebar
	connectionStatus={workspace.connectionStatus}
	explorer={workspace.explorer}
	loadingExplorer={workspace.isExplorerLoading}
	databases={workspace.databases}
	searchQuery={workspace.explorerSearch}
	onChangeDatabase={(database) => void workspace.handleDatabaseChange(database)}
	onSearchChange={(value) => (workspace.explorerSearch = value)}
	onRefreshDatabases={() => workspace.loadDatabases()}
	onRefreshTables={() => workspace.loadExplorer()}
	onCreateDatabase={(params) => workspace.handleCreateDatabase(params)}
	onTableAction={(action, schema, table) =>
		void workspace.handleTableAction(action, schema, table)}
	onSchemaAction={(action, schema) =>
		void workspace.handleSchemaAction(action, schema)}
	onOpenObjectDefinition={(params) =>
		void workspace.openObjectDefinition(params)}
	onViewSequence={(schema, name) => void workspace.viewSequence(schema, name)}
	activeTable={activeTab?.resultContext ?? null}
	savedQueries={workspace.favoritesForConnection}
	historyItems={workspace.historyForConnection}
	onAddTab={() => workspace.addQueryTab()}
	onOpenDiagram={() => workspace.addDiagramTab()}
	onOpenSavedQuery={(sql) => workspace.openSavedQuery(sql)}
	onOpenHistory={(index) => (workspace.selectedHistoryIndex = index)}
	activeTabKind={activeTab?.kind ?? null}
/>

<section class="flex-1 relative flex flex-col min-w-0 min-h-0 bg-qc-bg">
	<QueryTabsBar
		tabs={workspace.tabs}
		activeTabId={workspace.activeTabId}
		tabContextMenu={workspace.tabContextMenu}
		onSelectTab={(tabId) => workspace.selectTab(tabId)}
		onOpenContextMenu={(event, tabId) =>
			workspace.openTabContextMenu(event, tabId)}
		onCloseTab={(tabId) => workspace.closeTab(tabId)}
		onAddTab={() => workspace.addQueryTab()}
		onCloseContextMenu={() => (workspace.tabContextMenu = null)}
		onCloseAllTabs={() => workspace.closeAllTabs()}
		onCloseAllTabsBut={(tabId) => workspace.closeAllTabsBut(tabId)}
	/>

	{#if activeTab?.kind === 'query'}
		<div
			bind:this={splitContainer}
			class={`flex-1 flex flex-col min-h-0 bg-qc-bg ${workspace.resizingResults ? 'select-none cursor-row-resize' : ''}`}
		>
			<SqlEditor
				value={activeTab.sql}
				onChange={(sql) => workspace.setActiveSql(sql)}
				onRun={(query) => void workspace.handleRunQuery(query)}
				onSaveQuery={() => workspace.saveActiveQuery()}
				onFormatQuery={() => workspace.formatActiveQuery()}
				running={workspace.isRunningQuery}
				disabled={!workspace.connectionStatus.connected}
				explorer={workspace.explorer}
				databaseType={workspace.connectionStatus.databaseType}
			/>

			{#if showQueryResults}
				<button
					type="button"
					aria-label="Resize results panel"
					onpointerdown={(event) => workspace.startResultsResize(event)}
					class="h-1.5 bg-qc-panel hover:bg-qc-cell/40 cursor-row-resize transition-colors shrink-0 z-20 relative flex items-center justify-center"
				>
					<div
						class="w-8 h-0.5 bg-qc-muted/50 rounded-full pointer-events-none"
					></div>
				</button>

				<div
					style={`height:${workspace.resultsPaneHeight}px;`}
					class="flex flex-col bg-qc-bg shrink-0 min-h-0"
				>
					<ResultsPane
						result={activeTab.result}
						sqlError={activeTab.sqlError || workspace.globalError}
						databaseType={workspace.connectionStatus.databaseType}
						resultContext={activeTab.resultContext}
						explorer={workspace.explorer}
						relationTrail={activeTab.relationTrail ?? []}
						loading={workspace.isRunningQuery}
						refreshSql={activeTab.lastRunSql}
						resultKey={`${activeTab.id}:${activeTab.lastRunSql}:${activeTab.result.durationMs}:${activeTab.result.rowCount}`}
						runQuery={(sql) => workspace.runSessionQuery(sql)}
						onRunSql={(query) =>
							workspace.executeQuery(query, {
								pushToHistory: false,
								targetTabId: activeTab.id,
								context: activeTab.resultContext,
							})}
						onApplyTableChanges={(context, changes) =>
							workspace.applyTableChanges(context, changes)}
						onFollowRelation={(hop) => workspace.followRelation(hop)}
						onActivateRelationTrail={(index) =>
							workspace.activateRelationTrail(index)}
						durationMs={activeTab.result.durationMs ||
							workspace.queryDurationMs}
					/>
				</div>
			{/if}
		</div>
	{:else if activeTab?.kind === 'diagram'}
		<div class="flex-1 min-w-0 min-h-0 flex flex-col">
			<DiagramView
				connectionStatus={workspace.connectionStatus}
				explorer={workspace.explorer}
				loadingExplorer={workspace.isExplorerLoading}
				onRefreshTables={() => workspace.loadExplorer()}
				onTableAction={(action, schema, table) =>
					void workspace.handleTableAction(action, schema, table)}
				embedded
			/>
		</div>
	{:else if activeTab}
		<div class="flex-1 min-w-0 min-h-0 flex flex-col">
			<ResultsPane
				result={activeTab.result}
				sqlError={activeTab.sqlError || workspace.globalError}
				databaseType={workspace.connectionStatus.databaseType}
				resultContext={activeTab.resultContext}
				explorer={workspace.explorer}
				relationTrail={activeTab.relationTrail ?? []}
				loading={workspace.isRunningQuery}
				refreshSql={activeTab.lastRunSql}
				resultKey={`${activeTab.id}:${activeTab.lastRunSql}:${activeTab.result.durationMs}:${activeTab.result.rowCount}`}
				runQuery={(sql) => workspace.runSessionQuery(sql)}
				onRunSql={(query) =>
					workspace.executeQuery(query, {
						pushToHistory: false,
						targetTabId: activeTab.id,
						context: activeTab.resultContext,
					})}
				onApplyTableChanges={(context, changes) =>
					workspace.applyTableChanges(context, changes)}
				onFollowRelation={(hop) => workspace.followRelation(hop)}
				onActivateRelationTrail={(index) =>
					workspace.activateRelationTrail(index)}
				durationMs={activeTab.result.durationMs || workspace.queryDurationMs}
			/>
		</div>
	{:else}
		<div class="flex-1 flex items-center justify-center p-8 bg-qc-bg">
			<div class="w-full max-w-sm">
				<div class="text-center mb-6">
					<div class="text-sm font-medium text-qc-fg">Quick Shortcuts</div>
					<div class="text-xs text-qc-muted mt-1">
						No tabs open, use these shortcuts to get started
					</div>
				</div>
				<div class="space-y-0 divide-y divide-qc-border">
					<div class="flex items-center justify-between px-3 py-2 text-xs">
						<span class="text-qc-muted">Run Query</span>
						<span
							class="px-2 py-0.5 rounded bg-qc-elevated border border-qc-border font-mono text-[11px] text-qc-subtle"
							>Ctrl+Enter</span
						>
					</div>
					<div class="flex items-center justify-between px-3 py-2 text-xs">
						<span class="text-qc-muted">Save Query</span>
						<span
							class="px-2 py-0.5 rounded bg-qc-elevated border border-qc-border font-mono text-[11px] text-qc-subtle"
							>Ctrl+S</span
						>
					</div>
					<div class="flex items-center justify-between px-3 py-2 text-xs">
						<span class="text-qc-muted">Format SQL</span>
						<span
							class="px-2 py-0.5 rounded bg-qc-elevated border border-qc-border font-mono text-[11px] text-qc-subtle"
							>Shift+Alt+F</span
						>
					</div>
					<div class="flex items-center justify-between px-3 py-2 text-xs">
						<span class="text-qc-muted">New Query Tab</span>
						<span
							class="px-2 py-0.5 rounded bg-qc-elevated border border-qc-border font-mono text-[11px] text-qc-subtle"
							>Ctrl+N</span
						>
					</div>
					<div class="flex items-center justify-between px-3 py-2 text-xs">
						<span class="text-qc-muted">Close Tab</span>
						<span
							class="px-2 py-0.5 rounded bg-qc-elevated border border-qc-border font-mono text-[11px] text-qc-subtle"
							>Ctrl+X</span
						>
					</div>
					<div class="flex items-center justify-between px-3 py-2 text-xs">
						<span class="text-qc-muted">Search</span>
						<span
							class="px-2 py-0.5 rounded bg-qc-elevated border border-qc-border font-mono text-[11px] text-qc-subtle"
							>Ctrl+K</span
						>
					</div>
				</div>
			</div>
		</div>
	{/if}
</section>
