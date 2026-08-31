<script lang="ts">
	import { LogOut, Search } from '@lucide/svelte';
	import { isTauri } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import type { ConnectionStatus } from '$lib/rpc';
	import ThemeToggle from '$lib/components/ui/ThemeToggle.svelte';
	import WindowControls from '$lib/components/ui/WindowControls.svelte';
	import QueryCastleLogo from '$lib/components/ui/QueryCastleLogo.svelte';
	import { gridChrome, requestOpenPendingChanges } from '$lib/stores/grid-chrome.svelte';
	import type { Snippet } from 'svelte';

	let {
		connectionStatus,
		onDisconnect,
		onOpenSearch,
		leading,
	}: {
		connectionStatus: ConnectionStatus;
		onDisconnect: () => void;
		onOpenSearch: () => void;
		leading?: Snippet;
	} = $props();

	const isMac = $derived(
		typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform),
	);

	async function handleHeaderDoubleClick() {
		if (!isTauri()) return;
		try {
			await getCurrentWindow().toggleMaximize();
		} catch {
			// ignore
		}
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header
	data-tauri-drag-region
	class="h-10 grid grid-cols-[minmax(0,1fr)_minmax(200px,28rem)_minmax(0,1fr)] items-center border-b border-qc-border bg-qc-panel shrink-0 px-2 gap-2"
	ondblclick={handleHeaderDoubleClick}
>
	<div class="flex items-center gap-1.5 min-w-0 overflow-hidden pl-1" data-tauri-drag-region>
		<QueryCastleLogo size={16} class="text-[#2563eb]" alt="" />
		{#if leading}
			<div class="min-w-0 max-w-full overflow-hidden" data-tauri-drag-region="false">
				{@render leading()}
			</div>
		{/if}
	</div>

	<div class="min-w-0" data-tauri-drag-region>
		<button
			type="button"
			onclick={onOpenSearch}
			onmousedown={(event) => event.stopPropagation()}
			data-tauri-drag-region="false"
			class="w-full max-w-md h-7 flex items-center gap-2 rounded-md border border-qc-border bg-qc-bg px-2.5 text-qc-muted hover:border-qc-muted"
		>
			<Search size={14} class="shrink-0" />
			<span class="text-[12px] flex-1 text-left truncate">Search or run commands...</span>
			<kbd class="text-[10px] text-qc-muted bg-qc-elevated border border-qc-border rounded px-1 py-0.5 font-mono">
				{isMac ? '⌘K' : 'Ctrl+K'}
			</kbd>
		</button>
	</div>

	<div class="flex items-center justify-end gap-0.5 min-w-0" data-tauri-drag-region="false">
		{#if gridChrome.changeCount > 0}
			<button
				type="button"
				onclick={requestOpenPendingChanges}
				onmousedown={(event) => event.stopPropagation()}
				class="changes-btn"
				title="Review pending changes"
			>
				<span class="changes-btn-inner">
					Changes
					<span class="changes-btn-count">{gridChrome.changeCount}</span>
				</span>
			</button>
		{/if}
		<ThemeToggle />
		{#if connectionStatus.connected}
			<div class="w-px h-4 bg-qc-border mx-1"></div>
			<button
				type="button"
				onclick={onDisconnect}
				onmousedown={(event) => event.stopPropagation()}
				class="w-7 h-7 rounded-md flex items-center justify-center text-qc-muted hover:bg-qc-hover hover:text-qc-subtle"
				title="Disconnect"
			>
				<LogOut size={14} />
			</button>
		{/if}
		<div class="w-px h-4 bg-qc-border mx-1"></div>
		<WindowControls />
	</div>
</header>
