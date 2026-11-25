<script lang="ts">
	import { Search, Table2, KeyRound, X, Braces, Hash, Play, Eye, Layers, Zap, Type } from '$lib/icons';
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
	let internalQuery = $state('');

	$effect(() => {
		if (open) {
			internalQuery = searchQuery;
			selectedIndex = 0;
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

	let backdropPointerDown = false;

	function handleBackdropPointerDown(e: PointerEvent) {
		backdropPointerDown = e.target === e.currentTarget;
	}

	function handleBackdropClick(e: MouseEvent) {
		const startedOnBackdrop = backdropPointerDown;
		backdropPointerDown = false;
		if (startedOnBackdrop && e.target === e.currentTarget) onClose();
	}

	function handleBackdropKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[80] flex items-start justify-center pt-[16vh] p-4 bg-black/45 backdrop-blur-[2px]"
		onpointerdown={handleBackdropPointerDown}
		onclick={handleBackdropClick}
		onkeydown={handleBackdropKeydown}
		role="presentation"
	>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="w-[560px] max-w-[94vw] bg-qc-elevated rounded-xl shadow-[0_24px_64px_rgba(0,0,0,0.45)] border border-qc-border overflow-hidden flex flex-col"
		>
			<div class="flex items-center gap-2.5 px-4 h-12 border-b border-qc-border-subtle shrink-0">
				<Search size={16} class="text-qc-muted shrink-0" />
				<input
					bind:this={inputEl}
					type="text"
					value={internalQuery}
					oninput={handleInput}
					onkeydown={handleKeydown}
					placeholder="Search tables, views, columns, routines…"
					class="flex-1 h-full bg-transparent outline-none text-[14px] placeholder:text-qc-muted text-qc-fg"
					autocomplete="off"
					spellcheck={false}
				/>
				{#if internalQuery}
					<button
						type="button"
						onclick={() => {
							internalQuery = '';
							onSearchChange('');
							inputEl?.focus();
						}}
						class="w-6 h-6 rounded hover:bg-qc-hover text-qc-muted hover:text-qc-fg flex items-center justify-center shrink-0"
						aria-label="Clear"
					>
						<X size={13} />
					</button>
				{/if}
				<span class="hidden sm:inline text-[11px] font-mono text-qc-muted/70 shrink-0">esc</span>
			</div>

			<div class="max-h-[48vh] overflow-y-auto overscroll-contain p-1.5">
				{#if !explorer}
					<div class="px-3 py-8 text-center text-[13px] text-qc-muted">Connect to a database to search.</div>
				{:else if results.length === 0}
					<div class="px-3 py-8 text-center">
						<div class="text-[13px] text-qc-fg">No results for "{internalQuery}"</div>
						<div class="text-xs text-qc-muted mt-1">Try a different term</div>
					</div>
				{:else}
					<div class="space-y-0.5">
						{#each results as r, i}
							<button
								type="button"
								onclick={() => selectResult(r)}
								onmouseenter={() => (selectedIndex = i)}
								class={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left text-[13px] transition-colors ${
									i === selectedIndex
										? 'bg-qc-sidebar-active text-qc-fg'
										: 'text-qc-fg hover:bg-qc-hover/60'
								}`}
							>
								<span
									class={`shrink-0 ${
										i === selectedIndex ? 'text-qc-fg' : 'text-qc-muted'
									}`}
								>
									{#if r.kind === 'schema'}
										<Layers size={15} />
									{:else if r.kind === 'table' && r.tableKind === 'view'}
										<Eye size={15} />
									{:else if r.kind === 'table'}
										<Table2 size={15} />
									{:else if r.kind === 'routine'}
										{#if r.routine.kind === 'procedure'}
											<Play size={15} />
										{:else}
											<Braces size={15} />
										{/if}
									{:else if r.kind === 'sequence'}
										<Hash size={15} />
									{:else if r.kind === 'index'}
										<KeyRound size={15} />
									{:else if r.kind === 'trigger'}
										<Zap size={15} />
									{:else if r.column.endsWith('_id')}
										<KeyRound size={14} />
									{:else}
										<Type size={14} />
									{/if}
								</span>

								{#if r.kind === 'schema'}
									<span class="font-medium truncate flex-1">{r.schema}</span>
									<span class="text-[11px] text-qc-muted shrink-0">
										Schema · {explorer.schemas.find((s) => s.name === r.schema)?.tables.length ?? 0} tables
									</span>
								{:else if r.kind === 'table'}
									<span class="font-medium truncate flex-1">{r.table}</span>
									<span class="text-[11px] text-qc-muted shrink-0">
										{r.schema} · {r.tableKind === 'view' ? 'View' : 'Table'}
									</span>
								{:else if r.kind === 'routine'}
									<span class="font-medium truncate flex-1">{routineSignature(r.routine)}</span>
									<span class="text-[11px] text-qc-muted shrink-0">
										{r.routine.schema} · {r.routine.kind === 'procedure' ? 'Procedure' : 'Function'}
									</span>
								{:else if r.kind === 'sequence'}
									<span class="font-medium truncate flex-1">{r.sequence.name}</span>
									<span class="text-[11px] text-qc-muted shrink-0">{r.sequence.schema} · Sequence</span>
								{:else if r.kind === 'index'}
									<span class="font-medium truncate flex-1">{r.name}</span>
									<span class="text-[11px] text-qc-muted shrink-0">{r.schema}.{r.table} · Index</span>
								{:else if r.kind === 'trigger'}
									<span class="font-medium truncate flex-1">{r.name}</span>
									<span class="text-[11px] text-qc-muted shrink-0">{r.schema}.{r.table} · Trigger</span>
								{:else}
									<span class="font-medium truncate flex-1">{r.column}</span>
									<span class="text-[11px] text-qc-muted shrink-0">{r.schema}.{r.table} · Column</span>
								{/if}
							</button>
						{/each}
					</div>
				{/if}
			</div>

			<div class="flex items-center justify-between px-3.5 py-2 border-t border-qc-border-subtle text-[11px] text-qc-muted/80">
				<span class="hidden sm:inline font-mono">↑↓ navigate · ↵ open · esc close</span>
				<span class="ml-auto tabular-nums">{results.length} {results.length === 1 ? 'result' : 'results'}</span>
			</div>
		</div>
	</div>
{/if}
