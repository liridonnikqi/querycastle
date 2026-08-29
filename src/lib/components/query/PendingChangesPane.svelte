<script lang="ts">
	import type { PendingChangeCard } from '$lib/utils/pending-changes';

	let {
		open = true,
		changeCount,
		cards,
		sqlPreview,
		syncing = false,
		error = '',
		onClose,
		onClear,
		onCommit,
	}: {
		open?: boolean;
		changeCount: number;
		cards: PendingChangeCard[];
		sqlPreview: string;
		syncing?: boolean;
		error?: string;
		onClose: () => void;
		onClear: () => void;
		onCommit: () => void;
	} = $props();

	let view = $state<'visual' | 'sql'>('visual');
</script>

{#if open}
	<aside class="w-[300px] shrink-0 border-l border-gray-200 bg-white flex flex-col min-h-0">
		<div class="h-11 px-3 border-b border-gray-200 flex items-center justify-between gap-2 shrink-0">
			<div class="text-sm font-semibold text-gray-900">Pending Changes</div>
			<div class="flex items-center gap-1">
				<div class="flex rounded-md border border-gray-200 p-0.5 text-[11px]">
					<button
						type="button"
						class={`h-6 px-2 rounded ${view === 'visual' ? 'bg-gray-900 text-white' : 'text-gray-600 hover:bg-gray-50'}`}
						onclick={() => (view = 'visual')}
					>
						Visual
					</button>
					<button
						type="button"
						class={`h-6 px-2 rounded ${view === 'sql' ? 'bg-gray-900 text-white' : 'text-gray-600 hover:bg-gray-50'}`}
						onclick={() => (view = 'sql')}
					>
						SQL
					</button>
				</div>
				<button
					type="button"
					class="h-6 w-6 rounded text-gray-400 hover:text-gray-700 hover:bg-gray-50"
					onclick={onClose}
					aria-label="Close pending changes"
				>
					×
				</button>
			</div>
		</div>
		<div class="flex-1 overflow-y-auto p-3 space-y-2 min-h-0">
			{#if error}
				<div class="rounded-md border border-red-200 bg-red-50 px-2.5 py-2 text-xs text-red-700">
					{error}
				</div>
			{/if}
			{#if changeCount === 0}
				<div class="text-xs text-gray-500 px-1 py-6 text-center">No pending changes.</div>
			{:else if view === 'sql'}
				<pre class="font-mono-code text-[11px] text-gray-700 whitespace-pre-wrap break-words rounded-md border border-gray-200 bg-gray-50 p-2.5">{sqlPreview || 'Nothing to commit.'}</pre>
			{:else}
				{#each cards as card (card.id)}
					<div class="rounded-md border border-gray-200 overflow-hidden">
						<div class="px-2.5 py-1.5 bg-gray-50 border-b border-gray-200 flex items-center gap-2">
							<span
								class={`h-4 min-w-4 px-1 rounded text-[10px] font-bold leading-4 text-center ${card.kind === 'update' ? 'bg-amber-100 text-amber-800' : card.kind === 'insert' ? 'bg-emerald-100 text-emerald-800' : 'bg-red-100 text-red-700'}`}
							>
								{card.badge}
							</span>
							<div class="truncate text-[11px] text-gray-600" title={card.title}>{card.title}</div>
						</div>
						<div class="text-[11px] font-mono">
							{#if card.before != null}
								<div class="px-2.5 py-1 bg-red-50 text-red-800">
									<span class="text-red-400 mr-1">-</span>{card.before}
								</div>
							{/if}
							{#if card.after != null}
								<div class="px-2.5 py-1 bg-emerald-50 text-emerald-800">
									<span class="text-emerald-500 mr-1">+</span>{card.after}
								</div>
							{/if}
						</div>
					</div>
				{/each}
			{/if}
		</div>
		<div class="h-12 px-3 border-t border-gray-200 flex items-center justify-between gap-2 shrink-0 bg-gray-50">
			<button
				type="button"
				class="text-xs text-gray-500 hover:text-gray-800 disabled:opacity-40"
				disabled={changeCount === 0 || syncing}
				onclick={onClear}
			>
				Clear all
			</button>
			<button
				type="button"
				class="h-8 px-3 rounded-md bg-emerald-500 text-white text-xs font-medium hover:bg-emerald-600 disabled:opacity-60"
				disabled={changeCount === 0 || syncing}
				onclick={onCommit}
			>
				{syncing ? 'Committing…' : `Commit all (${changeCount})`}
			</button>
		</div>
	</aside>
{/if}
