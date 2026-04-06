<script lang="ts">
	import { X, Plus, Trash2, Wrench } from "@lucide/svelte";
	import type { DatabaseExplorer } from "$lib/rpc";

	let {
		visible,
		explorer,
		onClose,
		onRunSql,
	}: {
		visible: boolean;
		explorer: DatabaseExplorer | null;
		onClose: () => void;
		onRunSql: (sql: string) => Promise<void>;
	} = $props();

	type Mode = "create_table" | "alter_table" | "create_index" | "create_relation" | "create_partition";
	let mode = $state<Mode>("create_table");
	let busy = $state(false);
	let message = $state("");
	let messageKind = $state<"ok" | "error">("ok");

	let selectedSchema = $state("public");
	let selectedTable = $state("");
	let tableName = $state("new_table");
	let columns = $state<Array<{ name: string; type: string; notNull: boolean }>>([
		{ name: "id", type: "bigserial primary key", notNull: true },
	]);

	let alterColumnName = $state("new_column");
	let alterColumnType = $state("text");

	let indexName = $state("idx_new");
	let indexColumn = $state("id");
	let uniqueIndex = $state(false);

	let fkName = $state("fk_new");
	let fkColumn = $state("id");
	let fkRefSchema = $state("public");
	let fkRefTable = $state("");
	let fkRefColumn = $state("id");

	let partitionColumn = $state("created_at");

	let availableSchemas = $derived(explorer?.schemas.map((s) => s.name) ?? []);
	let availableTables = $derived(explorer?.schemas.find((s) => s.name === selectedSchema)?.tables ?? []);

	$effect(() => {
		if (!visible) return;
		if (!selectedSchema && availableSchemas.length > 0) selectedSchema = availableSchemas[0];
		if (!selectedTable && availableTables.length > 0) selectedTable = availableTables[0].name;
		if (!fkRefTable && availableTables.length > 0) fkRefTable = availableTables[0].name;
	});

	function addColumn() {
		columns = [...columns, { name: `column_${columns.length + 1}`, type: "text", notNull: false }];
	}

	function removeColumn(index: number) {
		columns = columns.filter((_, i) => i !== index);
	}

	function updateColumn(index: number, patch: Partial<{ name: string; type: string; notNull: boolean }>) {
		columns = columns.map((column, i) => (i === index ? { ...column, ...patch } : column));
	}

	async function submit() {
		busy = true;
		message = "";
		try {
			let sql = "";
			if (mode === "create_table") {
				const cols = columns.map((c) => `"${c.name}" ${c.type}${c.notNull ? " NOT NULL" : ""}`).join(",\n  ");
				sql = `CREATE TABLE "${selectedSchema}"."${tableName}" (\n  ${cols}\n);`;
			}
			if (mode === "alter_table") {
				sql = `ALTER TABLE "${selectedSchema}"."${selectedTable}" ADD COLUMN "${alterColumnName}" ${alterColumnType};`;
			}
			if (mode === "create_index") {
				sql = `CREATE ${uniqueIndex ? "UNIQUE " : ""}INDEX "${indexName}" ON "${selectedSchema}"."${selectedTable}" ("${indexColumn}");`;
			}
			if (mode === "create_relation") {
				sql = `ALTER TABLE "${selectedSchema}"."${selectedTable}" ADD CONSTRAINT "${fkName}" FOREIGN KEY ("${fkColumn}") REFERENCES "${fkRefSchema}"."${fkRefTable}" ("${fkRefColumn}");`;
			}
			if (mode === "create_partition") {
				sql = `ALTER TABLE "${selectedSchema}"."${selectedTable}" PARTITION BY RANGE ("${partitionColumn}");`;
			}

			await onRunSql(sql);
			messageKind = "ok";
			message = "Executed successfully";
		} catch (error) {
			messageKind = "error";
			message = error instanceof Error ? error.message : String(error);
		} finally {
			busy = false;
		}
	}
</script>

{#if visible}
	<div class="absolute inset-0 bg-black/35 flex items-center justify-center z-50 p-4">
		<div class="w-full max-w-4xl rounded-2xl bg-white border border-gray-200 shadow-[0_24px_60px_rgba(16,37,70,0.26)] overflow-hidden">
			<div class="px-5 py-4 border-b border-gray-200 flex items-center justify-between bg-white">
				<h3 class="text-gray-900 text-sm font-semibold inline-flex items-center gap-2"><Wrench size={15} class="text-emerald-600" />Schema Studio</h3>
				<button onclick={onClose} class="text-gray-500 hover:text-gray-900 w-7 h-7 rounded-md hover:bg-gray-100 flex items-center justify-center" aria-label="Close studio"><X size={16} /></button>
			</div>

			<div class="p-5 space-y-4 text-xs">
				<div class="inline-flex rounded-md border border-gray-200 bg-gray-50 p-1 flex-wrap gap-1">
					<button onclick={() => (mode = "create_table")} class={`px-2 py-1 rounded border text-xs ${mode === "create_table" ? "bg-emerald-500 border-emerald-500 text-white" : "bg-white border-transparent text-gray-600 hover:text-gray-900"}`}>Create Table</button>
					<button onclick={() => (mode = "alter_table")} class={`px-2 py-1 rounded border text-xs ${mode === "alter_table" ? "bg-emerald-500 border-emerald-500 text-white" : "bg-white border-transparent text-gray-600 hover:text-gray-900"}`}>Alter Table</button>
					<button onclick={() => (mode = "create_index")} class={`px-2 py-1 rounded border text-xs ${mode === "create_index" ? "bg-emerald-500 border-emerald-500 text-white" : "bg-white border-transparent text-gray-600 hover:text-gray-900"}`}>Index</button>
					<button onclick={() => (mode = "create_relation")} class={`px-2 py-1 rounded border text-xs ${mode === "create_relation" ? "bg-emerald-500 border-emerald-500 text-white" : "bg-white border-transparent text-gray-600 hover:text-gray-900"}`}>Relation</button>
					<button onclick={() => (mode = "create_partition")} class={`px-2 py-1 rounded border text-xs ${mode === "create_partition" ? "bg-emerald-500 border-emerald-500 text-white" : "bg-white border-transparent text-gray-600 hover:text-gray-900"}`}>Partition</button>
				</div>

				<div class="grid grid-cols-2 gap-3">
					<label class="flex flex-col gap-1 text-gray-600">
						Schema
						<select value={selectedSchema} onchange={(e) => (selectedSchema = (e.currentTarget as HTMLSelectElement).value)} class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500">
							{#each availableSchemas as schema}
								<option value={schema}>{schema}</option>
							{/each}
						</select>
					</label>
					<label class="flex flex-col gap-1 text-gray-600">
						Table
						<select value={selectedTable} onchange={(e) => (selectedTable = (e.currentTarget as HTMLSelectElement).value)} class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500">
							{#each availableTables as t}
								<option value={t.name}>{t.name}</option>
							{/each}
						</select>
					</label>
				</div>

				{#if mode === "create_table"}
					<div class="space-y-2">
						<label class="flex flex-col gap-1 text-gray-600">
							Table Name
							<input value={tableName} oninput={(e) => (tableName = (e.currentTarget as HTMLInputElement).value)} class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
						</label>
						<div class="space-y-2">
							{#each columns as column, index}
								<div class="grid grid-cols-12 gap-2 items-center">
									<input value={column.name} oninput={(e) => updateColumn(index, { name: (e.currentTarget as HTMLInputElement).value })} class="col-span-4 h-8 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
									<input value={column.type} oninput={(e) => updateColumn(index, { type: (e.currentTarget as HTMLInputElement).value })} class="col-span-5 h-8 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
									<label class="col-span-2 flex items-center gap-1 text-gray-600"><input type="checkbox" checked={column.notNull} onchange={(e) => updateColumn(index, { notNull: (e.currentTarget as HTMLInputElement).checked })} /> NN</label>
									<button onclick={() => removeColumn(index)} class="col-span-1 h-8 rounded-md border border-red-200 bg-red-50 text-red-600 flex items-center justify-center hover:bg-red-100"><Trash2 size={12} /></button>
								</div>
							{/each}
						</div>
						<button onclick={addColumn} class="h-8 px-3 rounded-md border border-gray-200 bg-white text-gray-700 inline-flex items-center gap-1 hover:bg-gray-50"><Plus size={12} />Add Column</button>
					</div>
				{/if}

				{#if mode === "alter_table"}
					<div class="grid grid-cols-2 gap-2">
						<input value={alterColumnName} oninput={(e) => (alterColumnName = (e.currentTarget as HTMLInputElement).value)} placeholder="column_name" class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
						<input value={alterColumnType} oninput={(e) => (alterColumnType = (e.currentTarget as HTMLInputElement).value)} placeholder="column_type" class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
					</div>
				{/if}

				{#if mode === "create_index"}
					<div class="grid grid-cols-3 gap-2">
						<input value={indexName} oninput={(e) => (indexName = (e.currentTarget as HTMLInputElement).value)} placeholder="index name" class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
						<input value={indexColumn} oninput={(e) => (indexColumn = (e.currentTarget as HTMLInputElement).value)} placeholder="column" class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
						<label class="flex items-center gap-2 text-gray-600"><input type="checkbox" checked={uniqueIndex} onchange={(e) => (uniqueIndex = (e.currentTarget as HTMLInputElement).checked)} />Unique</label>
					</div>
				{/if}

				{#if mode === "create_relation"}
					<div class="grid grid-cols-2 gap-2">
						<input value={fkName} oninput={(e) => (fkName = (e.currentTarget as HTMLInputElement).value)} placeholder="constraint name" class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
						<input value={fkColumn} oninput={(e) => (fkColumn = (e.currentTarget as HTMLInputElement).value)} placeholder="column" class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
						<input value={fkRefSchema} oninput={(e) => (fkRefSchema = (e.currentTarget as HTMLInputElement).value)} placeholder="ref schema" class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
						<input value={fkRefTable} oninput={(e) => (fkRefTable = (e.currentTarget as HTMLInputElement).value)} placeholder="ref table" class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
						<input value={fkRefColumn} oninput={(e) => (fkRefColumn = (e.currentTarget as HTMLInputElement).value)} placeholder="ref column" class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
					</div>
				{/if}

				{#if mode === "create_partition"}
					<input value={partitionColumn} oninput={(e) => (partitionColumn = (e.currentTarget as HTMLInputElement).value)} placeholder="partition column" class="h-9 px-2 rounded-md bg-white border border-gray-200 text-gray-900 outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500" />
				{/if}

				{#if message}
					<div class={`text-xs px-3 py-2 rounded-md border ${messageKind === "ok" ? "bg-emerald-50 border-emerald-200 text-emerald-700" : "bg-red-50 border-red-200 text-red-700"}`}>{message}</div>
				{/if}
			</div>

			<div class="px-5 py-4 border-t border-gray-200 flex justify-end gap-2 bg-gray-50">
				<button onclick={onClose} class="h-9 px-4 rounded-md text-sm border border-gray-200 bg-white text-gray-700 hover:bg-gray-100">Close</button>
				<button onclick={submit} disabled={busy} class="h-9 px-4 rounded-md text-sm border border-emerald-500 bg-emerald-500 text-white font-medium hover:bg-emerald-600 hover:border-emerald-600 disabled:opacity-60">{busy ? "Running..." : "Execute"}</button>
			</div>
		</div>
	</div>
{/if}




