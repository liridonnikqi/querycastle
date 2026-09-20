<script lang="ts">
	import { onMount } from 'svelte';
	import { isTauri } from '@tauri-apps/api/core';
	import { getVersion } from '@tauri-apps/api/app';
	import { check } from '@tauri-apps/plugin-updater';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { Download, RefreshCw, Check, AlertCircle, Loader2 } from '$lib/icons';
	import GithubIcon from '$lib/components/ui/GithubIcon.svelte';
	import { downloadAndInstallUpdate } from '$lib/updater';
	import { toast } from '$lib/stores/toast.svelte';

	const REPO_URL = 'https://github.com/liridonnikqi/querycastle';

	let version = $state('0.1.1');
	let status = $state<
		'idle' | 'checking' | 'available' | 'downloading' | 'uptodate' | 'error' | 'ready'
	>('idle');
	let errorMsg = $state('');
	let updateVersion = $state('');
	let updateBody = $state('');
	let downloaded = $state(0);
	let total = $state<number | undefined>(undefined);
	let isDesktop = $state(false);
	let pendingUpdate: Awaited<ReturnType<typeof check>> = null;

	onMount(() => {
		isDesktop = isTauri();
		void getVersion()
			.then((v) => (version = v))
			.catch(() => {});
		if (isDesktop) void silentCheck();
	});

	async function silentCheck() {
		if (!isDesktop || status === 'checking' || status === 'downloading') return;
		try {
			const update = await check();
			if (update) {
				pendingUpdate = update;
				updateVersion = update.version;
				updateBody = update.body ?? '';
				status = 'available';
			}
		} catch {
			// silent fail
		}
	}

	async function handleCheck() {
		if (!isDesktop) return;
		if (status === 'checking' || status === 'downloading') return;
		status = 'checking';
		errorMsg = '';
		try {
			const update = await check();
			if (update) {
				pendingUpdate = update;
				updateVersion = update.version;
				updateBody = update.body ?? '';
				status = 'available';
				toast.success(`Update available: v${update.version}`);
			} else {
				status = 'uptodate';
				toast.success('QueryCastle is up to date');
				setTimeout(() => {
					if (status === 'uptodate') status = 'idle';
				}, 3000);
			}
		} catch (e) {
			status = 'error';
			errorMsg = e instanceof Error ? e.message : String(e);
			toast.error(`Update check failed: ${errorMsg}`);
		}
	}

	async function handleInstall() {
		if (!pendingUpdate) return;
		status = 'downloading';
		downloaded = 0;
		total = undefined;
		try {
			await downloadAndInstallUpdate(pendingUpdate, (p) => {
				downloaded = p.downloaded;
				total = p.total;
			});
			status = 'ready';
		} catch (e) {
			status = 'error';
			errorMsg = e instanceof Error ? e.message : String(e);
			toast.error(`Update failed: ${errorMsg}`);
		}
	}

	function dismiss() {
		status = 'idle';
		errorMsg = '';
	}

	function openRepo() {
		if (isDesktop) {
			openUrl(REPO_URL).catch(() => {
				window.open(REPO_URL, '_blank', 'noopener,noreferrer');
			});
		} else {
			window.open(REPO_URL, '_blank', 'noopener,noreferrer');
		}
	}
</script>

<footer
	class="h-8 flex items-center justify-between px-3 bg-qc-panel text-qc-muted text-xs shrink-0 border-t border-qc-border select-none"
>
	<div class="flex items-center gap-2 min-w-0">
		<span>v{version}</span>
		{#if !isDesktop}
			<span>Updates available in desktop app</span>
		{:else if status === 'checking'}
			<span class="inline-flex items-center gap-1">
				<Loader2 size={10} class="animate-spin" /> Checking…
			</span>
		{:else if status === 'available'}
			<span class="text-qc-fg font-medium" data-tip={updateBody}>v{updateVersion} available</span>
		{:else if status === 'downloading'}
			<span class="inline-flex items-center gap-1 text-qc-subtle">
				<Loader2 size={10} class="animate-spin" />
				Downloading{#if total} {Math.round((downloaded / total) * 100)}%{/if}
			</span>
		{:else if status === 'ready'}
			<span class="text-qc-fg inline-flex items-center gap-1">
				<Check size={10} /> Restart to apply update
			</span>
		{:else if status === 'error'}
			<span class="text-qc-danger inline-flex items-center gap-1" data-tip={errorMsg}>
				<AlertCircle size={10} /> Update failed
			</span>
		{/if}
	</div>

	<div class="flex items-center gap-1">
		{#if isDesktop}
			{#if status === 'idle' || status === 'uptodate' || status === 'error'}
				<button
					onclick={handleCheck}
					class="h-5 w-5 rounded hover:bg-qc-hover text-qc-muted hover:text-qc-fg inline-flex items-center justify-center"
					aria-label="Check for updates"
					data-tip="Check for updates — see if a newer version of QueryCastle is available"
				>
					<RefreshCw size={14} />
				</button>
			{:else if status === 'available'}
				<button
					onclick={handleInstall}
					class="h-5 px-2.5 rounded bg-qc-primary hover:bg-qc-primary-hover text-qc-primary-fg inline-flex items-center gap-1 text-[11px] font-medium"
				>
					<Download size={10} /> Update to v{updateVersion}
				</button>
				<button
					onclick={dismiss}
					class="h-5 w-5 rounded hover:bg-qc-hover text-qc-muted hover:text-qc-fg inline-flex items-center justify-center"
					aria-label="Dismiss"
				>
					×
				</button>
			{:else if status === 'checking'}
				<button
					disabled
					class="h-5 w-5 rounded text-gray-500 inline-flex items-center justify-center opacity-70 cursor-wait"
					aria-label="Checking for updates"
					data-tip="Checking for updates…"
				>
					<Loader2 size={11} class="animate-spin" />
				</button>
			{:else if status === 'downloading'}
				<button
					disabled
					class="h-5 px-2 rounded bg-gray-50 border border-gray-200 text-gray-500 inline-flex items-center gap-1 text-[11px] opacity-70 cursor-wait"
				>
					<Loader2 size={10} class="animate-spin" /> Downloading{#if total} {Math.round((downloaded / total) * 100)}%{/if}
				</button>
			{:else if status === 'ready'}
				<button
					onclick={async () => {
						try {
							const { relaunch } = await import('@tauri-apps/plugin-process');
							await relaunch();
						} catch {}
					}}
					class="h-5 px-2.5 rounded bg-qc-primary hover:bg-qc-primary-hover text-qc-primary-fg text-[11px] font-medium"
				>
					Restart now
				</button>
			{/if}
			<span class="w-px h-3 bg-qc-border" aria-hidden="true"></span>
		{/if}
		<button
			onclick={openRepo}
			class="h-5 w-5 rounded hover:bg-qc-hover text-qc-muted hover:text-qc-fg inline-flex items-center justify-center"
			aria-label="Open the QueryCastle GitHub repository"
			data-tip="View the QueryCastle repository on GitHub"
		>
			<GithubIcon size={14} />
		</button>
	</div>
</footer>
