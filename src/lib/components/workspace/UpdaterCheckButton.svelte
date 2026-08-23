<script lang="ts">
	import { isTauri } from '@tauri-apps/api/core';
	import { check } from '@tauri-apps/plugin-updater';
	import { Download, Loader2 } from '@lucide/svelte';
	import { downloadAndInstallUpdate } from '$lib/updater';
	import { showSnackbar, dismissSnackbar } from '$lib/stores/snackbar';

	let checking = $state(false);
	let isDesktop = $derived(isTauri());

	async function handleManualCheck() {
		if (!isDesktop || checking) return;
		checking = true;
		const loadingId = showSnackbar({
			message: 'Checking for updates...',
			type: 'info',
			duration: 0,
			showClose: false
		});
		try {
			const update = await check();
			dismissSnackbar(loadingId);
			if (update) {
				const id = showSnackbar({
					message: `Update v${update.version} available`,
					description: update.body ?? 'A new version is ready to install.',
					type: 'update',
					duration: 0,
					actionLabel: 'Update',
					onAction: async () => {
						dismissSnackbar(id);
						await handleInstall(update);
					}
				});
			} else {
				showSnackbar({ message: 'You are up to date', type: 'success' });
			}
		} catch (e) {
			dismissSnackbar(loadingId);
			const msg = e instanceof Error ? e.message : String(e);
			showSnackbar({
				message: 'Failed to check for updates',
				description: msg,
				type: 'error',
				actionLabel: 'Retry',
				onAction: () => void handleManualCheck()
			});
		} finally {
			checking = false;
		}
	}

	async function handleInstall(update: Awaited<ReturnType<typeof check>>) {
		if (!update) return;
		const dlId = showSnackbar({
			message: `Downloading v${update.version}...`,
			type: 'update',
			duration: 0,
			showClose: false
		});
		try {
			await downloadAndInstallUpdate(update, (p) => {
				const pct = p.total ? ` ${Math.round((p.downloaded / p.total) * 100)}%` : '';
				dismissSnackbar(dlId);
				// recreate with progress (simplified)
				showSnackbar({
					message: `Downloading v${update.version}...${pct}`,
					type: 'update',
					duration: 0,
					showClose: false
				});
			});
			dismissSnackbar(dlId);
			showSnackbar({
				message: 'Update ready — restart to apply',
				description: `v${update.version} installed`,
				type: 'success',
				duration: 0,
				actionLabel: 'Restart',
				onAction: async () => {
					try {
						const { relaunch } = await import('@tauri-apps/plugin-process');
						await relaunch();
					} catch {}
				}
			});
		} catch (e) {
			dismissSnackbar(dlId);
			const msg = e instanceof Error ? e.message : String(e);
			showSnackbar({
				message: 'Update failed',
				description: msg,
				type: 'error',
				actionLabel: 'Retry',
				onAction: () => void handleInstall(update)
			});
		}
	}
</script>

{#if isDesktop}
	<button
		onclick={handleManualCheck}
		disabled={checking}
		class="h-7 px-2.5 rounded border border-white/15 text-xs text-gray-300 hover:text-white hover:bg-white/10 inline-flex items-center gap-1.5 disabled:opacity-50"
		title="Check for updates"
	>
		{#if checking}
			<Loader2 size={12} class="animate-spin" /> Checking...
		{:else}
			<Download size={12} /> Check for updates
		{/if}
	</button>
{/if}
