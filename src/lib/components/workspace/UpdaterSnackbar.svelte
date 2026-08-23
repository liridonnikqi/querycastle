<script lang="ts">
	import { onMount } from 'svelte';
	import { isTauri } from '@tauri-apps/api/core';
	import { check } from '@tauri-apps/plugin-updater';
	import { downloadAndInstallUpdate } from '$lib/updater';
	import { showSnackbar, dismissSnackbar } from '$lib/stores/snackbar';

	let isDesktop = $state(false);
	let pendingUpdate: Awaited<ReturnType<typeof check>> = null;
	let activeSnackbarId: string | null = null;
	let downloadingId: string | null = null;

	onMount(() => {
		isDesktop = isTauri();
		if (!isDesktop) return;
		const t = setTimeout(() => void silentCheck(), 3000);
		return () => clearTimeout(t);
	});

	async function silentCheck() {
		if (!isDesktop) return;
		try {
			const update = await check();
			if (update) {
				pendingUpdate = update;
				showUpdateAvailable(update.version, update.body);
			}
		} catch {
			// silent fail - will show on manual check
		}
	}

	function showUpdateAvailable(version: string, body?: string | null) {
		if (activeSnackbarId) dismissSnackbar(activeSnackbarId);
		activeSnackbarId = showSnackbar({
			message: `Update v${version} available`,
			description: body ?? 'A new version of QueryCastle is ready to install.',
			type: 'update',
			duration: 0,
			actionLabel: 'Update',
			onAction: () => void handleInstall(),
			showClose: true
		});
	}

	async function handleInstall() {
		if (!pendingUpdate) return;
		if (activeSnackbarId) dismissSnackbar(activeSnackbarId);
		activeSnackbarId = null;

		downloadingId = showSnackbar({
			message: `Downloading v${pendingUpdate.version}...`,
			description: 'Please keep the app open',
			type: 'update',
			duration: 0,
			showClose: false
		});

		try {
			await downloadAndInstallUpdate(pendingUpdate, (p) => {
				if (downloadingId) {
					const pct = p.total ? ` ${Math.round((p.downloaded / p.total) * 100)}%` : '';
					// update snackbar message with progress (re-create for simplicity)
					dismissSnackbar(downloadingId);
					downloadingId = showSnackbar({
						message: `Downloading v${pendingUpdate!.version}...${pct}`,
						description: p.total
							? `${(p.downloaded / 1024 / 1024).toFixed(1)} / ${(p.total / 1024 / 1024).toFixed(1)} MB`
							: 'Please keep the app open',
						type: 'update',
						duration: 0,
						showClose: false
					});
				}
			});
			if (downloadingId) dismissSnackbar(downloadingId);
			downloadingId = null;
			showSnackbar({
				message: 'Update ready — restart to apply',
				description: `v${pendingUpdate.version} has been installed.`,
				type: 'success',
				duration: 0,
				actionLabel: 'Restart',
				onAction: async () => {
					try {
						const { relaunch } = await import('@tauri-apps/plugin-process');
						await relaunch();
					} catch {
						// fallback: just inform
					}
				}
			});
			pendingUpdate = null;
		} catch (e) {
			if (downloadingId) dismissSnackbar(downloadingId);
			downloadingId = null;
			const msg = e instanceof Error ? e.message : String(e);
			showSnackbar({
				message: 'Update failed',
				description: msg,
				type: 'error',
				duration: 0,
				actionLabel: 'Retry',
				onAction: () => void handleInstall()
			});
		}
	}

	// Expose manual check for anywhere in app (e.g., header menu)
	export async function checkForUpdatesManual() {
		if (!isDesktop) {
			showSnackbar({ message: 'Updates only available in desktop app', type: 'info' });
			return;
		}
		const id = showSnackbar({
			message: 'Checking for updates...',
			type: 'info',
			duration: 0,
			showClose: false
		});
		try {
			const update = await check();
			dismissSnackbar(id);
			if (update) {
				pendingUpdate = update;
				showUpdateAvailable(update.version, update.body);
			} else {
				showSnackbar({ message: 'You are up to date', description: 'No updates available', type: 'success' });
			}
		} catch (e) {
			dismissSnackbar(id);
			const msg = e instanceof Error ? e.message : String(e);
			showSnackbar({
				message: 'Failed to check for updates',
				description: msg,
				type: 'error',
				actionLabel: 'Retry',
				onAction: () => void checkForUpdatesManual()
			});
		}
	}
</script>

<!-- This component is invisible - it only manages snackbars -->
