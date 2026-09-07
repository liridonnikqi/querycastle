<script lang="ts">
	import { Trash2, X } from '@lucide/svelte';
	import type { GridColumnMeta } from '$lib/utils/grid-editors';
	import { displayCellText } from '$lib/utils/grid-editors';
	import { HIDDEN_ROW_ID_COLUMN } from '$lib/utils/dialect';
	import ColumnTypeIcon from '$lib/components/query/ColumnTypeIcon.svelte';

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

	function metaFor(column: string) {
		return metas.find((item) => item.name === column);
	}

	let jsonText = $derived.by(() => {
		const body: Record<string, unknown> = {};
		for (const column of columns) {
			if (column === HIDDEN_ROW_ID_COLUMN) continue;
			body[column] = values[column] ?? null;
		}
		return JSON.stringify(body, null, 2);
	});
</script>

{#if open}
	<aside class="w-[300px] shrink-0 border-l border-qc-border bg-qc-panel flex flex-col min-h-0">
		<div class="h-9 flex items-center justify-between px-3 border-b border-qc-border shrink-0">
			<span class="text-[13px] font-medium text-qc-fg truncate">{rowLabel}</span>
			<button
				type="button"
				class="w-6 h-6 rounded flex items-center justify-center text-qc-muted hover:bg-qc-hover"
				onclick={onClose}
				aria-label="Close inspector"
			>
				<X size={14} />
			</button>
		</div>
		<div class="flex items-center h-8 border-b border-qc-border px-2 gap-0.5 shrink-0">
			<button
				type="button"
				class={`h-6 px-2.5 rounded text-[11px] font-medium transition-colors duration-150 ${tab === 'fields' ? 'text-qc-fg bg-qc-hover' : 'text-qc-muted hover:text-qc-subtle'}`}
				onclick={() => (tab = 'fields')}
			>
				Fields
			</button>
			<button
				type="button"
				class={`h-6 px-2.5 rounded text-[11px] font-medium transition-colors duration-150 ${tab === 'json' ? 'text-qc-fg bg-qc-hover' : 'text-qc-muted hover:text-qc-subtle'}`}
				onclick={() => (tab = 'json')}
			>
				JSON
			</button>
			<button
				type="button"
				class={`h-6 px-2.5 rounded text-[11px] font-medium transition-colors duration-150 ${tab === 'sql' ? 'text-qc-fg bg-qc-hover' : 'text-qc-muted hover:text-qc-subtle'}`}
				onclick={() => (tab = 'sql')}
			>
				SQL
			</button>
		</div>
		<div class="flex-1 overflow-y-auto px-3 py-3 space-y-3 min-h-0">
			{#if tab === 'fields'}
				{#each columns as column}
					{#if column !== HIDDEN_ROW_ID_COLUMN}
						{@const meta = metaFor(column)}
						{@const locked = meta?.isAuto || meta?.isPrimary}
						<div>
							<label class="flex items-center gap-1.5 text-[11px] text-qc-muted mb-1.5">
								<span class="text-qc-fg font-medium">{column}</span>
								<ColumnTypeIcon {meta} />
								<span class="text-[10px] uppercase tracking-wide">{meta?.dataType ?? 'text'}</span>
							</label>
							{#if meta?.kind === 'text' && String(values[column] ?? '').length > 48}
								<textarea
									value={values[column] == null ? '' : String(values[column])}
									disabled={!editable || locked}
									oninput={(event) => onFieldChange(column, event.currentTarget.value)}
									class="field-input w-full min-h-[72px] px-2.5 py-2 text-[12px] font-mono resize-y disabled:opacity-60"
								></textarea>
							{:else}
								<input
									value={values[column] == null ? '' : String(values[column])}
									disabled={!editable || locked}
									oninput={(event) => onFieldChange(column, event.currentTarget.value)}
									class="field-input w-full h-8 px-2.5 text-[12px] font-mono disabled:opacity-60"
									title={displayCellText(values[column], meta)}
								/>
							{/if}
						</div>
					{/if}
				{/each}
			{:else if tab === 'json'}
				<pre class="font-mono text-[11px] text-qc-data whitespace-pre-wrap break-words rounded-md border border-qc-border bg-qc-bg p-2.5">{jsonText}</pre>
			{:else}
				<pre class="font-mono text-[11px] text-qc-data whitespace-pre-wrap break-words rounded-md border border-qc-border bg-qc-bg p-2.5">{sqlPreview || 'No pending SQL for this row.'}</pre>
			{/if}
		</div>
		{#if editable}
			<div class="p-3 border-t border-qc-border shrink-0">
				<button
					type="button"
					onclick={onDelete}
					class="w-full h-8 btn-danger text-[12px] font-medium inline-flex items-center justify-center gap-1.5"
				>
					<Trash2 size={12} />
					Delete 1 row
				</button>
			</div>
		{/if}
	</aside>
{/if}
