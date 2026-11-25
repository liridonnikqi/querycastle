<script lang="ts">
	import { LogOut, Search } from '$lib/icons';
	import { isTauri } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import type { ConnectionStatus } from '$lib/rpc';
	import ThemeToggle from '$lib/components/ui/ThemeToggle.svelte';
	import WindowControls from '$lib/components/ui/WindowControls.svelte';
	import QueryCastleLogo from '$lib/components/ui/QueryCastleLogo.svelte';
	import { theme } from '$lib/theme.svelte';
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
	class="h-10 grid grid-cols-[minmax(0,1fr)_minmax(180px,min(20rem,30vw))_minmax(0,1fr)] items-center border-b border-qc-border bg-qc-panel shrink-0 px-2 gap-2"
	ondblclick={handleHeaderDoubleClick}
>
	<div class="flex items-center gap-1.5 min-w-0 overflow-hidden pl-1" data-tauri-drag-region>
		<QueryCastleLogo
			size={16}
			class={theme.value === 'light' ? 'text-black' : 'text-white'}
			alt=""
		/>
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
			class="w-full h-7 flex items-center gap-2 rounded-md bg-qc-bg/75 hover:bg-qc-bg border border-qc-border/80 hover:border-qc-conn-hover-border px-2.5 text-qc-muted hover:text-qc-subtle transition-colors"
			aria-label="Search or run commands"
		>
			<Search size={13} class="shrink-0 opacity-75" />
			<span class="text-[12px] flex-1 text-left truncate">Search…</span>
			<span class="text-[10px] font-mono text-qc-muted/75 shrink-0">
				{isMac ? '⌘K' : 'Ctrl K'}
			</span>
		</button>
	</div>

	<div class="flex items-center justify-end gap-0.5 min-w-0 shrink-0" data-tauri-drag-region="false">
		{#if gridChrome.changeCount > 0}
			<button
				type="button"
				onclick={requestOpenPendingChanges}
				onmousedown={(event) => event.stopPropagation()}
				class="changes-btn shrink-0"
				data-tip="Review pending changes"
			>
				<span class="changes-btn-inner">
					Changes
					<span class="changes-btn-count">{gridChrome.changeCount}</span>
				</span>
			</button>
		{/if}
		<ThemeToggle />
		{#if connectionStatus.connected}
			<div class="w-px h-4 bg-qc-border mx-1 shrink-0"></div>
			<button
				type="button"
				onclick={onDisconnect}
				onmousedown={(event) => event.stopPropagation()}
				class="w-7 h-7 rounded-md flex items-center justify-center text-qc-muted hover:bg-qc-hover hover:text-qc-subtle shrink-0"
				data-tip="Disconnect"
				aria-label="Disconnect"
			>
				<LogOut size={14} />
			</button>
		{/if}
		<div class="w-px h-4 bg-qc-border mx-1 shrink-0"></div>
		<WindowControls />
	</div>
</header>
