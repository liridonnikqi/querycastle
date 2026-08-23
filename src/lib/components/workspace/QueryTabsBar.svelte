<script lang="ts">
	import { FileCode2, Plus } from '@lucide/svelte';
	import type { TabContextMenu, WorkspaceTab } from '$lib/utils/workspace';

	let {
		tabs,
		activeTabId,
		tabContextMenu,
		onSelectTab,
		onOpenContextMenu,
		onCloseTab,
		onAddTab,
		onCloseContextMenu,
		onCloseAllTabs,
		onCloseAllTabsBut,
	}: {
		tabs: WorkspaceTab[];
		activeTabId: string;
		tabContextMenu: TabContextMenu;
		onSelectTab: (tabId: string) => void;
		onOpenContextMenu: (event: MouseEvent, tabId: string) => void;
		onCloseTab: (tabId: string) => void;
		onAddTab: () => void;
		onCloseContextMenu: () => void;
		onCloseAllTabs: () => void;
		onCloseAllTabsBut: (tabId: string) => void;
	} = $props();
</script>

<div class="flex items-center h-9 bg-gray-50/80 border-b border-gray-200 overflow-x-auto hide-scrollbar shrink-0">
	{#each tabs as tab}
		<div
			class={`flex items-center min-w-0 max-w-72 px-4 py-2 border-r border-gray-200 border-t-2 text-sm font-medium relative z-10 -mb-[1px] ${tab.id === activeTabId ? 'bg-white border-t-emerald-500 text-gray-800' : 'border-t-transparent text-gray-500 hover:bg-gray-100/50 hover:text-gray-700'}`}
		>
			<button
				onclick={() => onSelectTab(tab.id)}
				oncontextmenu={(event) => onOpenContextMenu(event, tab.id)}
				class="inline-flex items-center space-x-2 min-w-0 flex-1"
			>
				<FileCode2
					size={16}
					class={`shrink-0 ${tab.id === activeTabId ? 'text-emerald-500' : ''}`}
				/>
				<span class="truncate">{tab.title}</span>
			</button>
			<button
				onclick={() => onCloseTab(tab.id)}
				class="text-gray-400 hover:text-gray-600 ml-2 shrink-0"
				aria-label={`Close ${tab.title}`}>x</button
			>
		</div>
	{/each}
	<button
		onclick={onAddTab}
		class="w-7 h-7 ml-1.5 flex items-center justify-center text-gray-400 hover:text-gray-900 transition-colors shrink-0"
		title="New tab (Ctrl+N)"
		aria-label="New query tab"
	>
		<Plus size={14} />
	</button>
</div>

{#if tabContextMenu}
	<button
		class="fixed inset-0 z-40"
		aria-label="Close tab menu"
		onclick={onCloseContextMenu}
	></button>
	<div
		class="fixed z-50 min-w-[180px] bg-white rounded-md border border-gray-200 shadow-[0_8px_24px_rgba(0,0,0,0.12)] py-1"
		style={`left:${tabContextMenu.x}px;top:${tabContextMenu.y}px;`}
	>
		<button
			onclick={onCloseAllTabs}
			class="w-full px-3 py-1.5 text-left text-sm text-gray-700 hover:bg-gray-50"
		>
			Close all
		</button>
		<button
			onclick={() => onCloseAllTabsBut(tabContextMenu?.tabId ?? '')}
			class="w-full px-3 py-1.5 text-left text-sm text-gray-700 hover:bg-gray-50"
		>
			Close all but this
		</button>
	</div>
{/if}
