<script lang="ts">
	import { Check, Copy, KeyRound, Link2, Search, Trash2, X } from '$lib/icons';
	import type { DatabaseForeignKey } from '$lib/rpc';
	import { HIDDEN_ROW_ID_COLUMN } from '$lib/utils/dialect';
	import {
		displayCellText,
		formatBooleanLabel,
		type GridColumnMeta,
	} from '$lib/utils/grid-editors';

	let {
		open,
		rowLabel,
		columns,
		values,
		metas,
		sqlPreview,
		editable = false,
		onClose,
		onFieldChange,
		onDelete,
	}: {
		open: boolean;
		rowLabel: string;
		columns: string[];
		values: Record<string, unknown>;
		metas: GridColumnMeta[];
		sqlPreview: string;
		editable?: boolean;
		onClose: () => void;
		onFieldChange: (column: string, raw: string) => void;
		onDelete: () => void;
	} = $props();

	let tab = $state<'fields' | 'json' | 'sql'>('fields');
	let fieldFilter = $state('');
	let copied = $state(false);
	let copiedColumn = $state<string | null>(null);
	let copyTimer: ReturnType<typeof setTimeout> | null = null;
	let copyFieldTimer: ReturnType<typeof setTimeout> | null = null;

	function metaFor(column: string) {
		return metas.find((item) => item.name === column);
	}

	const visibleColumns = $derived(
		columns.filter((column) => column !== HIDDEN_ROW_ID_COLUMN),
	);

	const showFilter = $derived(visibleColumns.length > 8 || fieldFilter.trim().length > 0);

	const filteredColumns = $derived.by(() => {
		const q = fieldFilter.trim().toLowerCase();
		if (!q) return visibleColumns;
		return visibleColumns.filter((column) => {
			if (column.toLowerCase().includes(q)) return true;
			const meta = metaFor(column);
			if (meta?.dataType.toLowerCase().includes(q)) return true;
			const val = values[column];
			return val != null && editText(val).toLowerCase().includes(q);
		});
	});

	let jsonText = $derived.by(() => {
		const body: Record<string, unknown> = {};
		for (const column of visibleColumns) {
			body[column] = values[column] ?? null;
		}
		return JSON.stringify(body, null, 2);
	});

	function fieldTip(meta: GridColumnMeta | undefined): string {
		const type = meta?.dataType || 'text';
		if (meta?.fk) {
			const ref = fkTarget(meta.fk);
			return meta.isPrimary ? `Primary key · ${type} → ${ref}` : `${type} → ${ref}`;
		}
		if (meta?.isPrimary) return `Primary key · ${type}`;
		return type;
	}

	function fkTarget(fk: DatabaseForeignKey): string {
		return `${fk.referencedSchema}.${fk.referencedTable}.${fk.referencedColumn}`;
	}

	function editText(value: unknown): string {
		if (value == null) return '';
		if (typeof value === 'object') return JSON.stringify(value, null, 2);
		return String(value);
	}

	function readText(value: unknown, meta: GridColumnMeta | undefined): string {
		if (typeof value === 'object' && value != null) return JSON.stringify(value);
		return displayCellText(value, meta);
	}

	function selectTab(next: 'fields' | 'json' | 'sql') {
		tab = next;
		copied = false;
	}

	async function copyActive() {
		const content = tab === 'sql' ? sqlPreview || '' : jsonText;
		try {
			await navigator.clipboard.writeText(content);
			copied = true;
			if (copyTimer) clearTimeout(copyTimer);
			copyTimer = setTimeout(() => {
				copied = false;
			}, 1500);
		} catch {
			// Ignore clipboard failures in restricted webviews.
		}
	}

	async function copyFieldValue(column: string, rawText: string) {
		try {
			await navigator.clipboard.writeText(rawText);
			copiedColumn = column;
			if (copyFieldTimer) clearTimeout(copyFieldTimer);
			copyFieldTimer = setTimeout(() => {
				copiedColumn = null;
			}, 1200);
		} catch {
			// Ignore clipboard failures in restricted webviews.
		}
	}

	const controlClass =
		'h-7 w-full rounded border border-qc-border bg-qc-bg px-1.5 font-mono-code text-[12px] text-qc-fg outline-none focus:border-qc-focus-border placeholder:italic placeholder:text-qc-muted';
</script>

{#if open}
	<aside
		class="flex w-[clamp(300px,28vw,350px)] min-h-0 shrink-0 flex-col border-l border-qc-border bg-qc-panel"
	>
		<div
			class="flex h-9 shrink-0 items-center justify-between gap-2 border-b border-qc-border px-3"
		>
			<div class="truncate text-[12px] font-semibold text-qc-fg">{rowLabel}</div>
			<div class="flex shrink-0 items-center gap-1">
				<div class="qc-segmented">
					<button
						type="button"
						class={`qc-seg-btn ${tab === 'fields' ? 'is-on' : ''}`}
						onclick={() => selectTab('fields')}
					>
						Fields
					</button>
					<button
						type="button"
						class={`qc-seg-btn ${tab === 'json' ? 'is-on' : ''}`}
						onclick={() => selectTab('json')}
					>
						JSON
					</button>
					<button
						type="button"
						class={`qc-seg-btn ${tab === 'sql' ? 'is-on' : ''}`}
						onclick={() => selectTab('sql')}
					>
						SQL
					</button>
				</div>
				{#if tab !== 'fields'}
					<button
						type="button"
						class="flex h-6 w-6 items-center justify-center rounded text-qc-muted hover:bg-qc-hover hover:text-qc-fg"
						onclick={copyActive}
						aria-label={tab === 'sql' ? 'Copy SQL' : 'Copy JSON'}
						data-tip={copied ? 'Copied' : tab === 'sql' ? 'Copy SQL' : 'Copy JSON'}
					>
						{#if copied}
							<Check size={13} class="text-qc-insert-accent" />
						{:else}
							<Copy size={13} />
						{/if}
					</button>
				{/if}
				<button
					type="button"
					class="flex h-6 w-6 shrink-0 items-center justify-center rounded text-qc-muted hover:bg-qc-hover hover:text-qc-fg"
					onclick={onClose}
					aria-label="Close inspector"
				>
					<X size={13} />
				</button>
			</div>
		</div>

		{#if tab === 'fields' && showFilter}
			<div class="relative shrink-0 border-b border-qc-border-subtle px-3 py-1.5">
				<Search
					size={12}
					class="pointer-events-none absolute left-5 top-1/2 -translate-y-1/2 text-qc-muted"
				/>
				<input
					type="text"
					bind:value={fieldFilter}
					placeholder="Filter…"
					aria-label="Filter fields"
					class={`h-7 w-full rounded-md border border-qc-border bg-qc-bg pl-7 text-[12px] text-qc-fg outline-none focus:border-qc-muted ${fieldFilter ? 'pr-7' : 'pr-2'}`}
				/>
				{#if fieldFilter}
					<button
						type="button"
						onclick={() => (fieldFilter = '')}
						aria-label="Clear filter"
						class="absolute right-4 top-1/2 flex h-4 w-4 -translate-y-1/2 items-center justify-center rounded text-qc-muted hover:text-qc-fg"
					>
						<X size={12} />
					</button>
				{/if}
			</div>
		{/if}

		<div class="min-h-0 flex-1 overflow-y-auto">
			{#if tab === 'fields'}
				{#if visibleColumns.length === 0}
					<div class="px-3 py-8 text-center text-xs text-qc-muted">No fields.</div>
				{:else if filteredColumns.length === 0}
					<div class="px-3 py-8 text-center text-xs text-qc-muted">No matches.</div>
				{:else}
					<div class="divide-y divide-qc-border-subtle">
						{#each filteredColumns as column (column)}
							{@const meta = metaFor(column)}
							{@const locked = meta?.isAuto || meta?.isPrimary}
							{@const value = values[column]}
							{@const isNull = value == null}
							{@const rawText = editText(value)}
							{@const isMultiline =
								rawText.includes('\n') ||
								(meta?.kind === 'text' && rawText.length > 120)}
							{@const boolLabel =
								meta?.kind === 'boolean' ? formatBooleanLabel(value) : ''}
							{@const editing = editable && !locked}
							<div class="group px-3 py-2">
								<div class="flex h-5 items-center gap-1.5">
									<div
										class="flex min-w-0 flex-1 items-center gap-1"
										data-tip={fieldTip(meta)}
									>
										{#if meta?.isPrimary}
											<KeyRound size={12} class="shrink-0 text-qc-muted" />
										{/if}
										{#if meta?.fk}
											<Link2 size={12} class="shrink-0 text-qc-muted" />
										{/if}
										<span class="truncate text-[12px] text-qc-fg" title={column}
											>{column}</span
										>
									</div>
									{#if isNull}
										<span class="h-4 w-4 shrink-0"></span>
									{:else}
										<button
											type="button"
											onclick={() => copyFieldValue(column, rawText)}
											aria-label={`Copy ${column}`}
											data-tip={copiedColumn === column ? 'Copied' : 'Copy value'}
											class={`flex h-4 w-4 shrink-0 items-center justify-center ${copiedColumn === column ? 'text-qc-insert-accent opacity-100' : 'text-qc-muted opacity-0 group-hover:opacity-100 focus:opacity-100 hover:text-qc-fg'}`}
										>
											{#if copiedColumn === column}
												<Check size={12} />
											{:else}
												<Copy size={12} />
											{/if}
										</button>
									{/if}
									<span
										class="max-w-[46%] shrink-0 truncate font-mono text-[11px] text-qc-muted"
										title={meta?.dataType ?? 'text'}
									>
										{meta?.dataType ?? 'text'}
									</span>
								</div>

								<div class="mt-1">
									{#if editing && isMultiline}
										<textarea
											rows={3}
											value={rawText}
											oninput={(event) =>
												onFieldChange(column, event.currentTarget.value)}
											placeholder={isNull ? 'NULL' : ''}
											class="min-h-[4.5rem] w-full resize-y rounded border border-qc-border bg-qc-bg px-1.5 py-1 font-mono-code text-[12px] leading-5 text-qc-fg outline-none focus:border-qc-focus-border placeholder:italic placeholder:text-qc-muted"
										></textarea>
									{:else if editing && meta?.kind === 'boolean' && (isNull || boolLabel !== '')}
										<select
											class={controlClass}
											value={boolLabel === 'True'
												? 'true'
												: boolLabel === 'False'
													? 'false'
													: ''}
											onchange={(event) =>
												onFieldChange(column, event.currentTarget.value)}
										>
											{#if !meta?.notNull || isNull}
												<option value="">Empty</option>
											{/if}
											<option value="true">True</option>
											<option value="false">False</option>
										</select>
									{:else if editing}
										<input
											value={rawText}
											oninput={(event) =>
												onFieldChange(column, event.currentTarget.value)}
											placeholder={isNull ? 'NULL' : ''}
											title={rawText}
											class={controlClass}
										/>
									{:else if isNull}
										<span class="text-[12px] italic leading-5 text-qc-muted">NULL</span>
									{:else if rawText === ''}
										<span class="text-[12px] italic leading-5 text-qc-muted">(empty)</span>
									{:else if isMultiline}
										<pre
											class="m-0 select-text whitespace-pre-wrap break-words font-mono-code text-[12px] leading-5 text-qc-data">{rawText}</pre>
									{:else}
										<div
											class="select-text break-all font-mono-code text-[12px] leading-5 text-qc-data"
											title={rawText}
										>
											{readText(value, meta)}
										</div>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				{/if}
			{:else if tab === 'json'}
				<pre
					class="select-text whitespace-pre-wrap break-words px-3 py-3 font-mono-code text-[11px] leading-relaxed text-qc-data">{jsonText}</pre>
			{:else}
				<pre
					class="select-text whitespace-pre-wrap break-words px-3 py-3 font-mono-code text-[11px] leading-relaxed text-qc-data">{sqlPreview ||
						'No pending SQL for this row.'}</pre>
			{/if}
		</div>

		{#if editable}
			<div
				class="flex h-10 shrink-0 items-center justify-end border-t border-qc-border-subtle px-3"
			>
				<button
					type="button"
					onclick={onDelete}
					class="inline-flex items-center gap-1.5 text-xs text-qc-muted hover:text-qc-danger"
				>
					<Trash2 size={12} />
					Delete row
				</button>
			</div>
		{/if}
	</aside>
{/if}
