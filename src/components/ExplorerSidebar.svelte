<script lang="ts">
	import {
		ChevronDown,
		ChevronRight,
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
	import type { ConnectionStatus, DatabaseExplorer } from "../lib/rpc";

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
		onRefresh,
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
		onRefresh: () => void | Promise<void>;
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

	function handleDatabaseChange(event: Event) {
		onChangeDatabase((event.currentTarget as HTMLSelectElement).value);
	}

	function handleSearchInput(event: Event) {
		onSearchChange((event.currentTarget as HTMLInputElement).value);
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
</script>

<aside class="w-[300px] bg-white border-r border-gray-200 flex flex-col shrink-0 shadow-[2px_0_10px_rgba(0,0,0,0.02)]">
	<div class="p-3 border-b border-gray-100 space-y-3">
		<div class="flex items-center gap-2">
			<select
				value={connectionStatus.database}
				onchange={handleDatabaseChange}
				class="flex-1 h-8 px-2 rounded-md border border-gray-200 bg-white text-xs text-gray-700 outline-none"
			>
				{#each databases as db}
					<option value={db}>{db}</option>
				{/each}
			</select>
			<button
				onclick={() => onRefresh()}
				class="w-8 h-8 rounded-md border border-gray-200 text-gray-500 hover:text-gray-800 hover:bg-gray-50 flex items-center justify-center"
				aria-label="Refresh explorer"
			>
				<RefreshCw size={14} />
			</button>
			{#if connectionStatus.connected && connectionStatus.databaseType === "postgres"}
				<button
					onclick={openCreateDatabaseModal}
					class="w-8 h-8 rounded-md border border-gray-200 text-gray-500 hover:text-gray-800 hover:bg-gray-50 flex items-center justify-center"
					aria-label="Create database"
					title="Create database"
				>
					<Plus size={14} />
				</button>
			{/if}
		</div>

		<div class="relative flex items-center w-full">
			<Search size={14} class="w-4 h-4 absolute left-2.5 text-gray-400" />
			<input
				type="text"
				value={searchQuery}
				oninput={handleSearchInput}
				placeholder="Search..."
				class="w-full bg-gray-50 border border-gray-200 text-gray-900 text-sm rounded-md focus:ring-emerald-500 focus:border-emerald-500 block pl-8 pr-2 py-1.5 shadow-sm placeholder-gray-400"
			/>
			<div class="absolute right-2 text-xs text-gray-400 font-medium bg-gray-100 border border-gray-200 px-1 rounded">Ctrl+K</div>
		</div>
	</div>

	<div class="flex-1 overflow-y-auto p-2 pb-6 text-sm">
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
			<div class="px-2 py-3 text-xs text-gray-500">No schemas or tables found.</div>
		{:else}
			<div class="flex flex-col">
				<div class="flex items-center w-full px-2 py-1.5 rounded-md text-gray-700">
					
					<Table2 size={14} class="mr-2 text-amber-500" />
					<span class="font-medium">Tables</span>
				</div>

				<div class="flex flex-col ml-5 relative before:absolute before:left-[-11px] before:top-0 before:bottom-0 before:w-px before:bg-gray-200">
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
							<div class="flex flex-col ml-5 relative before:absolute before:left-[-11px] before:top-0 before:bottom-0 before:w-px before:bg-gray-200">
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
			<button onclick={() => runMenuAction("drop")} class="w-full flex items-center gap-2 text-left px-3 py-1.5 text-red-600 hover:bg-red-50"><Trash2 size={14} />Delete</button>
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
