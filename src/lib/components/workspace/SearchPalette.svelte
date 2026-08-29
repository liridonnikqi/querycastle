<script lang="ts">
	import { Search, Table2, Columns2, KeyRound, ChevronRight, X, Braces, Hash, Play, Eye, Layers, Zap } from '@lucide/svelte';
	import type { DatabaseExplorer, DatabaseRoutine, DatabaseSequence } from '$lib/rpc';
	import { routineSignature, schemaRoutines, schemaSequences, tableIndexes, tableTriggers } from '$lib/utils/schema-objects';

	let {
		open,
		searchQuery,
		explorer,
		onSearchChange,
		onClose,
		onSelectTable,
		onSelectSchema,
		onSelectRoutine,
		onSelectSequence,
		onSelectIndex,
		onSelectTrigger,
	}: {
		open: boolean;
		searchQuery: string;
		explorer: DatabaseExplorer | null;
		onSearchChange: (value: string) => void;
		onClose: () => void;
		onSelectTable: (schema: string, table: string) => void;
		onSelectSchema: (schema: string) => void;
		onSelectRoutine: (routine: DatabaseRoutine) => void;
		onSelectSequence: (sequence: DatabaseSequence) => void;
		onSelectIndex: (schema: string, table: string, name: string) => void;
		onSelectTrigger: (schema: string, table: string, name: string) => void;
	} = $props();

	let inputEl: HTMLInputElement | null = $state(null);
	let selectedIndex = $state(0);
	// svelte-ignore state_referenced_locally
	let internalQuery = $state(searchQuery);

	$effect(() => {
		internalQuery = searchQuery;
	});

	$effect(() => {
		if (open) {
			selectedIndex = 0;
			// focus next tick
			setTimeout(() => inputEl?.focus(), 30);
		}
	});

	type Result =
		| { kind: 'schema'; schema: string }
		| { kind: 'table'; schema: string; table: string; tableKind: string }
		| { kind: 'column'; schema: string; table: string; column: string }
		| { kind: 'routine'; routine: DatabaseRoutine }
		| { kind: 'sequence'; sequence: DatabaseSequence }
		| { kind: 'index'; schema: string; table: string; name: string }
		| { kind: 'trigger'; schema: string; table: string; name: string };

	let results = $derived.by(() => {
		if (!explorer) return [] as Result[];
		const q = internalQuery.trim().toLowerCase();
		const out: Result[] = [];
		for (const schema of explorer.schemas) {
			if (!q || schema.name.toLowerCase().includes(q)) {
				out.push({ kind: 'schema', schema: schema.name });
			}
			for (const table of schema.tables) {
				const tableMatch = !q || table.name.toLowerCase().includes(q) || `${schema.name}.${table.name}`.toLowerCase().includes(q);
				if (tableMatch) {
					out.push({ kind: 'table', schema: schema.name, table: table.name, tableKind: table.kind });
				}
				for (const col of table.columns) {
					if (q && col.name.toLowerCase().includes(q)) {
						out.push({ kind: 'column', schema: schema.name, table: table.name, column: col.name });
					}
				}
				if (q) {
					for (const index of tableIndexes(table)) {
						if (index.name.toLowerCase().includes(q)) {
							out.push({ kind: 'index', schema: schema.name, table: table.name, name: index.name });
						}
					}
					for (const trigger of tableTriggers(table)) {
						if (trigger.name.toLowerCase().includes(q)) {
							out.push({ kind: 'trigger', schema: schema.name, table: table.name, name: trigger.name });
						}
					}
				}
			}
			for (const routine of schemaRoutines(schema)) {
				if (q && (routine.name.toLowerCase().includes(q) || routineSignature(routine).toLowerCase().includes(q))) {
					out.push({ kind: 'routine', routine });
				}
			}
			for (const sequence of schemaSequences(schema)) {
				if (q && sequence.name.toLowerCase().includes(q)) {
					out.push({ kind: 'sequence', sequence });
				}
			}
		}
		return out.slice(0, 50);
	});

	function handleInput(e: Event) {
		const v = (e.currentTarget as HTMLInputElement).value;
		internalQuery = v;
		onSearchChange(v);
		selectedIndex = 0;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onClose();
		} else if (e.key === 'ArrowDown') {
			e.preventDefault();
			selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, 0);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			const r = results[selectedIndex];
			if (r) selectResult(r);
		}
	}

	function selectResult(r: Result) {
		if (r.kind === 'table') {
			onSelectTable(r.schema, r.table);
			onClose();
		} else if (r.kind === 'schema') {
			onSelectSchema(r.schema);
			onClose();
		} else if (r.kind === 'column') {
			onSelectTable(r.schema, r.table);
			onClose();
		} else if (r.kind === 'routine') {
			onSelectRoutine(r.routine);
			onClose();
		} else if (r.kind === 'sequence') {
			onSelectSequence(r.sequence);
			onClose();
		} else if (r.kind === 'index') {
			onSelectIndex(r.schema, r.table, r.name);
			onClose();
		} else if (r.kind === 'trigger') {
			onSelectTrigger(r.schema, r.table, r.name);
			onClose();
		}
	}

	function handleBackdropKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[80] flex items-start justify-center pt-[18vh] p-4 bg-black/40 backdrop-blur-[2px]"
		onclick={onClose}
		onkeydown={handleBackdropKeydown}
		role="presentation"
	>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="w-[640px] max-w-[95vw] bg-white rounded-xl shadow-[0_20px_60px_rgba(0,0,0,0.2)] border border-gray-200 overflow-hidden flex flex-col"
			onclick={(e) => e.stopPropagation()}
		>
			<div class="flex items-center gap-3 px-4 h-[56px] border-b border-gray-100 shrink-0">
				<Search size={18} class="text-gray-400 shrink-0" />
				<input
					bind:this={inputEl}
					type="text"
					value={internalQuery}
					oninput={handleInput}
					onkeydown={handleKeydown}
					placeholder="Search tables, views, functions..."
					class="flex-1 h-full bg-transparent outline-none text-[15px] placeholder-gray-400 text-gray-900"
					autocomplete="off"
					spellcheck={false}
				/>
				{#if internalQuery}
					<button
						onclick={() => {
							internalQuery = '';
							onSearchChange('');
							inputEl?.focus();
						}}
						class="w-7 h-7 rounded-full bg-gray-100 hover:bg-gray-200 text-gray-500 flex items-center justify-center"
						aria-label="Clear"
					>
						<X size={14} />
					</button>
				{/if}
				<div class="hidden sm:flex items-center gap-1 ml-2">
					<kbd class="px-1.5 py-0.5 bg-gray-100 border border-gray-200 rounded text-[10px] font-medium text-gray-600">ESC</kbd>
				</div>
			</div>

			<div class="max-h-[50vh] overflow-y-auto overscroll-contain p-2">
				{#if !explorer}
					<div class="px-3 py-8 text-center text-sm text-gray-500">Connect to a database to search.</div>
				{:else if results.length === 0}
					<div class="px-3 py-8 text-center">
						<div class="text-sm text-gray-700">No results for "{internalQuery}"</div>
						<div class="text-xs text-gray-500 mt-1">Try a different term</div>
					</div>
				{:else}
					<div class="space-y-1">
						{#each results as r, i}
							<button
								onclick={() => selectResult(r)}
								onmouseenter={() => (selectedIndex = i)}
								class={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-left text-sm transition
									${i === selectedIndex ? 'bg-[#1c1c1e] text-white' : 'text-gray-700 hover:bg-gray-50'}`}
							>
								<div class={`w-7 h-7 rounded-md flex items-center justify-center shrink-0 ${i === selectedIndex ? 'bg-white/15 text-white' : r.kind === 'schema' ? 'bg-amber-100 text-amber-600' : r.kind === 'table' && r.tableKind === 'view' ? 'bg-sky-100 text-sky-600' : r.kind === 'table' ? 'bg-emerald-100 text-emerald-600' : r.kind === 'routine' ? 'bg-violet-100 text-violet-600' : r.kind === 'sequence' ? 'bg-amber-100 text-amber-600' : r.kind === 'index' ? 'bg-slate-100 text-slate-600' : r.kind === 'trigger' ? 'bg-orange-100 text-orange-600' : 'bg-gray-100 text-gray-500'}`}>
									{#if r.kind === 'schema'}
										<Table2 size={14} />
									{:else if r.kind === 'table' && r.tableKind === 'view'}
										<Eye size={14} />
									{:else if r.kind === 'table'}
										<Columns2 size={14} />
									{:else if r.kind === 'routine'}
										{#if r.routine.kind === 'procedure'}
											<Play size={14} />
										{:else}
											<Braces size={14} />
										{/if}
									{:else if r.kind === 'sequence'}
										<Hash size={14} />
									{:else if r.kind === 'index'}
										<Layers size={14} />
									{:else if r.kind === 'trigger'}
										<Zap size={14} />
									{:else if r.column.endsWith('_id')}
										<KeyRound size={13} />
									{:else}
										<span class="text-[10px] font-bold">T</span>
									{/if}
								</div>
								<div class="flex-1 min-w-0">
									{#if r.kind === 'schema'}
										<div class="font-medium truncate">{r.schema}</div>
										<div class={`text-xs truncate ${i === selectedIndex ? 'text-white/60' : 'text-gray-500'}`}>Schema • {explorer.schemas.find(s=>s.name===r.schema)?.tables.length ?? 0} tables</div>
									{:else if r.kind === 'table'}
										<div class="font-medium truncate">{r.table}</div>
										<div class={`text-xs truncate ${i === selectedIndex ? 'text-white/60' : 'text-gray-500'}`}>{r.schema} • {r.tableKind === 'view' ? 'View' : 'Table'}</div>
									{:else if r.kind === 'routine'}
										<div class="font-medium truncate">{routineSignature(r.routine)}</div>
										<div class={`text-xs truncate ${i === selectedIndex ? 'text-white/60' : 'text-gray-500'}`}>{r.routine.schema} • {r.routine.kind === 'procedure' ? 'Procedure' : 'Function'}</div>
									{:else if r.kind === 'sequence'}
										<div class="font-medium truncate">{r.sequence.name}</div>
										<div class={`text-xs truncate ${i === selectedIndex ? 'text-white/60' : 'text-gray-500'}`}>{r.sequence.schema} • Sequence</div>
									{:else if r.kind === 'index'}
										<div class="font-medium truncate">{r.name}</div>
										<div class={`text-xs truncate ${i === selectedIndex ? 'text-white/60' : 'text-gray-500'}`}>{r.schema}.{r.table} • Index</div>
									{:else if r.kind === 'trigger'}
										<div class="font-medium truncate">{r.name}</div>
										<div class={`text-xs truncate ${i === selectedIndex ? 'text-white/60' : 'text-gray-500'}`}>{r.schema}.{r.table} • Trigger</div>
									{:else}
										<div class="font-medium truncate">{r.column}</div>
										<div class={`text-xs truncate ${i === selectedIndex ? 'text-white/60' : 'text-gray-500'}`}>{r.schema}.{r.table} • Column</div>
									{/if}
								</div>
								<ChevronRight size={14} class={i === selectedIndex ? 'text-white/50' : 'text-gray-400'} />
							</button>
						{/each}
					</div>
				{/if}
			</div>

			<div class="flex items-center justify-between px-3 py-2.5 border-t border-gray-100 bg-gray-50 text-[11px] text-gray-500">
				<div class="flex items-center gap-3">
					<span class="hidden sm:inline-flex items-center gap-1"><kbd class="px-1 py-0.5 bg-white border border-gray-200 rounded text-[10px]">↑↓</kbd> Navigate</span>
					<span class="inline-flex items-center gap-1"><kbd class="px-1 py-0.5 bg-white border border-gray-200 rounded text-[10px]">↵</kbd> Select</span>
					<span class="hidden sm:inline-flex items-center gap-1"><kbd class="px-1 py-0.5 bg-white border border-gray-200 rounded text-[10px]">ESC</kbd> Close</span>
				</div>
				<span class="text-gray-400">{results.length} results</span>
			</div>
		</div>
	</div>
{/if}
