<script lang="ts">
	import { ChevronLeft, ChevronRight, FileCode2, GitFork, Plus, ScrollText, Table2, Terminal, X } from '@lucide/svelte';
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

	let scroller = $state<HTMLDivElement | null>(null);
	let canScrollLeft = $state(false);
	let canScrollRight = $state(false);

	function tabIcon(tab: WorkspaceTab) {
		if (tab.kind === 'diagram') return GitFork;
		if (tab.kind === 'data') return Table2;
		if (tab.title.toLowerCase().includes('diagram')) return GitFork;
		if (tab.title.toLowerCase().includes('log')) return ScrollText;
		if (tab.kind === 'query') return Terminal;
		return FileCode2;
	}

	function updateOverflow() {
		const container = scroller;
		if (!container) {
			canScrollLeft = false;
			canScrollRight = false;
			return;
		}
		const max = container.scrollWidth - container.clientWidth;
		canScrollLeft = container.scrollLeft > 2;
		canScrollRight = max > 2 && container.scrollLeft < max - 2;
	}

	function scrollTabs(direction: -1 | 1) {
		const container = scroller;
		if (!container) return;
		const delta = Math.max(160, Math.round(container.clientWidth * 0.55));
		container.scrollBy({ left: direction * delta, behavior: 'smooth' });
	}

	function scrollTabIntoView(tabId: string) {
		const container = scroller;
		if (!container) return;
		const node = container.querySelector(`[data-tab-id="${CSS.escape(tabId)}"]`);
		if (!(node instanceof HTMLElement)) return;
		const pad = 8;
		const left = node.offsetLeft;
		const right = left + node.offsetWidth;
		const viewLeft = container.scrollLeft;
		const viewRight = viewLeft + container.clientWidth;
		if (right > viewRight) {
			container.scrollLeft = right - container.clientWidth + pad;
		} else if (left < viewLeft) {
			container.scrollLeft = Math.max(0, left - pad);
		}
		updateOverflow();
	}

	function selectTab(tabId: string) {
		onSelectTab(tabId);
		requestAnimationFrame(() => scrollTabIntoView(tabId));
	}

	function onWheel(event: WheelEvent) {
		const container = scroller;
		if (!container) return;
		if (container.scrollWidth <= container.clientWidth) return;
		if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
		event.preventDefault();
		container.scrollLeft += event.deltaY;
	}

	$effect(() => {
		const id = activeTabId;
		requestAnimationFrame(() => scrollTabIntoView(id));
	});

	$effect(() => {
		const container = scroller;
		tabs;
		if (!container) return;
		const onScroll = () => updateOverflow();
		container.addEventListener('scroll', onScroll, { passive: true });
		const observer = new ResizeObserver(() => updateOverflow());
		observer.observe(container);
		requestAnimationFrame(updateOverflow);
		return () => {
			container.removeEventListener('scroll', onScroll);
			observer.disconnect();
		};
	});
</script>

<div class="tab-strip grid h-9 grid-cols-[minmax(0,max-content)_2rem] items-center bg-qc-panel shrink-0 min-w-0 w-full">
	<div class="relative min-w-0 h-full">
		<div
			bind:this={scroller}
			onwheel={onWheel}
			class="h-full min-w-0 max-w-full overflow-x-auto overflow-y-hidden hide-scrollbar"
		>
			<div class="flex items-stretch h-full w-max">
				{#each tabs as tab (tab.id)}
					{@const Icon = tabIcon(tab)}
					<div
						data-tab-id={tab.id}
						class={`tab-item group flex items-center gap-1.5 px-3 text-[12px] whitespace-nowrap h-full shrink-0 ${tab.id === activeTabId ? 'active text-qc-fg' : 'text-qc-muted hover:text-qc-subtle hover:bg-qc-hover/60'}`}
					>
						<button
							onclick={() => selectTab(tab.id)}
							oncontextmenu={(event) => onOpenContextMenu(event, tab.id)}
							class="inline-flex items-center gap-1.5 min-w-0"
						>
							<Icon size={12} class="shrink-0" />
							<span class="truncate max-w-40">{tab.title}</span>
						</button>
						<button
							onclick={(e) => {
								e.stopPropagation();
								onCloseTab(tab.id);
							}}
							class="w-3.5 h-3.5 opacity-50 hover:opacity-100"
							aria-label={`Close ${tab.title}`}
						>
							<X size={12} />
						</button>
					</div>
				{/each}
			</div>
		</div>
		{#if canScrollLeft}
			<button
				type="button"
				class="tab-overflow-btn left-0"
				onclick={() => scrollTabs(-1)}
				aria-label="Scroll tabs left"
				title="More tabs"
			>
				<ChevronLeft size={14} />
			</button>
		{/if}
		{#if canScrollRight}
			<button
				type="button"
				class="tab-overflow-btn right-0"
				onclick={() => scrollTabs(1)}
				aria-label="Scroll tabs right"
				title="More tabs"
			>
				<ChevronRight size={14} />
			</button>
		{/if}
	</div>
	<button
		onclick={onAddTab}
		class="w-8 h-full flex items-center justify-center text-qc-muted hover:text-qc-subtle hover:bg-qc-hover/60 shrink-0"
		title="New tab (Ctrl+N)"
		aria-label="New query tab"
	>
		<Plus size={14} />
	</button>
</div>

{#if tabContextMenu}
	<button class="fixed inset-0 z-40" aria-label="Close tab menu" onclick={onCloseContextMenu}></button>
	<div
		class="fixed z-50 min-w-[180px] bg-qc-elevated rounded-md border border-qc-border shadow-[0_8px_24px_rgba(0,0,0,0.24)] py-1"
		style={`left:${tabContextMenu.x}px;top:${tabContextMenu.y}px;`}
	>
		<button onclick={onCloseAllTabs} class="w-full px-3 py-1.5 text-left text-[12px] text-qc-fg hover:bg-qc-hover">
			Close all
		</button>
		<button
			onclick={() => onCloseAllTabsBut(tabContextMenu?.tabId ?? '')}
			class="w-full px-3 py-1.5 text-left text-[12px] text-qc-fg hover:bg-qc-hover"
		>
			Close all but this
		</button>
	</div>
{/if}
