<script lang="ts">
	import { onMount } from 'svelte';
import { ChevronRight, Maximize2, Minimize2, Minus, Plus, Unplug, X } from '@lucide/svelte';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { ConnectionStatus } from '$lib/rpc';

	let {
		connectionStatus,
		onCreateConnection,
		onDisconnect,
	}: {
		connectionStatus: ConnectionStatus;
		onCreateConnection: () => void;
		onDisconnect: () => void;
	} = $props();

	const appWindow = getCurrentWindow();
	let maximized = $state(false);
	let desktopWindowControls = $state(false);
	let mounted = false;

	function isPrimaryMouse(event: MouseEvent) {
		return event.button === 0;
	}

	async function startWindowDrag(event: MouseEvent) {
		if (!mounted || !desktopWindowControls || !isPrimaryMouse(event)) return;
		try {
			await appWindow.startDragging();
		} catch {
			// Ignore blocked/unsupported drag attempts.
		}
	}

	async function handleHeaderDoubleClick() {
		if (!mounted || !desktopWindowControls) return;
		await handleToggleMaximize();
	}

	async function refreshMaximizedState() {
		if (!mounted || !desktopWindowControls) return;
		maximized = await appWindow.isMaximized();
	}

	async function handleMinimize() {
		if (!mounted || !desktopWindowControls) return;
		try {
			await appWindow.minimize();
		} catch {
			// Ignore blocked/unsupported minimize attempts.
		}
	}

	async function handleToggleMaximize() {
		if (!mounted || !desktopWindowControls) return;
		try {
			await appWindow.toggleMaximize();
		} catch {
			// Ignore blocked/unsupported maximize attempts.
		}
		await refreshMaximizedState();
	}

	async function handleClose() {
		if (!mounted || !desktopWindowControls) return;
		try {
			await appWindow.close();
		} catch {
			// Ignore blocked/unsupported close attempts.
		}
	}

	onMount(() => {
		mounted = true;
		desktopWindowControls = isTauri();
		if (!desktopWindowControls) return () => {
			mounted = false;
		};

		let unlistenResize: (() => void) | null = null;
		void (async () => {
			await refreshMaximizedState();
			unlistenResize = await appWindow.onResized(() => {
				void refreshMaximizedState();
			});
		})();

		return () => {
			mounted = false;
			unlistenResize?.();
		};
	});
</script>

<header
	class="flex items-center h-12 bg-[#1c1c1e] text-gray-300 px-3 shrink-0 shadow-sm z-30 relative"
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		data-tauri-drag-region
		class="flex items-center space-x-3 min-w-0 flex-1 h-full cursor-grab active:cursor-grabbing"
		onmousedown={startWindowDrag}
		ondblclick={handleHeaderDoubleClick}
	>
		<img src="/icon.svg?v=2" alt="QueryCastle" class="w-6 h-6 rounded-[5px] object-contain shrink-0" />
		<div class="flex items-center text-sm min-w-0">
			<span class="text-gray-400 cursor-default hover:text-gray-200 truncate">
				{connectionStatus.connected ? connectionStatus.host : 'Disconnected'}
			</span>

			<ChevronRight size={14} class="mx-1 text-gray-600 shrink-0" />

			<span class="font-medium text-gray-100 cursor-default flex items-center truncate">
				{connectionStatus.connected
					? connectionStatus.database || 'No database selected'
					: 'No database selected'}
			</span>
		</div>
	</div>
	<div class="flex items-center gap-2">
		{#if !connectionStatus.connected}
			<button
				onclick={onCreateConnection}
				onmousedown={(event) => event.stopPropagation()}
				ondblclick={(event) => event.stopPropagation()}
				class="h-7 px-2 rounded border border-white/15 text-xs text-gray-300 hover:text-white hover:bg-white/10 inline-flex items-center gap-1"
			>
				<Plus size={12} />
				Connection
			</button>
		{:else}
			<button
				onclick={onDisconnect}
				onmousedown={(event) => event.stopPropagation()}
				ondblclick={(event) => event.stopPropagation()}
				class="h-7 px-2 rounded border border-white/15 text-xs text-gray-300 hover:text-white hover:bg-red-600 hover:border-red-600 inline-flex items-center gap-1"
			>
				<Unplug size={12} />
				Disconnect
			</button>
		{/if}
	</div>
	{#if desktopWindowControls}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			data-tauri-drag-region
			class="h-full w-3 cursor-grab active:cursor-grabbing"
			onmousedown={startWindowDrag}
			ondblclick={handleHeaderDoubleClick}
		></div>
		<div class="flex items-center gap-1 ml-3">
			<button
				type="button"
				aria-label="Minimize window"
				class="w-9 h-8 rounded-md flex items-center justify-center text-gray-300 hover:bg-white/10 hover:text-white"
				onmousedown={(event) => event.stopPropagation()}
				ondblclick={(event) => event.stopPropagation()}
				onclick={handleMinimize}
			>
				<Minus size={14} />
			</button>
			<button
				type="button"
				aria-label={maximized ? 'Restore window' : 'Maximize window'}
				class="w-9 h-8 rounded-md flex items-center justify-center text-gray-300 hover:bg-white/10 hover:text-white"
				onmousedown={(event) => event.stopPropagation()}
				ondblclick={(event) => event.stopPropagation()}
				onclick={handleToggleMaximize}
			>
				{#if maximized}
					<Minimize2 size={13} />
				{:else}
					<Maximize2 size={13} />
				{/if}
			</button>
			<button
				type="button"
				aria-label="Close window"
				class="w-9 h-8 rounded-md flex items-center justify-center text-gray-300 hover:bg-red-600 hover:text-white"
				onmousedown={(event) => event.stopPropagation()}
				ondblclick={(event) => event.stopPropagation()}
				onclick={handleClose}
			>
				<X size={14} />
			</button>
		</div>
	{/if}
</header>
