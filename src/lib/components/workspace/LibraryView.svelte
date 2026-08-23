<script lang="ts">
	import { Check, Clock, FileCode2, Search, Star, Trash2, X } from '@lucide/svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
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
		onDeleteHistory,
		onClearHistory,
		onClearSavedQueries,
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
		onDeleteHistory: (index: number) => void;
		onClearHistory: () => void;
		onClearSavedQueries: () => void;
	} = $props();

	let historySearch = $state('');
	let savedSearch = $state('');

	let filteredHistory = $derived.by(() => {
		const q = historySearch.trim().toLowerCase();
		const items = historyForConnection.map((item, originalIndex) => ({ ...item, originalIndex }));
		if (!q) return items;
		return items.filter((item) => item.sql.toLowerCase().includes(q) || item.time.toLowerCase().includes(q));
	});

	let filteredFavorites = $derived.by(() => {
		const q = savedSearch.trim().toLowerCase();
		if (!q) return favoritesForConnection;
		return favoritesForConnection.filter((item) => item.title.toLowerCase().includes(q) || item.sql.toLowerCase().includes(q));
	});

	let confirmDialog: { open: boolean; title: string; message: string; confirmLabel: string; onConfirm: () => void } | null = $state(null);

	function handleDeleteWithConfirm(index: number) {
		confirmDialog = {
			open: true,
			title: 'Delete query?',
			message: 'This will remove the query from history. This cannot be undone.',
			confirmLabel: 'Delete',
			onConfirm: () => {
				confirmDialog = null;
				onDeleteHistory(index);
			}
		};
	}
	function handleClearWithConfirm() {
		confirmDialog = {
			open: true,
			title: 'Clear history?',
			message: `This will delete all ${historyForConnection.length} queries from history for this connection. This cannot be undone.`,
			confirmLabel: 'Clear all',
			onConfirm: () => {
				confirmDialog = null;
				onClearHistory();
			}
		};
	}
	function handleDeleteSavedWithConfirm(id: string, title: string) {
		confirmDialog = {
			open: true,
			title: 'Delete saved query?',
			message: `Delete "${title}"? This cannot be undone.`,
			confirmLabel: 'Delete',
			onConfirm: () => {
				confirmDialog = null;
				onDeleteSavedQuery(id);
			}
		};
	}
	function handleClearSavedWithConfirm() {
		confirmDialog = {
			open: true,
			title: 'Clear saved queries?',
			message: `This will delete all ${favoritesForConnection.length} saved queries for this connection. This cannot be undone.`,
			confirmLabel: 'Clear all',
			onConfirm: () => {
				confirmDialog = null;
				onClearSavedQueries();
			}
		};
	}
</script>

<section class="flex-1 min-w-0 flex border-l border-gray-100 bg-gray-50">
	<aside class="w-[260px] border-r border-gray-200 bg-white flex flex-col shrink-0">
		<div class="p-3 text-sm overflow-auto">
			{#if mainView === 'saved_queries'}
				<div class="space-y-3">
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-1.5">
							<Star size={14} class="text-amber-500" />
							<span class="text-[11px] font-semibold tracking-widest text-gray-500 uppercase">Saved</span>
							<span class="px-1.5 py-0.5 rounded-full bg-gray-100 border border-gray-200 text-[10px] font-medium text-gray-600">{favoritesForConnection.length}</span>
						</div>
						{#if favoritesForConnection.length > 0}
							<button
								onclick={handleClearSavedWithConfirm}
								class="text-[11px] text-gray-500 hover:text-red-600 flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-red-50"
								title="Clear saved queries"
							>
								<Trash2 size={12} /> Clear
							</button>
						{/if}
					</div>
					{#if favoritesForConnection.length > 0}
						<div class="flex items-center gap-2 px-3 h-9 bg-white border border-gray-200 rounded-lg hover:border-gray-300 focus-within:border-gray-300 focus-within:ring-1 focus-within:ring-gray-200">
							<Search size={14} class="text-gray-400 shrink-0" />
							<input
								type="text"
								placeholder="Search saved..."
								bind:value={savedSearch}
								class="flex-1 h-full bg-transparent outline-none text-xs placeholder-gray-400 text-gray-900"
							/>
							{#if savedSearch}
								<button
									onclick={() => (savedSearch = '')}
									class="w-5 h-5 rounded-full bg-gray-100 hover:bg-gray-200 text-gray-500 flex items-center justify-center shrink-0"
									aria-label="Clear search"
								>
									<X size={12} />
								</button>
							{/if}
						</div>
					{/if}
					<div class="space-y-1">
						{#if favoritesForConnection.length === 0}
							<div class="py-8 text-center">
								<Star size={20} class="mx-auto text-gray-300 mb-2" />
								<div class="text-xs text-gray-500">No saved queries</div>
								<div class="text-[11px] text-gray-400 mt-1">Save a query to see it here</div>
							</div>
						{:else if filteredFavorites.length === 0}
							<div class="py-4 text-center text-xs text-gray-500">No matches for "{savedSearch}"</div>
						{:else}
							{#each filteredFavorites as item}
								<div
									class={`w-full px-2 py-1.5 rounded-md flex items-center gap-2 min-w-0 group/saved ${selectedSavedQueryId === item.id ? 'bg-gray-100 text-gray-900' : 'text-gray-700 hover:bg-gray-50'}`}
								>
									<button
										onclick={() => onSelectSavedQuery(item.id)}
										class="min-w-0 flex-1 text-left inline-flex items-center gap-2"
									>
										<FileCode2 size={14} class="shrink-0 text-emerald-500" />
										<span class="truncate text-xs">{item.title}</span>
									</button>
									<button
										onclick={() => handleDeleteSavedWithConfirm(item.id, item.title)}
										class="opacity-0 group-hover/saved:opacity-100 shrink-0 w-6 h-6 rounded flex items-center justify-center text-gray-400 hover:text-red-600 hover:bg-red-50 transition"
										aria-label={`Delete ${item.title}`}
										title="Delete saved query"
									>
										<Trash2 size={12} />
									</button>
								</div>
							{/each}
						{/if}
					</div>
				</div>
			{:else if mainView === 'history'}
				<div class="space-y-3">
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-1.5">
							<Clock size={14} class="text-gray-500" />
							<span class="text-[11px] font-semibold tracking-widest text-gray-500 uppercase">History</span>
							<span class="px-1.5 py-0.5 rounded-full bg-gray-100 border border-gray-200 text-[10px] font-medium text-gray-600">{historyForConnection.length}</span>
						</div>
						{#if historyForConnection.length > 0}
							<button
								onclick={handleClearWithConfirm}
								class="text-[11px] text-gray-500 hover:text-red-600 flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-red-50"
								title="Clear history"
							>
								<Trash2 size={12} /> Clear
							</button>
						{/if}
					</div>
					{#if historyForConnection.length > 0}
						<div class="flex items-center gap-2 px-3 h-9 bg-white border border-gray-200 rounded-lg hover:border-gray-300 focus-within:border-gray-300 focus-within:ring-1 focus-within:ring-gray-200">
							<Search size={14} class="text-gray-400 shrink-0" />
							<input
								type="text"
								placeholder="Search history..."
								bind:value={historySearch}
								class="flex-1 h-full bg-transparent outline-none text-xs placeholder-gray-400 text-gray-900"
							/>
							{#if historySearch}
								<button
									onclick={() => (historySearch = '')}
									class="w-5 h-5 rounded-full bg-gray-100 hover:bg-gray-200 text-gray-500 flex items-center justify-center shrink-0"
									aria-label="Clear search"
								>
									<X size={12} />
								</button>
							{/if}
						</div>
					{/if}
					<div class="space-y-1.5">
						{#if historyForConnection.length === 0}
							<div class="py-8 text-center">
								<Clock size={20} class="mx-auto text-gray-300 mb-2" />
								<div class="text-xs text-gray-500">No query history</div>
								<div class="text-[11px] text-gray-400 mt-1">Run a query to see it here</div>
							</div>
						{:else}
							{#each filteredHistory as item}
								<div class={`relative rounded-lg border group/history ${selectedHistoryIndex === item.originalIndex ? 'border-gray-300' : 'border-gray-200'}`}>
									<!-- svelte-ignore a11y_click_events_have_key_events -->
									<!-- svelte-ignore a11y_no_static_element_interactions -->
									<div
										role="button"
										tabindex="0"
										onclick={() => onSelectHistory(item.originalIndex)}
										onkeydown={(e) => {
											if (e.key === 'Enter' || e.key === ' ') onSelectHistory(item.originalIndex);
										}}
										class={`w-full text-left p-2 rounded-lg cursor-pointer transition min-w-0 ${selectedHistoryIndex === item.originalIndex ? 'bg-gray-100 text-gray-900' : 'bg-white text-gray-700 hover:bg-gray-50'}`}
									>
										<div class="flex items-center justify-between gap-2 mb-1">
											<span class={`inline-flex items-center gap-1 text-[10px] font-medium px-1.5 py-0.5 rounded-full border ${item.success ? 'bg-emerald-50 text-emerald-700 border-emerald-200' : 'bg-red-50 text-red-700 border-red-200'}`}>
												{#if item.success}
													<Check size={10} /> {item.durationMs}ms
												{:else}
													<X size={10} /> Error
												{/if}
											</span>
											<div class="flex items-center gap-1">
												<span class="text-[10px] text-gray-500">{item.time}</span>
												<button
													onclick={(e) => {
														e.stopPropagation();
														handleDeleteWithConfirm(item.originalIndex);
													}}
													class="opacity-0 group-hover/history:opacity-100 w-5 h-5 rounded flex items-center justify-center text-gray-400 hover:text-red-600 hover:bg-red-50 transition"
													aria-label="Delete"
												>
													<Trash2 size={11} />
												</button>
											</div>
										</div>
										<div class="truncate font-mono text-xs leading-relaxed text-gray-800">{item.sql}</div>
									</div>
								</div>
							{/each}
							{#if filteredHistory.length === 0}
								<div class="py-4 text-center text-xs text-gray-500">No matches for "{historySearch}"</div>
							{/if}
						{/if}
					</div>
				</div>
			{/if}
		</div>
	</aside>

	<div class="flex-1 min-w-0 flex flex-col">
		{#if mainView === 'saved_queries'}
			<div class="flex-1 overflow-auto p-4 bg-gray-50">
				{#if selectedSavedQuery}
					<div class="rounded-lg border border-gray-200 bg-white overflow-hidden">
						<div class="px-3 py-2 border-b border-gray-100 flex items-center justify-between">
							<div class="flex items-center gap-2 min-w-0">
								<Star size={12} class="text-amber-500 shrink-0" />
								<span class="text-sm font-medium text-gray-900 truncate">{selectedSavedQuery.title}</span>
							</div>
							<div class="flex items-center gap-1.5">
								<button
									onclick={async () => {
										if (selectedSavedQuery) await navigator.clipboard.writeText(selectedSavedQuery.sql);
									}}
									class="h-7 px-2.5 rounded-md border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-50"
								>
									Copy
								</button>
								<button
									onclick={() => {
										if (selectedSavedQuery) onOpenSavedQuery(selectedSavedQuery.sql);
									}}
									class="h-7 px-2.5 rounded-md bg-white border border-gray-200 text-xs text-gray-700 hover:bg-gray-50"
								>
									Open
								</button>
								<button
									onclick={() => {
										if (selectedSavedQuery) handleDeleteSavedWithConfirm(selectedSavedQuery.id, selectedSavedQuery.title);
									}}
									class="h-7 px-2.5 rounded-md border border-gray-200 bg-white text-xs text-gray-700 hover:bg-red-50 hover:text-red-600 hover:border-red-200"
								>
									Remove
								</button>
							</div>
						</div>
						<div class="p-3">
							<pre class="font-mono text-xs leading-6 text-gray-800 whitespace-pre-wrap">{selectedSavedQuery.sql}</pre>
						</div>
					</div>
				{:else}
					<div class="h-full flex flex-col items-center justify-center text-center p-6">
						<Star size={20} class="text-gray-300 mb-2" />
						<div class="text-xs text-gray-500">No saved query selected</div>
						<div class="text-[11px] text-gray-400 mt-1">Select a saved query to preview</div>
					</div>
				{/if}
			</div>
		{:else if mainView === 'history'}
			<div class="flex-1 overflow-auto p-4 bg-gray-50">
				{#if selectedHistoryQuery}
					<div class="rounded-lg border border-gray-200 bg-white overflow-hidden">
						<div class="px-3 py-2 border-b border-gray-100 flex items-center justify-between">
							<div class="flex items-center gap-2 text-xs">
								<span class="text-gray-500">{selectedHistoryQuery.time}</span>
								<span class={`inline-flex items-center gap-1 text-[10px] font-medium px-1.5 py-0.5 rounded-full border ${selectedHistoryQuery.success ? 'bg-emerald-50 text-emerald-700 border-emerald-200' : 'bg-red-50 text-red-700 border-red-200'}`}>
									{#if selectedHistoryQuery.success}
										<Check size={10} /> {selectedHistoryQuery.durationMs}ms
									{:else}
										<X size={10} /> Error
									{/if}
								</span>
							</div>
							<div class="flex items-center gap-1.5">
								<button
									onclick={async () => {
										if (selectedHistoryQuery) await navigator.clipboard.writeText(selectedHistoryQuery.sql);
									}}
									class="h-7 px-2.5 rounded-md border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-50"
								>
									Copy
								</button>
								<button
									onclick={() => {
										if (selectedHistoryQuery) onOpenSavedQuery(selectedHistoryQuery.sql);
									}}
									class="h-7 px-2.5 rounded-md bg-white border border-gray-200 text-xs text-gray-700 hover:bg-gray-50"
								>
									Open
								</button>
							</div>
						</div>
						<div class="p-3">
							<pre class="font-mono text-xs leading-6 text-gray-800 whitespace-pre-wrap">{selectedHistoryQuery.sql}</pre>
							{#if selectedHistoryQuery.error}
								<div class="mt-2 rounded-md bg-red-50 border border-red-200 px-2.5 py-1.5 text-xs text-red-700">{selectedHistoryQuery.error}</div>
							{/if}
						</div>
					</div>
				{:else}
					<div class="h-full flex flex-col items-center justify-center text-center p-6">
						<Clock size={20} class="text-gray-300 mb-2" />
						<div class="text-xs text-gray-500">Select a query to preview</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<ConfirmDialog
		open={!!confirmDialog?.open}
		title={confirmDialog?.title ?? ''}
		message={confirmDialog?.message ?? ''}
		confirmLabel={confirmDialog?.confirmLabel ?? 'Confirm'}
		variant="danger"
		onConfirm={() => confirmDialog?.onConfirm()}
		onCancel={() => (confirmDialog = null)}
	/>
</section>
