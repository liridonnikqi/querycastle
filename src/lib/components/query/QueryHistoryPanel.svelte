<script lang="ts">
	import { Clock3, Keyboard, Star, Trash2, X } from "@lucide/svelte";
	import type { QueryHistoryItem, SavedQueryItem } from "$lib/types";

	let {
		items,
		favorites,
		currentConnectionKey,
		onPickSql,
		onDeleteFavorite,
		tabs,
		activeTabId,
		onSetActiveTab,
		onCloseTab,
	}: {
		items: QueryHistoryItem[];
		favorites: SavedQueryItem[];
		currentConnectionKey: string;
		onPickSql: (sql: string) => void;
		onDeleteFavorite: (id: string) => void;
		tabs: Array<{ id: string; title: string }>;
		activeTabId: string;
		onSetActiveTab: (tabId: string) => void;
		onCloseTab: (tabId: string) => void;
	} = $props();
</script>

<aside class="w-80 border-l border-[var(--border-soft)] bg-[var(--surface-1)] flex flex-col shrink-0">
	<div class="h-10 border-b border-[var(--border-soft)] flex items-center px-2 shrink-0 bg-[var(--surface-0)] gap-1 overflow-x-auto">
		{#each tabs as tab}
			<div class={`h-7 px-2 rounded-md border text-xs flex items-center gap-2 ${tab.id === activeTabId ? "bg-[#eaf2ff] border-[#bfd4f7] text-[var(--title)]" : "bg-[var(--surface-0)] border-[var(--border-soft)] text-[var(--text-muted)]"}`}>
				<button onclick={() => onSetActiveTab(tab.id)} class="truncate">{tab.title}</button>
				<button onclick={() => onCloseTab(tab.id)} class="text-[#8a90a1] hover:text-[var(--title)]" aria-label={`Close ${tab.title}`}><X size={12} /></button>
			</div>
		{/each}
	</div>

	<div class="flex-1 overflow-y-auto p-3 space-y-2">
		{#if activeTabId === "shortcuts"}
			<div class="rounded-lg border border-[var(--border-soft)] bg-[var(--surface-0)] p-3 text-xs text-[var(--text-muted)] space-y-2">
				<div class="flex items-center gap-2 text-[var(--title)] font-medium"><Keyboard size={14} /> Keyboard Shortcuts</div>
				<div class="flex items-center justify-between"><span>Quick Search</span><span>Ctrl P</span></div>
				<div class="flex items-center justify-between"><span>Run Query</span><span>Ctrl Enter</span></div>
				<div class="flex items-center justify-between"><span>Format SQL</span><span>Ctrl Shift F</span></div>
				<div class="flex items-center justify-between"><span>Save Query</span><span>Ctrl S</span></div>
				<div class="flex items-center justify-between"><span>Find / Replace</span><span>Ctrl F</span></div>
				<div class="flex items-center justify-between"><span>New Tab</span><span>Ctrl T</span></div>
			</div>
		{:else if activeTabId === "saved"}
			<div class="rounded-lg border border-[var(--border-soft)] bg-[var(--surface-0)] p-2 text-[11px] text-[var(--text-muted)]">
				Scope: {currentConnectionKey}
			</div>
			{#if favorites.length === 0}
				<div class="text-xs text-[#8c93a4]">No saved queries for this connection.</div>
			{:else}
				{#each favorites as item}
					<div class="p-3 rounded-lg border border-[var(--border-soft)] bg-[var(--surface-0)] hover:border-[#b9cae6] transition-colors group">
						<div class="flex items-center justify-between mb-2">
							<div class="inline-flex items-center gap-1.5 text-xs text-[#687085]"><Star size={12} />Saved</div>
							<button onclick={() => onDeleteFavorite(item.id)} class="text-[#8a90a1] hover:text-[var(--danger)]" aria-label="Delete saved query"><Trash2 size={12} /></button>
						</div>
						<div class="text-xs font-medium text-[var(--title)] mb-1 line-clamp-1">{item.title}</div>
						<button onclick={() => onPickSql(item.sql)} class="font-mono-code text-xs text-[#4f5f76] line-clamp-2 leading-relaxed group-hover:text-[var(--title)] transition-colors text-left w-full">{item.sql}</button>
					</div>
				{/each}
			{/if}
		{:else if items.length === 0}
			<div class="text-xs text-[#8c93a4]">No queries yet.</div>
		{:else}
			{#each items as item}
				<div class={`p-3 rounded-lg border transition-colors cursor-pointer group ${item.success ? "bg-[var(--surface-0)] border-[var(--border-soft)] hover:border-[#b9cae6]" : "bg-[#fff5f4] border-[#f2c7c3] hover:border-[#e7afa9]"}`}>
					<div class="flex justify-between items-start mb-2">
						<div class="flex items-center gap-1.5 text-xs text-[#71788a]">
							<Clock3 size={13} />
							{item.time}
						</div>
						<div class="text-xs font-mono-code text-[var(--title)] bg-[var(--surface-2)] px-1.5 py-0.5 rounded border border-[var(--border-soft)]">{item.durationMs}ms</div>
					</div>
					<button onclick={() => onPickSql(item.sql)} class="font-mono-code text-xs text-[#4f5f76] line-clamp-2 leading-relaxed group-hover:text-[var(--title)] transition-colors text-left w-full">{item.sql}</button>
					{#if item.error}
						<div class="mt-2 text-xs text-[var(--danger)]">{item.error}</div>
					{/if}
				</div>
			{/each}
		{/if}
	</div>
</aside>




