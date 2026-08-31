<script lang="ts">
	import { onMount } from 'svelte';
	import { isTauri } from '@tauri-apps/api/core';
	import { getVersion } from '@tauri-apps/api/app';
	import { check } from '@tauri-apps/plugin-updater';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { Download, RefreshCw, Check, AlertCircle, Loader2 } from '@lucide/svelte';
	import GithubIcon from '$lib/components/ui/GithubIcon.svelte';
	import { downloadAndInstallUpdate } from '$lib/updater';

	const REPO_URL = 'https://github.com/liridonnikqi/querycastle';
	const tooltipClass =
		'pointer-events-none absolute bottom-full right-0 mb-1.5 w-max max-w-[240px] rounded bg-qc-elevated border border-qc-border px-2 py-1 text-[10px] font-normal leading-snug text-qc-fg opacity-0 shadow-lg transition-opacity duration-150 group-hover:opacity-100 group-focus-visible:opacity-100 z-50';

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
		status = 'checking';
		errorMsg = '';
		try {
			const update = await check();
			if (update) {
				pendingUpdate = update;
				updateVersion = update.version;
				updateBody = update.body ?? '';
				status = 'available';
			} else {
				status = 'uptodate';
				setTimeout(() => {
					if (status === 'uptodate') status = 'idle';
				}, 3000);
			}
		} catch (e) {
			status = 'error';
			errorMsg = e instanceof Error ? e.message : String(e);
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
			<span class="text-qc-fg font-medium" title={updateBody}>v{updateVersion} available</span>
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
			<span class="text-qc-danger inline-flex items-center gap-1" title={errorMsg}>
				<AlertCircle size={10} /> Update failed
			</span>
		{/if}
	</div>

	<div class="flex items-center gap-1">
		{#if isDesktop}
			{#if status === 'idle' || status === 'uptodate' || status === 'error'}
				<button
					onclick={handleCheck}
					class="group relative h-5 w-5 rounded hover:bg-qc-hover text-qc-muted hover:text-qc-fg inline-flex items-center justify-center"
					aria-label="Check for updates"
				>
					<RefreshCw size={14} />
					<span class={tooltipClass}
						>Check for updates — see if a newer version of QueryCastle is available</span
					>
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
					class="group relative h-5 w-5 rounded text-gray-500 inline-flex items-center justify-center opacity-70 cursor-wait"
					aria-label="Checking for updates"
				>
					<Loader2 size={11} class="animate-spin" />
					<span class={tooltipClass}>Checking for updates…</span>
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
			class="group relative h-5 w-5 rounded hover:bg-qc-hover text-qc-muted hover:text-qc-fg inline-flex items-center justify-center"
			aria-label="Open the QueryCastle GitHub repository"
		>
			<GithubIcon size={14} />
			<span class={tooltipClass}>View the QueryCastle repository on GitHub</span>
		</button>
	</div>
</footer>
