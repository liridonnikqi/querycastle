<script lang="ts">
	import {
		Braces,
		Check,
		ChevronDown,
		ChevronRight,
		Copy,
		Eye,
		Hash,
		History,
		Layers,
		Play,
		Plus,
		RefreshCw,
		Search,
		Star,
		SquarePen,
		Table2,
		Trash2,
		Database,
		Network,
	} from '@lucide/svelte';
	import type {
		ConnectionStatus,
		DatabaseExplorer,
		DatabaseRoutine,
		DatabaseSequence,
		ObjectDefinitionParams,
	} from '$lib/rpc';
	import type { QueryHistoryItem, SavedQueryItem } from '$lib/types';
	import { dialectCapabilities } from '$lib/utils/dialect';
	import type { SchemaAction, TableAction } from '$lib/utils/workspace';
	import ExplorerGroup from '$lib/components/explorer/ExplorerGroup.svelte';
	import {
		filterExplorer,
		routineSignature,
		schemaFunctions,
		schemaProcedures,
		schemaSequences,
		schemaTables,
		schemaViews,
	} from '$lib/utils/schema-objects';

	let {
		connectionStatus,
		explorer,
		loadingExplorer,
		databases,
		searchQuery,
		onChangeDatabase,
		onSearchChange,
		onRefreshDatabases,
		onRefreshTables,
		onCreateDatabase,
		onTableAction,
		onSchemaAction,
		onOpenObjectDefinition,
		onViewSequence,
		activeTable = null,
		savedQueries = [],
		historyItems = [],
		onAddTab,
		onOpenDiagram,
		onOpenSavedQuery,
		onOpenHistory,
		activeTabKind: _activeTabKind = null,
	}: {
		connectionStatus: ConnectionStatus;
		explorer: DatabaseExplorer | null;
		loadingExplorer: boolean;
		databases: string[];
		searchQuery: string;
		onChangeDatabase: (database: string) => void;
		onSearchChange: (value: string) => void;
		onRefreshDatabases: () => void | Promise<void>;
		onRefreshTables: () => void | Promise<void>;
		onCreateDatabase: (params: {
			name: string;
			encoding: string;
		}) => void | Promise<void>;
		onTableAction: (action: TableAction, schema: string, table: string) => void;
		onSchemaAction: (action: SchemaAction, schema: string) => void;
		onOpenObjectDefinition: (
			params: ObjectDefinitionParams,
		) => void | Promise<void>;
		onViewSequence: (schema: string, name: string) => void | Promise<void>;
		activeTable?: { schema: string; table: string } | null;
		savedQueries?: SavedQueryItem[];
		historyItems?: QueryHistoryItem[];
		onAddTab?: () => void;
		onOpenDiagram?: () => void;
		onOpenSavedQuery?: (sql: string) => void;
		onOpenHistory?: (index: number) => void;
		activeTabKind?: 'query' | 'data' | 'diagram' | null;
	} = $props();

	const skeletonRows = [0, 1, 2, 3, 4, 5];

	let contextMenu = $state<{
		x: number;
		y: number;
		schema: string;
		table: string;
		kind: string;
	} | null>(null);
	let schemaContextMenu = $state<{
		x: number;
		y: number;
		schema: string;
	} | null>(null);
	let objectContextMenu = $state<{
		x: number;
		y: number;
		kind: 'function' | 'procedure' | 'sequence' | 'index' | 'trigger' | 'view';
		schema: string;
		name: string;
		objectId?: string;
		identityArgs?: string;
		table?: string;
		canViewData?: boolean;
	} | null>(null);
	let showCreateDatabaseModal = $state(false);
	let newDatabaseName = $state('');
	let newDatabaseEncoding = $state('UTF8');
	let creatingDatabase = $state(false);
	let showDatabaseMenu = $state(false);
	let showSchemaMenu = $state(false);
	let explorerPane = $state<'tables' | 'saved' | 'history'>('tables');
	let selectedSchemaName = $state('');
	let showSearch = $state(false);
	let refreshingCount = $state(0);
	let refreshingDatabases = $state(false);
	let refreshingTables = $state(false);

	const postgresEncodings = ['UTF8', 'LATIN1', 'LATIN2', 'WIN1252'];

	function openContextMenu(
		event: MouseEvent,
		schema: string,
		table: string,
		kind: string,
	) {
		event.preventDefault();
		const menuWidth = 210;
		const menuHeight = 290;
		const margin = 8;
		const maxX = window.innerWidth - menuWidth - margin;
		const maxY = window.innerHeight - menuHeight - margin;
		const x = Math.max(margin, Math.min(event.clientX, maxX));
		const y = Math.max(margin, Math.min(event.clientY, maxY));
		contextMenu = { x, y, schema, table, kind };
		objectContextMenu = null;
	}

	function runMenuAction(action: TableAction) {
		if (!contextMenu) return;
		onTableAction(action, contextMenu.schema, contextMenu.table);
		contextMenu = null;
	}

	function openSchemaContextMenu(event: MouseEvent, schema: string) {
		event.preventDefault();
		const menuWidth = 210;
		const menuHeight = 140;
		const margin = 8;
		const maxX = window.innerWidth - menuWidth - margin;
		const maxY = window.innerHeight - menuHeight - margin;
		const x = Math.max(margin, Math.min(event.clientX, maxX));
		const y = Math.max(margin, Math.min(event.clientY, maxY));
		schemaContextMenu = { x, y, schema };
		contextMenu = null;
		objectContextMenu = null;
	}

	function runSchemaMenuAction(action: SchemaAction) {
		if (!schemaContextMenu) return;
		onSchemaAction(action, schemaContextMenu.schema);
		schemaContextMenu = null;
	}

	function handleSearchInput(event: Event) {
		onSearchChange((event.currentTarget as HTMLInputElement).value);
	}

	function toggleDatabaseMenu() {
		showDatabaseMenu = !showDatabaseMenu;
		showSchemaMenu = false;
	}

	function handleDatabaseSelect(database: string) {
		onChangeDatabase(database);
		showDatabaseMenu = false;
	}

	function openCreateDatabaseModal() {
		showCreateDatabaseModal = true;
		newDatabaseName = '';
		newDatabaseEncoding = 'UTF8';
	}

	function closeCreateDatabaseModal() {
		if (creatingDatabase) return;
		showCreateDatabaseModal = false;
	}

	async function submitCreateDatabase() {
		const name = newDatabaseName.trim();
		if (!name) return;
		creatingDatabase = true;
		try {
			await onCreateDatabase({ name, encoding: newDatabaseEncoding });
			showCreateDatabaseModal = false;
		} finally {
			creatingDatabase = false;
		}
	}

	let openGroups = $state(new Set<string>());

	function groupKey(schema: string, group: string) {
		return `${schema}::${group}`;
	}

	function isGroupOpen(schema: string, group: string) {
		if (searchQuery.trim().length > 0) return true;
		return openGroups.has(groupKey(schema, group));
	}

	function toggleGroup(schema: string, group: string) {
		const key = groupKey(schema, group);
		const next = new Set(openGroups);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		openGroups = next;
	}

	function openObjectMenu(
		event: MouseEvent,
		item: NonNullable<typeof objectContextMenu>,
	) {
		event.preventDefault();
		event.stopPropagation();
		const menuWidth = 210;
		const menuHeight = 140;
		const margin = 8;
		const maxX = window.innerWidth - menuWidth - margin;
		const maxY = window.innerHeight - menuHeight - margin;
		objectContextMenu = {
			...item,
			x: Math.max(margin, Math.min(event.clientX, maxX)),
			y: Math.max(margin, Math.min(event.clientY, maxY)),
		};
		contextMenu = null;
		schemaContextMenu = null;
	}

	function openRoutine(routine: DatabaseRoutine) {
		void onOpenObjectDefinition({
			kind: routine.kind,
			schema: routine.schema,
			name: routine.name,
			objectId: routine.objectId,
			identityArgs: routine.identityArgs,
		});
	}

	function openSequence(sequence: DatabaseSequence) {
		void onViewSequence(sequence.schema, sequence.name);
	}

	let filteredExplorer = $derived.by(() => {
		if (!explorer) return null;
		return filterExplorer(explorer, searchQuery);
	});

	let currentSchema = $derived.by(() => {
		const schemas = filteredExplorer?.schemas ?? [];
		if (schemas.length === 0) return null;
		return (
			schemas.find((schema) => schema.name === selectedSchemaName) ??
			schemas[0] ??
			null
		);
	});

	let listedRelations = $derived.by(() => {
		if (!currentSchema) return [];
		const tables = [
			...schemaTables(currentSchema),
			...schemaViews(currentSchema),
		];
		tables.sort((a, b) => a.name.localeCompare(b.name));
		return tables;
	});

	let listedTables = $derived(
		listedRelations.filter((item) => item.kind !== 'view'),
	);
	let listedViews = $derived(
		listedRelations.filter((item) => item.kind === 'view'),
	);
	let filteredSavedQueries = $derived.by(() => {
		const q = searchQuery.trim().toLowerCase();
		if (!q) return savedQueries;
		return savedQueries.filter((item) => item.title.toLowerCase().includes(q));
	});
	let filteredHistoryItems = $derived.by(() => {
		const q = searchQuery.trim().toLowerCase();
		const items = q
			? historyItems.filter((item) => item.sql.toLowerCase().includes(q))
			: historyItems;
		return items.slice(0, 12);
	});

	$effect(() => {
		const schemas = filteredExplorer?.schemas ?? [];
		if (schemas.length === 0) return;
		if (!schemas.some((schema) => schema.name === selectedSchemaName)) {
			selectedSchemaName = schemas[0].name;
		}
	});

	async function refreshEntities() {
		refreshingCount += 1;
		refreshingTables = true;
		refreshingDatabases = true;
		try {
			await Promise.all([onRefreshTables(), onRefreshDatabases()]);
		} finally {
			refreshingCount = Math.max(0, refreshingCount - 1);
			refreshingTables = false;
			refreshingDatabases = false;
		}
	}

	let selectedDatabaseLabel = $derived.by(() => {
		if (connectionStatus.database) return connectionStatus.database;
		return databases[0] ?? 'Select database';
	});

	function openTable(schema: string, table: string) {
		onTableAction('view_data', schema, table);
	}

	function handleHeaderPlus() {
		if (explorerPane !== 'tables') {
			onAddTab?.();
			return;
		}
		if (
			connectionStatus.connected &&
			dialectCapabilities(connectionStatus.databaseType).canCreateDatabase
		) {
			openCreateDatabaseModal();
		}
	}

	function openSaved(sql: string) {
		onOpenSavedQuery?.(sql);
	}

	let paneTitle = $derived(
		explorerPane === 'saved'
			? 'Saved'
			: explorerPane === 'history'
				? 'History'
				: 'Tables',
	);

	let diagramRail = $state(false);

	function selectExplorerPane(pane: 'tables' | 'saved' | 'history') {
		explorerPane = pane;
		diagramRail = false;
	}

	function selectDiagramRail() {
		diagramRail = true;
		onOpenDiagram?.();
	}
</script>

<aside class="flex h-full shrink-0">
	<nav
		class="w-11 bg-qc-panel border-r border-qc-border flex flex-col items-center py-2 shrink-0"
	>
		<div class="flex flex-col items-center gap-0.5">
			<button
				type="button"
				title="Tables"
				aria-label="Tables"
				onclick={() => selectExplorerPane('tables')}
				class={`rail-btn ${!diagramRail && explorerPane === 'tables' ? 'active' : ''}`}
			>
				<Database size={18} strokeWidth={1.75} />
			</button>
			<button
				type="button"
				title="Saved queries"
				aria-label="Saved queries"
				onclick={() => selectExplorerPane('saved')}
				class={`rail-btn ${!diagramRail && explorerPane === 'saved' ? 'active' : ''}`}
			>
				<Star size={18} strokeWidth={1.75} />
			</button>
			<button
				type="button"
				title="History"
				aria-label="History"
				onclick={() => selectExplorerPane('history')}
				class={`rail-btn ${!diagramRail && explorerPane === 'history' ? 'active' : ''}`}
			>
				<History size={18} strokeWidth={1.75} />
			</button>
			<button
				type="button"
				title="Schema diagram"
				aria-label="Schema diagram"
				onclick={selectDiagramRail}
				class={`rail-btn ${diagramRail ? 'active' : ''}`}
			>
				<Network size={18} strokeWidth={1.75} />
			</button>
		</div>
	</nav>

	<div
		class="w-[220px] bg-qc-panel border-r border-qc-border flex flex-col min-h-0"
	>
		<div
			class="flex items-center h-9 border-b border-qc-border px-1.5 gap-0.5 shrink-0 relative"
		>
			{#if explorerPane === 'tables'}
				<div class="flex-1 min-w-0 flex items-center gap-2.5 px-1 text-[12px]">
					<button
						type="button"
						class="inline-flex items-center gap-1 min-w-0 text-qc-muted hover:text-qc-subtle"
						onclick={toggleDatabaseMenu}
						oncontextmenu={(event) =>
							currentSchema && openSchemaContextMenu(event, currentSchema.name)}
						title={`Switch database (${selectedDatabaseLabel})`}
						aria-haspopup="listbox"
						aria-expanded={showDatabaseMenu}
					>
						<Database size={12} class="shrink-0 opacity-80" />
						<span class="truncate">{selectedDatabaseLabel}</span>
						<ChevronDown size={11} class="shrink-0 opacity-70" />
					</button>

					<button
						type="button"
						class="inline-flex items-center gap-1 min-w-0 text-qc-fg font-medium"
						onclick={() => {
							showSchemaMenu = !showSchemaMenu;
							showDatabaseMenu = false;
						}}
						oncontextmenu={(event) =>
							currentSchema && openSchemaContextMenu(event, currentSchema.name)}
						title={`Switch schema (${currentSchema?.name ?? 'schema'})`}
						aria-haspopup="listbox"
						aria-expanded={showSchemaMenu}
					>
						<Layers size={12} class="shrink-0 opacity-80" />
						<span class="truncate">{currentSchema?.name ?? 'schema'}</span>
						<ChevronDown size={11} class="shrink-0 opacity-70" />
					</button>
				</div>
			{:else}
				<div class="flex-1 text-[13px] font-medium text-qc-fg px-1 truncate">
					{paneTitle}
				</div>
			{/if}
			{#if explorerPane === 'tables'}
				<button
					type="button"
					class="w-7 h-7 rounded flex items-center justify-center text-qc-muted hover:bg-qc-hover"
					onclick={() => void refreshEntities()}
					aria-label="Refresh schema"
					title="Refresh"
				>
					<RefreshCw
						size={14}
						strokeWidth={2}
						class={refreshingTables || refreshingDatabases
							? 'animate-spin'
							: ''}
					/>
				</button>
			{/if}
			{#if explorerPane !== 'tables'}
				<button
					type="button"
					class="w-7 h-7 rounded flex items-center justify-center text-qc-muted hover:bg-qc-hover"
					onclick={handleHeaderPlus}
					aria-label="New query"
					title="New query"
				>
					<Plus size={15} strokeWidth={2} />
				</button>
			{/if}
			<button
				type="button"
				class="w-7 h-7 rounded flex items-center justify-center text-qc-muted hover:bg-qc-hover"
				onclick={() => (showSearch = !showSearch)}
				aria-label="Search schema"
				title="Search"
			>
				<Search size={14} strokeWidth={2} />
			</button>
			{#if showDatabaseMenu}
				<button
					type="button"
					class="fixed inset-0 z-20 cursor-default"
					aria-label="Close database selector"
					onclick={() => (showDatabaseMenu = false)}
				></button>
				<div
					class="absolute left-1.5 right-1.5 top-[calc(100%+4px)] z-30 overflow-hidden rounded-md border border-qc-border bg-qc-panel shadow-[0_8px_24px_rgba(0,0,0,0.24)] py-1"
				>
					{#each databases as db}
						<button
							type="button"
							class={`w-full px-2 py-1.5 text-left text-xs flex items-center justify-between gap-2 ${db === connectionStatus.database ? 'bg-qc-hover text-qc-fg' : 'text-qc-fg hover:bg-qc-hover'}`}
							onclick={() => handleDatabaseSelect(db)}
						>
							<span class="truncate">{db}</span>
							{#if db === connectionStatus.database}
								<Check size={12} class="shrink-0 text-qc-subtle" />
							{/if}
						</button>
					{/each}
					{#if connectionStatus.connected && dialectCapabilities(connectionStatus.databaseType).canCreateDatabase}
						<div class="my-1 border-t border-qc-border-subtle"></div>
						<button
							type="button"
							class="w-full px-2 py-1.5 text-left text-xs text-qc-subtle hover:bg-qc-hover"
							onclick={() => {
								showDatabaseMenu = false;
								openCreateDatabaseModal();
							}}
						>
							New database…
						</button>
					{/if}
				</div>
			{/if}
			{#if showSchemaMenu}
				<button
					type="button"
					class="fixed inset-0 z-20 cursor-default"
					aria-label="Close schema selector"
					onclick={() => (showSchemaMenu = false)}
				></button>
				<div
					class="absolute left-1.5 right-1.5 top-[calc(100%+4px)] z-30 overflow-hidden rounded-md border border-qc-border bg-qc-panel shadow-[0_8px_24px_rgba(0,0,0,0.24)] py-1"
				>
					{#each filteredExplorer?.schemas ?? [] as schema}
						<button
							type="button"
							class={`w-full px-2 py-1.5 text-left text-xs flex items-center justify-between gap-2 ${schema.name === currentSchema?.name ? 'bg-qc-hover text-qc-fg' : 'text-qc-fg hover:bg-qc-hover'}`}
							onclick={() => {
								selectedSchemaName = schema.name;
								showSchemaMenu = false;
							}}
						>
							<span class="truncate">{schema.name}</span>
							{#if schema.name === currentSchema?.name}
								<Check size={12} class="shrink-0 text-qc-subtle" />
							{/if}
						</button>
					{/each}
				</div>
			{/if}
			{#if refreshingCount > 0}
				<div
					class="absolute inset-x-0 bottom-0 h-px overflow-hidden pointer-events-none"
				>
					<div class="refresh-activity-bar"></div>
				</div>
			{/if}
		</div>

		{#if showSearch}
			<div class="relative px-2 py-1.5 border-b border-qc-border-subtle">
				<Search
					size={12}
					class="absolute left-4 top-1/2 -translate-y-1/2 text-qc-muted"
				/>
				<input
					type="text"
					value={searchQuery}
					oninput={handleSearchInput}
					placeholder="Filter…"
					class="w-full h-7 bg-qc-bg border border-qc-border text-qc-fg text-[12px] rounded-md pl-7 pr-2 outline-none focus:border-qc-muted"
				/>
			</div>
		{/if}

		<div class="flex-1 overflow-y-auto py-1.5 text-sm min-h-0">
			{#key explorerPane}
				{#if explorerPane === 'saved'}
					{#if filteredSavedQueries.length === 0}
						<div class="px-3 py-2 text-[11px] text-qc-muted">
							{savedQueries.length === 0 ? 'No saved queries.' : 'No matches.'}
						</div>
					{:else}
						{#each filteredSavedQueries as item (item.id)}
							<button
								type="button"
								class="sidebar-item w-full flex items-center gap-2 text-[12px] text-left"
								onclick={() => openSaved(item.sql)}
								title={item.title}
							>
								<Star
									size={14}
									strokeWidth={2}
									class="text-qc-muted shrink-0"
								/>
								<span class="truncate">{item.title}</span>
							</button>
						{/each}
					{/if}
				{:else if explorerPane === 'history'}
					{#if filteredHistoryItems.length === 0}
						<div class="px-3 py-2 text-[11px] text-qc-muted">
							{historyItems.length === 0 ? 'No history yet.' : 'No matches.'}
						</div>
					{:else}
						{#each filteredHistoryItems as item, index (`${index}-${item.time}-${item.sql}`)}
							<button
								type="button"
								class="sidebar-item w-full flex items-center gap-2 text-[12px] text-left"
								onclick={() => {
									const index = historyItems.indexOf(item);
									onOpenHistory?.(index);
									onOpenSavedQuery?.(item.sql);
								}}
								title={item.sql}
							>
								<History
									size={14}
									strokeWidth={2}
									class="text-qc-muted shrink-0"
								/>
								<span class="truncate font-mono text-[11px]">{item.sql}</span>
							</button>
						{/each}
					{/if}
				{:else if !connectionStatus.connected}
					<div class="px-3 py-3 text-xs text-qc-muted">
						Connect to load schema.
					</div>
				{:else if loadingExplorer && !filteredExplorer}
					<div class="px-3 py-3 space-y-2 animate-pulse">
						{#each skeletonRows as row}
							<div
								class={`h-3 rounded bg-qc-hover ${row % 3 === 0 ? 'w-36' : row % 3 === 1 ? 'w-28' : 'w-32'}`}
							></div>
						{/each}
					</div>
				{:else if currentSchema}
					{@const schema = currentSchema}
					{#snippet relationRow(table: { name: string; kind: string })}
						{@const active =
							activeTable?.schema === schema.name &&
							activeTable?.table === table.name}
						<button
							type="button"
							onclick={() => openTable(schema.name, table.name)}
							oncontextmenu={(event) =>
								openContextMenu(event, schema.name, table.name, table.kind)}
							class={`sidebar-item w-full flex items-center gap-1.5 text-[12px] text-left ${active ? 'active' : ''}`}
						>
							{#if table.kind === 'view'}
								<Eye size={14} strokeWidth={1.5} class="shrink-0" />
							{:else}
								<Table2 size={14} strokeWidth={1.5} class="shrink-0" />
							{/if}
							<span class="truncate">{table.name}</span>
						</button>
					{/snippet}
					{#if listedViews.length > 0}
						{#if listedTables.length > 0}
							<div
								class="px-3 pt-1 pb-0.5 text-[10px] uppercase tracking-wide text-qc-muted"
							>
								Tables
							</div>
							{#each listedTables as table (table.name)}
								{@render relationRow(table)}
							{/each}
						{/if}
						<div
							class="px-3 pt-2 pb-0.5 text-[10px] uppercase tracking-wide text-qc-muted"
						>
							Views
						</div>
						{#each listedViews as table (table.name)}
							{@render relationRow(table)}
						{/each}
					{:else}
						{#each listedRelations as table (table.name)}
							{@render relationRow(table)}
						{/each}
					{/if}

					<ExplorerGroup
						title="Functions"
						count={schemaFunctions(schema).length}
						open={isGroupOpen(schema.name, 'functions')}
						onToggle={() => toggleGroup(schema.name, 'functions')}
					>
						{#each schemaFunctions(schema) as routine}
							<button
								type="button"
								onclick={() => openRoutine(routine)}
								oncontextmenu={(event) =>
									openObjectMenu(event, {
										x: 0,
										y: 0,
										kind: 'function',
										schema: routine.schema,
										name: routine.name,
										objectId: routine.objectId,
										identityArgs: routine.identityArgs,
									})}
								class="sidebar-item w-full flex items-center gap-1.5 text-[12px] text-left"
								title={routine.returnType
									? `${routineSignature(routine)} → ${routine.returnType}`
									: routineSignature(routine)}
							>
								<Braces size={14} strokeWidth={1.5} class="shrink-0" />
								<span class="truncate">{routineSignature(routine)}</span>
							</button>
						{/each}
					</ExplorerGroup>

					<ExplorerGroup
						title="Procedures"
						count={schemaProcedures(schema).length}
						open={isGroupOpen(schema.name, 'procedures')}
						onToggle={() => toggleGroup(schema.name, 'procedures')}
					>
						{#each schemaProcedures(schema) as routine}
							<button
								type="button"
								onclick={() => openRoutine(routine)}
								oncontextmenu={(event) =>
									openObjectMenu(event, {
										x: 0,
										y: 0,
										kind: 'procedure',
										schema: routine.schema,
										name: routine.name,
										objectId: routine.objectId,
										identityArgs: routine.identityArgs,
									})}
								class="sidebar-item w-full flex items-center gap-1.5 text-[12px] text-left"
								title={routineSignature(routine)}
							>
								<Play size={14} strokeWidth={1.5} class="shrink-0" />
								<span class="truncate">{routineSignature(routine)}</span>
							</button>
						{/each}
					</ExplorerGroup>

					<ExplorerGroup
						title="Sequences"
						count={schemaSequences(schema).length}
						open={isGroupOpen(schema.name, 'sequences')}
						onToggle={() => toggleGroup(schema.name, 'sequences')}
					>
						{#each schemaSequences(schema) as sequence}
							<button
								type="button"
								onclick={() => openSequence(sequence)}
								oncontextmenu={(event) =>
									openObjectMenu(event, {
										x: 0,
										y: 0,
										kind: 'sequence',
										schema: sequence.schema,
										name: sequence.name,
										canViewData: true,
									})}
								class="sidebar-item w-full flex items-center gap-1.5 text-[12px] text-left"
								title={sequence.dataType
									? `${sequence.name} (${sequence.dataType})`
									: sequence.name}
							>
								<Hash size={14} strokeWidth={1.5} class="shrink-0" />
								<span class="truncate">{sequence.name}</span>
							</button>
						{/each}
					</ExplorerGroup>
				{/if}
			{/key}
		</div>

		{#if contextMenu}
			<button
				class="fixed inset-0 z-70"
				aria-label="Close table action menu"
				onclick={() => (contextMenu = null)}
			></button>
			<div
				class="fixed z-[75] min-w-[210px] bg-qc-panel rounded-lg shadow-[0_8px_30px_rgb(0,0,0,0.28)] border border-qc-border py-1.5 text-sm"
				style={`left:${contextMenu.x}px;top:${contextMenu.y}px;`}
			>
				<button
					onclick={() => runMenuAction('view_data')}
					class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
					>View Data</button
				>
				<button
					onclick={() => runMenuAction('view_structure')}
					class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
					>View Structure</button
				>
				{#if contextMenu.kind === 'view'}
					<button
						onclick={() => {
							const menu = contextMenu;
							if (!menu) return;
							void onOpenObjectDefinition({
								kind: 'view',
								schema: menu.schema,
								name: menu.table,
							});
							contextMenu = null;
						}}
						class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
						>View Definition</button
					>
				{:else}
					<button
						onclick={() => runMenuAction('sql_create')}
						class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
						>SQL: Create</button
					>
				{/if}
				<div class="h-px bg-qc-border my-1 mx-2"></div>
				<button
					onclick={() => runMenuAction('rename')}
					class="w-full flex items-center gap-2 text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
					><SquarePen size={14} />Rename</button
				>
				{#if contextMenu.kind !== 'view'}
					<button
						onclick={() => runMenuAction('duplicate')}
						class="w-full flex items-center gap-2 text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
						><Copy size={14} />Duplicate</button
					>
				{/if}
				<button
					onclick={() => runMenuAction('copy_name')}
					class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
					>Copy Name</button
				>
				<button
					onclick={() => runMenuAction('drop')}
					class="w-full flex items-center gap-2 text-left px-3 py-1.5 text-qc-danger hover:bg-qc-danger/10"
					><Trash2 size={14} />{contextMenu.kind === 'view'
						? 'Drop View'
						: 'Delete Cascade'}</button
				>
			</div>
		{/if}

		{#if objectContextMenu}
			<button
				class="fixed inset-0 z-70"
				aria-label="Close object action menu"
				onclick={() => (objectContextMenu = null)}
			></button>
			<div
				class="fixed z-[75] min-w-[210px] bg-qc-panel rounded-lg shadow-[0_8px_30px_rgb(0,0,0,0.28)] border border-qc-border py-1.5 text-sm"
				style={`left:${objectContextMenu.x}px;top:${objectContextMenu.y}px;`}
			>
				{#if objectContextMenu.canViewData}
					<button
						onclick={() => {
							if (objectContextMenu)
								void onViewSequence(
									objectContextMenu.schema,
									objectContextMenu.name,
								);
							objectContextMenu = null;
						}}
						class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
						>View Data</button
					>
				{/if}
				<button
					onclick={() => {
						if (!objectContextMenu) return;
						void onOpenObjectDefinition({
							kind: objectContextMenu.kind,
							schema: objectContextMenu.schema,
							name: objectContextMenu.name,
							objectId: objectContextMenu.objectId,
							identityArgs: objectContextMenu.identityArgs,
							table: objectContextMenu.table,
						});
						objectContextMenu = null;
					}}
					class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
					>View Definition</button
				>
				<button
					onclick={async () => {
						if (!objectContextMenu) return;
						await navigator.clipboard.writeText(
							objectContextMenu.identityArgs
								? `${objectContextMenu.schema}.${objectContextMenu.name}(${objectContextMenu.identityArgs})`
								: `${objectContextMenu.schema}.${objectContextMenu.name}`,
						);
						objectContextMenu = null;
					}}
					class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
					>Copy Name</button
				>
			</div>
		{/if}

		{#if schemaContextMenu}
			<button
				class="fixed inset-0 z-70"
				aria-label="Close schema action menu"
				onclick={() => (schemaContextMenu = null)}
			></button>
			<div
				class="fixed z-[75] min-w-[210px] bg-qc-panel rounded-lg shadow-[0_8px_30px_rgb(0,0,0,0.28)] border border-qc-border py-1.5 text-sm"
				style={`left:${schemaContextMenu.x}px;top:${schemaContextMenu.y}px;`}
			>
				<button
					onclick={() => runSchemaMenuAction('copy_name')}
					class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
					>Copy Name</button
				>
				<button
					onclick={() => runSchemaMenuAction('copy_quoted_name')}
					class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
					>Copy Quoted Name</button
				>
				<div class="h-px bg-qc-border my-1 mx-2"></div>
				<button
					onclick={() => runSchemaMenuAction('sql_list_tables')}
					class="w-full text-left px-3 py-1.5 text-qc-fg hover:bg-qc-hover"
					>SQL: List Tables</button
				>
			</div>
		{/if}

		{#if showCreateDatabaseModal}
			<div
				class="fixed inset-0 z-70 bg-black/55 backdrop-blur-[1px] flex items-center justify-center p-4"
			>
				<div
					class="w-full max-w-md overflow-hidden rounded-xl border border-qc-border bg-qc-panel"
				>
					<div
						class="h-10 px-4 border-b border-qc-border flex items-center justify-between bg-qc-elevated"
					>
						<h3 class="text-sm font-semibold text-qc-fg">Add Database</h3>
						<button
							onclick={closeCreateDatabaseModal}
							class="text-qc-muted hover:text-qc-fg"
							aria-label="Close create database modal">x</button
						>
					</div>
					<div class="p-4 space-y-3">
						<input
							bind:value={newDatabaseName}
							placeholder="Database Name"
							class="ui-input w-full h-9 text-sm px-2"
						/>
						<select
							bind:value={newDatabaseEncoding}
							class="w-full h-9 px-2 rounded-md border border-qc-border bg-qc-panel text-sm text-qc-fg outline-none"
						>
							{#each postgresEncodings as encoding}
								<option value={encoding}>{encoding}</option>
							{/each}
						</select>
					</div>
					<div
						class="h-12 px-4 border-t border-qc-border flex items-center justify-end gap-2 bg-qc-elevated"
					>
						<button
							onclick={closeCreateDatabaseModal}
							disabled={creatingDatabase}
							class="btn-secondary h-8 px-3 text-xs font-medium disabled:opacity-60"
						>
							Cancel
						</button>
						<button
							onclick={submitCreateDatabase}
							disabled={creatingDatabase || newDatabaseName.trim().length === 0}
							class="btn-primary h-8 px-3 text-xs disabled:opacity-60"
						>
							{creatingDatabase ? 'Creating...' : 'Add'}
						</button>
					</div>
				</div>
			</div>
		{/if}
	</div>
</aside>

<style>
	@keyframes refresh-activity-slide {
		0% {
			transform: translateX(-110%);
			width: 26%;
		}

		50% {
			transform: translateX(55%);
			width: 46%;
		}

		100% {
			transform: translateX(230%);
			width: 26%;
		}
	}

	.refresh-activity-bar {
		height: 100%;
		background: var(--qc-cell);
		animation: refresh-activity-slide 900ms ease-in-out infinite;
	}
</style>
