<script lang="ts">
	import { onMount } from "svelte";
	import {
		ChevronDown,
		ChevronRight,
		ChevronsUpDown,
		Columns2,
		Copy,
		Info,
		KeyRound,
		ListOrdered,
		Plus,
		RefreshCw,
		Search,
		SquarePen,
		Table2,
		Trash2,
	} from "@lucide/svelte";
	import type { ConnectionStatus, DatabaseExplorer } from "$lib/rpc";

	type TableAction =
		| "view_data"
		| "view_structure"
		| "export_file"
		| "import_file"
		| "copy_name"
		| "hide"
		| "sql_create"
		| "rename"
		| "drop"
		| "truncate"
		| "duplicate";
	type SchemaAction =
		| "copy_name"
		| "copy_quoted_name"
		| "sql_list_tables";

	let {
		connectionStatus,
		explorer,
		loadingExplorer,
		databases,
		searchQuery,
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
	}: {
		connectionStatus: ConnectionStatus;
		explorer: DatabaseExplorer | null;
		loadingExplorer: boolean;
		databases: string[];
		searchQuery: string;
		expandedSchemas: Set<string>;
		expandedTables: Set<string>;
		onChangeDatabase: (database: string) => void;
		onSearchChange: (value: string) => void;
		onToggleSchema: (name: string) => void;
		onToggleTable: (schema: string, table: string) => void;
		onRefreshDatabases: () => void | Promise<void>;
		onRefreshTables: () => void | Promise<void>;
		onCreateDatabase: (params: { name: string; encoding: string }) => void | Promise<void>;
		onTableAction: (action: TableAction, schema: string, table: string) => void;
		onSchemaAction: (action: SchemaAction, schema: string) => void;
		onFollowForeignKey: (schema: string, table: string) => void;
	} = $props();

	const skeletonRows = [0, 1, 2, 3, 4, 5];

	let contextMenu = $state<{
		x: number;
		y: number;
		schema: string;
		table: string;
	} | null>(null);
	let schemaContextMenu = $state<{
		x: number;
		y: number;
		schema: string;
	} | null>(null);
	let showCreateDatabaseModal = $state(false);
	let newDatabaseName = $state("");
	let newDatabaseEncoding = $state("UTF8");
	let creatingDatabase = $state(false);
	let showDatabaseMenu = $state(false);
	let refreshingCount = $state(0);
	let refreshingDatabases = $state(false);
	let refreshingTables = $state(false);

	const postgresEncodings = ["UTF8", "LATIN1", "LATIN2", "WIN1252"];

	function openContextMenu(event: MouseEvent, schema: string, table: string) {
		event.preventDefault();
		const menuWidth = 210;
		const menuHeight = 290;
		const margin = 8;
		const maxX = window.innerWidth - menuWidth - margin;
		const maxY = window.innerHeight - menuHeight - margin;
		const x = Math.max(margin, Math.min(event.clientX, maxX));
		const y = Math.max(margin, Math.min(event.clientY, maxY));
		contextMenu = { x, y, schema, table };
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
	}

	function handleDatabaseSelect(database: string) {
		onChangeDatabase(database);
		showDatabaseMenu = false;
	}

	function openCreateDatabaseModal() {
		showCreateDatabaseModal = true;
		newDatabaseName = "";
		newDatabaseEncoding = "UTF8";
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

	let filteredExplorer = $derived.by(() => {
		if (!explorer) return null;
		const query = searchQuery.trim().toLowerCase();
		if (!query) return explorer;

		const filteredSchemas = explorer.schemas
			.map((schema) => ({
				...schema,
				tables: schema.tables.filter((table) => {
					if (table.name.toLowerCase().includes(query)) return true;
					if (table.columns.some((column) => column.name.toLowerCase().includes(query))) return true;
					return false;
				}),
			}))
			.filter((schema) => schema.tables.length > 0);

		return { ...explorer, schemas: filteredSchemas };
	});

	let totalEntities = $derived.by(() => {
		if (!explorer) return 0;
		return explorer.schemas.reduce((sum, schema) => sum + schema.tables.length, 0);
	});

	function expandAllEntities() {
		if (!explorer) return;

		for (const schema of explorer.schemas) {
			if (!expandedSchemas.has(schema.name)) onToggleSchema(schema.name);
			for (const table of schema.tables) {
				const key = `${schema.name}.${table.name}`;
				if (!expandedTables.has(key)) onToggleTable(schema.name, table.name);
			}
		}
	}

	function collapseAllEntities() {
		if (!explorer) return;

		for (const key of expandedTables) {
			const dot = key.indexOf(".");
			if (dot <= 0) continue;
			const schema = key.slice(0, dot);
			const table = key.slice(dot + 1);
			onToggleTable(schema, table);
		}

		for (const schemaName of expandedSchemas) {
			onToggleSchema(schemaName);
		}
	}

	async function refreshEntities() {
		refreshingCount += 1;
		refreshingTables = true;
		try {
			await onRefreshTables();
		} finally {
			refreshingCount = Math.max(0, refreshingCount - 1);
			refreshingTables = false;
		}
	}

	async function refreshDatabaseSelector() {
		refreshingCount += 1;
		refreshingDatabases = true;
		try {
			await onRefreshDatabases();
		} finally {
			refreshingCount = Math.max(0, refreshingCount - 1);
			refreshingDatabases = false;
		}
	}

	let allEntitiesExpanded = $derived.by(() => {
		if (!explorer) return false;
		for (const schema of explorer.schemas) {
			if (!expandedSchemas.has(schema.name)) return false;
			for (const table of schema.tables) {
				if (!expandedTables.has(`${schema.name}.${table.name}`)) return false;
			}
		}
		return true;
	});

	function toggleExpandCollapseEntities() {
		if (allEntitiesExpanded) {
			collapseAllEntities();
			return;
		}
		expandAllEntities();
	}

	let selectedDatabaseLabel = $derived.by(() => {
		if (connectionStatus.database) return connectionStatus.database;
		return databases[0] ?? "Select database";
	});

	onMount(() => {
		// Ctrl+K now handled globally by SearchPalette in +page.svelte
		return () => {};
	});
</script>

<aside class="w-[260px] bg-white border-r border-gray-200 flex flex-col shrink-0 shadow-[2px_0_10px_rgba(0,0,0,0.02)]">
	<div class="p-2.5 border-b border-gray-100 space-y-2.5">
		<div class="relative">
			<div class="h-8 bg-white border border-gray-200 rounded-md flex items-center overflow-hidden hover:border-gray-300 focus-within:border-gray-300 focus-within:ring-1 focus-within:ring-gray-200">
				<button
					type="button"
					class="relative flex-1 h-full min-w-0 pl-2 pr-6 flex items-center text-left text-xs text-gray-700 hover:bg-white focus:outline-none"
					aria-haspopup="listbox"
					aria-expanded={showDatabaseMenu}
					aria-label="Select database"
					onclick={toggleDatabaseMenu}
				>
					<span class="truncate">{selectedDatabaseLabel}</span>
					<ChevronDown size={13} class="absolute right-2 text-gray-400 shrink-0" />
				</button>
				<div class="w-px h-4 bg-gray-200"></div>
				<button
					type="button"
					onclick={refreshDatabaseSelector}
					class="w-8 h-full text-gray-500 hover:text-gray-800 flex items-center justify-center"
					aria-label="Refresh databases"
					title="Refresh databases"
				>
					<RefreshCw size={13} class={refreshingDatabases ? "animate-spin" : ""} />
				</button>
				{#if connectionStatus.connected && connectionStatus.databaseType === "postgres"}
					<button
						type="button"
						onclick={openCreateDatabaseModal}
						class="w-8 h-full rounded-r-md text-gray-500 hover:text-gray-800 flex items-center justify-center"
						aria-label="Create database"
						title="Create database"
					>
						<Plus size={13} />
					</button>
				{/if}
			</div>

			{#if showDatabaseMenu}
				<button
					type="button"
					class="fixed inset-0 z-20 cursor-default"
					aria-label="Close database selector"
					onclick={() => (showDatabaseMenu = false)}
				></button>
				<div class="absolute left-0 right-0 top-[calc(100%+4px)] z-30 overflow-hidden rounded-md border border-gray-200 bg-white shadow-[0_8px_24px_rgba(0,0,0,0.08)] py-1">
					{#each databases as db}
						<button
							type="button"
							class={`w-full px-2 py-1.5 text-left text-xs ${
								db === connectionStatus.database
									? "bg-emerald-50 text-emerald-700"
									: "text-gray-700 hover:bg-gray-50"
							}`}
							onclick={() => handleDatabaseSelect(db)}
						>
							{db}
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<div class="relative flex items-center w-full">
			<Search size={14} class="w-4 h-4 absolute left-2.5 text-gray-400" />
			<input
				type="text"
				value={searchQuery}
				oninput={handleSearchInput}
				placeholder="Search..."
				class="w-full h-8 bg-white border border-gray-200 text-gray-900 text-sm rounded-md block pl-8 pr-14 py-1.5 placeholder-gray-400 focus:outline-none hover:border-gray-300 focus:border-gray-300 focus:ring-1 focus:ring-gray-200"
			/>
			<div class="absolute right-1.5 text-xs text-gray-500 font-medium bg-gray-50 border border-gray-200 px-1.5 py-0.5 rounded">Ctrl+K</div>
		</div>

	</div>

	<div class="flex-1 overflow-y-auto  pb-2 text-sm">
	
		<div class="h-px bg-gray-200 overflow-hidden">
			{#if refreshingCount > 0}
				<div class="refresh-activity-bar"></div>
			{/if}
		</div>
		{#if !connectionStatus.connected}
			<div class="px-2 py-3 text-xs text-gray-500">Connect to load schema.</div>
		{:else if loadingExplorer && !filteredExplorer}
			<div class="px-2 py-3 space-y-2 animate-pulse">
				<div class="h-3 w-20 rounded bg-gray-200"></div>
				{#each skeletonRows as row}
					<div class="flex items-center gap-2">
						<div class="h-4 w-4 rounded bg-gray-200 shrink-0"></div>
						<div
							class={`h-3 rounded bg-gray-200 ${row % 3 === 0 ? "w-44" : row % 3 === 1 ? "w-36" : "w-40"}`}
						></div>
					</div>
				{/each}
			</div>
		{:else if !filteredExplorer || filteredExplorer.schemas.length === 0}
			<div class="px-2 py-3 text-xs text-gray-500">No entities found.</div>
		{:else}
			<div class="flex flex-col">
				<div class="flex items-center justify-between w-full px-2 py-1.5 rounded-md text-gray-700">
					<div class="flex items-center gap-1.5 min-w-0">
						<span class="text-xs font-medium tracking-wide text-gray-700">Entities</span>
						<span class="px-1.5 py-0.5 rounded-full bg-gray-100 border border-gray-200 text-[10px] leading-none font-medium text-gray-500">
							{totalEntities}
						</span>
					</div>
					<div class="flex items-center gap-0.5">
						<button
							type="button"
							class="w-6 h-6 rounded text-gray-500 hover:text-gray-800 hover:bg-gray-100 flex items-center justify-center"
							title={allEntitiesExpanded ? "Collapse all entities" : "Expand all entities"}
							aria-label={allEntitiesExpanded ? "Collapse all entities" : "Expand all entities"}
							onclick={toggleExpandCollapseEntities}
						>
							<ChevronsUpDown size={13} />
						</button>
						<button
							type="button"
							class="w-6 h-6 rounded text-gray-500 hover:text-gray-800 hover:bg-gray-100 flex items-center justify-center"
							title="Refresh tables"
							aria-label="Refresh tables"
							onclick={refreshEntities}
						>
							<RefreshCw size={13} class={refreshingTables ? "animate-spin" : ""} />
						</button>
					</div>
				</div>

				<div class="flex flex-col">
					{#each filteredExplorer.schemas as schema}
						<button
							onclick={() => onToggleSchema(schema.name)}
							oncontextmenu={(event) => openSchemaContextMenu(event, schema.name)}
							class="flex items-center w-full min-w-0 px-2 py-1.5 hover:bg-gray-50 rounded-md text-gray-700 group text-left"
						>
							{#if expandedSchemas.has(schema.name)}
								<ChevronDown size={14} class="mr-1 text-gray-400 group-hover:text-gray-600 shrink-0" />
							{:else}
								<ChevronRight size={14} class="mr-1 text-gray-400 group-hover:text-gray-600 shrink-0" />
							{/if}
							<Table2 size={14} class="mr-2 text-amber-500 shrink-0" />
							<span class="truncate min-w-0" title={schema.name}>{schema.name}</span>
						</button>

						{#if expandedSchemas.has(schema.name)}
							<div class="flex flex-col ml-5">
								{#each schema.tables as table}
									<button
										onclick={() => onToggleTable(schema.name, table.name)}
										oncontextmenu={(event) => openContextMenu(event, schema.name, table.name)}
										class="flex items-center w-full min-w-0 px-2 py-1 hover:bg-gray-50 rounded-md text-gray-700 group text-left"
									>
										{#if expandedTables.has(`${schema.name}.${table.name}`)}
											<ChevronDown size={14} class="mr-1 text-gray-400 group-hover:text-gray-600 shrink-0" />
										{:else}
											<ChevronRight size={14} class="mr-1 text-gray-400 group-hover:text-gray-600 shrink-0" />
										{/if}
										<div class="w-4 h-4 mr-2 shrink-0 flex items-center justify-center rounded bg-emerald-500 text-white"><Columns2 size={11} /></div>
										<span class="font-medium text-gray-800 truncate min-w-0" title={table.name}>{table.name}</span>
									</button>

									{#if expandedTables.has(`${schema.name}.${table.name}`)}
										<div class="flex flex-col ml-7 mt-0.5 space-y-0.5 relative before:absolute before:left-[-19px] before:top-0 before:bottom-0 before:w-px before:bg-gray-200">
											{#each table.columns as column}
												<div class="flex items-center w-full px-2 py-1 hover:bg-gray-50 rounded-md text-gray-600">
													{#if column.name.endsWith("_id")}
														<KeyRound size={13} class="mr-2 text-gray-400 shrink-0" />
													{:else}
														<div class="w-3.5 h-3.5 mr-2 shrink-0 border border-gray-300 rounded-sm bg-gray-50 flex items-center justify-center"><span class="text-[8px] font-medium text-gray-400">T</span></div>
													{/if}
													<span class="truncate min-w-0">{column.name}</span>
												</div>
											{/each}

											{#if table.foreignKeys.length > 0}
												<div class="mt-1">
													<div class="flex items-center w-full px-2 py-1 text-gray-700">
														<div class="w-4 h-4 mr-2 flex items-center justify-center rounded bg-blue-500 text-white"><ListOrdered size={11} /></div>
														<span class="font-medium text-gray-800">Relations</span>
													</div>
													{#each table.foreignKeys as fk}
														<button onclick={() => onFollowForeignKey(fk.referencedSchema, fk.referencedTable)} class="flex items-center w-full px-2 py-1 hover:bg-gray-50 rounded-md text-gray-600 text-left">
															<Info size={13} class="mr-2 text-gray-400" />
															<span>{fk.column}</span>
														</button>
													{/each}
												</div>
											{/if}
										</div>
									{/if}
								{/each}
							</div>
						{/if}
					{/each}
				</div>
			</div>
		{/if}
	</div>

	{#if contextMenu}
		<button class="fixed inset-0 z-70" aria-label="Close table action menu" onclick={() => (contextMenu = null)}></button>
		<div class="fixed z-[75] min-w-[210px] bg-white rounded-lg shadow-[0_8px_30px_rgb(0,0,0,0.12)] border border-gray-200 py-1.5 text-sm" style={`left:${contextMenu.x}px;top:${contextMenu.y}px;`}>
			<button onclick={() => runMenuAction("view_data")} class="w-full text-left px-3 py-1.5 text-gray-700 hover:bg-gray-100">View Data</button>
			<button onclick={() => runMenuAction("view_structure")} class="w-full text-left px-3 py-1.5 text-gray-700 hover:bg-gray-100">View Structure</button>
			<button onclick={() => runMenuAction("sql_create")} class="w-full text-left px-3 py-1.5 text-gray-700 hover:bg-gray-100">SQL: Create</button>
			<div class="h-px bg-gray-200 my-1 mx-2"></div>
			<button onclick={() => runMenuAction("rename")} class="w-full flex items-center gap-2 text-left px-3 py-1.5 text-gray-700 hover:bg-gray-100"><SquarePen size={14} />Rename</button>
			<button onclick={() => runMenuAction("duplicate")} class="w-full flex items-center gap-2 text-left px-3 py-1.5 text-gray-700 hover:bg-gray-100"><Copy size={14} />Duplicate</button>
			<button onclick={() => runMenuAction("copy_name")} class="w-full text-left px-3 py-1.5 text-gray-700 hover:bg-gray-100">Copy Name</button>
			<button onclick={() => runMenuAction("drop")} class="w-full flex items-center gap-2 text-left px-3 py-1.5 text-red-600 hover:bg-red-50"><Trash2 size={14} />Delete Cascade</button>
		</div>
	{/if}

	{#if schemaContextMenu}
		<button class="fixed inset-0 z-70" aria-label="Close schema action menu" onclick={() => (schemaContextMenu = null)}></button>
		<div class="fixed z-[75] min-w-[210px] bg-white rounded-lg shadow-[0_8px_30px_rgb(0,0,0,0.12)] border border-gray-200 py-1.5 text-sm" style={`left:${schemaContextMenu.x}px;top:${schemaContextMenu.y}px;`}>
			<button onclick={() => runSchemaMenuAction("copy_name")} class="w-full text-left px-3 py-1.5 text-gray-700 hover:bg-gray-100">Copy Name</button>
			<button onclick={() => runSchemaMenuAction("copy_quoted_name")} class="w-full text-left px-3 py-1.5 text-gray-700 hover:bg-gray-100">Copy Quoted Name</button>
			<div class="h-px bg-gray-200 my-1 mx-2"></div>
			<button onclick={() => runSchemaMenuAction("sql_list_tables")} class="w-full text-left px-3 py-1.5 text-gray-700 hover:bg-gray-100">SQL: List Tables</button>
		</div>
	{/if}

	{#if showCreateDatabaseModal}
		<div class="fixed inset-0 z-[70] bg-black/55 backdrop-blur-[1px] flex items-center justify-center p-4">
			<div class="w-full max-w-md overflow-hidden rounded-xl border border-gray-200 bg-white shadow-[0_24px_60px_rgba(16,37,70,0.26)]">
				<div class="h-10 px-4 border-b border-gray-200 flex items-center justify-between bg-gray-50">
					<h3 class="text-sm font-semibold text-gray-900">Add Database</h3>
					<button onclick={closeCreateDatabaseModal} class="text-gray-500 hover:text-gray-900" aria-label="Close create database modal">x</button>
				</div>
				<div class="p-4 space-y-3">
					<input
						bind:value={newDatabaseName}
						placeholder="Database Name"
						class="ui-input w-full h-9 text-sm px-2"
					/>
					<select
						bind:value={newDatabaseEncoding}
						class="w-full h-9 px-2 rounded-md border border-gray-200 bg-white text-sm text-gray-700 outline-none"
					>
						{#each postgresEncodings as encoding}
							<option value={encoding}>{encoding}</option>
						{/each}
					</select>
				</div>
				<div class="h-12 px-4 border-t border-gray-200 flex items-center justify-end gap-2 bg-gray-50">
					<button
						onclick={closeCreateDatabaseModal}
						disabled={creatingDatabase}
						class="btn-secondary h-8 px-3 rounded-md text-xs disabled:opacity-60"
					>
						Cancel
					</button>
					<button
						onclick={submitCreateDatabase}
						disabled={creatingDatabase || newDatabaseName.trim().length === 0}
						class="h-8 px-3 rounded-md border border-emerald-500 bg-emerald-500 text-white text-xs hover:bg-emerald-600 hover:border-emerald-600"
					>
						{creatingDatabase ? "Creating..." : "Add"}
					</button>
				</div>
			</div>
		</div>
	{/if}

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
		background: #10b981;
		border-top: 1px solid #10b981;
		border-bottom: 1px solid #10b981;
		animation: refresh-activity-slide 900ms ease-in-out infinite;
	}
</style>

