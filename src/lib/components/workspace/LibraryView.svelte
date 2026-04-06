<script lang="ts">
	import { FileCode2, Trash2 } from '@lucide/svelte';
	import type { QueryHistoryItem, SavedQueryItem } from '$lib/types';
	import type { MainView } from '$lib/utils/workspace';

	let {
		mainView,
		favoritesForConnection,
		selectedSavedQueryId,
		selectedSavedQuery,
		historyForConnection,
		selectedHistoryIndex,
		selectedHistoryQuery,
		onSelectSavedQuery,
		onDeleteSavedQuery,
		onOpenSavedQuery,
		onSelectHistory,
	}: {
		mainView: MainView;
		favoritesForConnection: SavedQueryItem[];
		selectedSavedQueryId: string;
		selectedSavedQuery: SavedQueryItem | null;
		historyForConnection: QueryHistoryItem[];
		selectedHistoryIndex: number;
		selectedHistoryQuery: QueryHistoryItem | null;
		onSelectSavedQuery: (id: string) => void;
		onDeleteSavedQuery: (id: string) => void;
		onOpenSavedQuery: (sql: string) => void;
		onSelectHistory: (index: number) => void;
	} = $props();
</script>

<section class="flex-1 min-w-0 flex border-l border-gray-100 bg-gray-50">
	<aside class="w-[260px] border-r border-gray-200 bg-white flex flex-col shrink-0">
		<div
			class="h-11 px-4 border-b border-gray-200 flex items-center text-xs font-semibold tracking-[0.08em] text-gray-500 uppercase"
		>
			Explorer
		</div>
		<div class="p-4 text-sm overflow-auto">
			{#if mainView === 'saved_queries'}
				<div class="space-y-3">
					<div>
						<div class="text-gray-700 font-medium mb-2">Saved Queries</div>
						<div class="space-y-1">
							{#if favoritesForConnection.length === 0}
								<div class="text-xs text-gray-500">No saved queries</div>
							{:else}
								{#each favoritesForConnection as item}
									<div
										class={`w-full px-2 py-1.5 rounded-md flex items-center gap-2 min-w-0 ${selectedSavedQueryId === item.id ? 'bg-gray-100 text-gray-900' : 'text-gray-700 hover:bg-gray-50'}`}
									>
										<button
											onclick={() => onSelectSavedQuery(item.id)}
											class="min-w-0 flex-1 text-left inline-flex items-center gap-2"
										>
											<FileCode2 size={14} class="shrink-0 text-emerald-500" />
											<span class="truncate">{item.title}</span>
										</button>
										<button
											onclick={() => onDeleteSavedQuery(item.id)}
											class="shrink-0 text-gray-400 hover:text-red-600"
											aria-label={`Delete ${item.title}`}
											title="Delete saved query"
										>
											<Trash2 size={14} />
										</button>
									</div>
								{/each}
							{/if}
						</div>
					</div>
				</div>
			{:else if mainView === 'last_queries'}
				<div class="space-y-2">
					<div class="text-gray-700 font-medium mb-2">Last Queries</div>
					<div class="space-y-1">
						{#if historyForConnection.length === 0}
							<div class="text-xs text-gray-500">No query history</div>
						{:else}
							{#each historyForConnection as item, index}
								<button
									onclick={() => onSelectHistory(index)}
									class={`w-full text-left px-2 py-1.5 rounded-md min-w-0 ${selectedHistoryIndex === index ? 'bg-gray-100 text-gray-900' : 'text-gray-700 hover:bg-gray-50'}`}
								>
									<div class="truncate font-mono-code text-xs">{item.sql}</div>
									<div class="mt-1 text-[10px] text-gray-500">
										{item.time} • {item.durationMs}ms
									</div>
								</button>
							{/each}
						{/if}
					</div>
				</div>
			{/if}
		</div>
	</aside>

	<div class="flex-1 min-w-0 flex flex-col">
		{#if mainView === 'saved_queries'}
			<div class="h-11 px-6 border-b border-gray-200 bg-white flex items-center justify-between">
				<div class="inline-flex items-center gap-2 min-w-0">
					<FileCode2 size={15} class="text-emerald-500 shrink-0" />
					<span class="text-sm font-semibold text-gray-900 truncate"
						>{selectedSavedQuery?.title ?? 'saved_query.sql'}</span
					>
				</div>
				<div class="flex items-center gap-2">
					<button
						onclick={() => {
							if (selectedSavedQuery) onDeleteSavedQuery(selectedSavedQuery.id);
						}}
						disabled={!selectedSavedQuery}
						class="h-8 px-3 rounded-md border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-50 disabled:opacity-60"
					>
						Remove
					</button>
					<button
						onclick={() => {
							if (selectedSavedQuery) onOpenSavedQuery(selectedSavedQuery.sql);
						}}
						disabled={!selectedSavedQuery}
						class="h-8 px-3 rounded-md border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-50 disabled:opacity-60"
					>
						Open in Editor
					</button>
				</div>
			</div>
			<div class="flex-1 overflow-auto p-6">
				{#if selectedSavedQuery}
					<div class="rounded-xl border border-gray-200 bg-white p-4">
						<pre
							class="font-mono-code text-sm leading-7 text-gray-700 whitespace-pre-wrap">{selectedSavedQuery.sql}</pre>
					</div>
				{:else}
					<div class="h-full flex items-center justify-center text-sm text-gray-500">
						No saved queries yet for this connection.
					</div>
				{/if}
			</div>
		{:else if mainView === 'last_queries'}
			<div class="h-11 px-6 border-b border-gray-200 bg-white flex items-center">
				<div class="inline-flex items-center gap-2 min-w-0">
					<FileCode2 size={15} class="text-emerald-500 shrink-0" />
					<span class="text-sm font-semibold text-gray-900 truncate">last_queries.sql</span>
				</div>
			</div>
			<div class="flex-1 overflow-auto p-6">
				{#if selectedHistoryQuery}
					<div class="rounded-xl border border-gray-200 bg-white p-4">
						<div class="text-xs text-gray-500 mb-3">
							{selectedHistoryQuery.time} • {selectedHistoryQuery.durationMs}ms • {selectedHistoryQuery.success
								? 'Success'
								: 'Error'}
						</div>
						<pre
							class="font-mono-code text-sm leading-7 text-gray-700 whitespace-pre-wrap">{selectedHistoryQuery.sql}</pre>
						{#if selectedHistoryQuery.error}
							<div class="mt-3 text-xs text-red-600">{selectedHistoryQuery.error}</div>
						{/if}
					</div>
				{:else}
					<div class="h-full flex items-center justify-center text-sm text-gray-500">
						No query history for this connection.
					</div>
				{/if}
			</div>
		{/if}
	</div>
</section>
