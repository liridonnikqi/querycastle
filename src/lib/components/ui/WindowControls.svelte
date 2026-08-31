<script lang="ts">
	import { onMount } from 'svelte';
	import { Maximize2, Minimize2, Minus, X } from '@lucide/svelte';
	import { isTauri } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';

	const appWindow = getCurrentWindow();
	let maximized = $state(false);
	let desktopWindowControls = $state(false);
	let mounted = false;

	async function refreshMaximizedState() {
		if (!mounted || !desktopWindowControls) return;
		maximized = await appWindow.isMaximized();
	}

	async function handleMinimize() {
		if (!mounted || !desktopWindowControls) return;
		try {
			await appWindow.minimize();
		} catch {
			// ignore
		}
	}

	async function handleToggleMaximize() {
		if (!mounted || !desktopWindowControls) return;
		try {
			await appWindow.toggleMaximize();
		} catch {
			// ignore
		}
		await refreshMaximizedState();
	}

	async function handleClose() {
		if (!mounted || !desktopWindowControls) return;
		try {
			await appWindow.close();
		} catch {
			// ignore
		}
	}

	onMount(() => {
		mounted = true;
		desktopWindowControls = isTauri();
		if (!desktopWindowControls) {
			return () => {
				mounted = false;
			};
		}

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

{#if desktopWindowControls}
	<div class="flex items-center" data-tauri-drag-region="false">
		<button
			type="button"
			aria-label="Minimize window"
			class="w-8 h-7 rounded-md flex items-center justify-center text-qc-muted hover:bg-qc-hover hover:text-qc-fg"
			onmousedown={(event) => event.stopPropagation()}
			onclick={handleMinimize}
		>
			<Minus size={14} />
		</button>
		<button
			type="button"
			aria-label={maximized ? 'Restore window' : 'Maximize window'}
			class="w-8 h-7 rounded-md flex items-center justify-center text-qc-muted hover:bg-qc-hover hover:text-qc-fg"
			onmousedown={(event) => event.stopPropagation()}
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
			class="w-8 h-7 rounded-md flex items-center justify-center text-qc-muted hover:bg-qc-danger hover:text-white"
			onmousedown={(event) => event.stopPropagation()}
			onclick={handleClose}
		>
			<X size={14} />
		</button>
	</div>
{/if}
