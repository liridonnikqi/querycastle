<script lang="ts">
	import { onMount } from 'svelte';
	import { isTauri } from '@tauri-apps/api/core';
	import { getVersion } from '@tauri-apps/api/app';
	import { check } from '@tauri-apps/plugin-updater';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { Download, RefreshCw, Check, AlertCircle, Loader2, X } from '$lib/icons';
	import GithubIcon from '$lib/components/ui/GithubIcon.svelte';
	import type { ConnectionStatus } from '$lib/rpc';
	import { displayDatabaseName } from '$lib/utils/dialect';
	import { downloadAndInstallUpdate } from '$lib/updater';
	import { toast } from '$lib/stores/toast.svelte';

	const REPO_URL = 'https://github.com/liridonnikqi/querycastle';

	let {
		connectionStatus,
	}: {
		connectionStatus?: ConnectionStatus;
	} = $props();

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
	class="h-7 flex items-center justify-between px-3 bg-qc-panel text-qc-muted text-[11px] shrink-0 border-t border-qc-border-subtle select-none"
>
	<div class="flex items-center gap-2 min-w-0">
		{#if connectionStatus?.connected}
			<span class="inline-flex items-center gap-1.5 text-qc-subtle truncate">
				<span class="w-1.5 h-1.5 rounded-full bg-emerald-500 shrink-0" aria-hidden="true"></span>
				<span class="font-medium text-qc-fg truncate max-w-44">
					{connectionStatus.name?.trim() ||
						displayDatabaseName(connectionStatus.database, connectionStatus.databaseType)}
				</span>
				{#if connectionStatus.database}
					<span class="text-qc-muted/60">·</span>
					<span class="truncate max-w-48">
						{displayDatabaseName(connectionStatus.database, connectionStatus.databaseType)}
					</span>
				{/if}
			</span>
			{#if connectionStatus.readOnly}
				<span class="px-1.5 py-0.5 rounded bg-qc-bg border border-qc-border text-[10px] leading-none text-qc-muted shrink-0">
					Read-only
				</span>
			{/if}
			{#if connectionStatus.sshTunnel}
				<span class="px-1.5 py-0.5 rounded bg-qc-bg border border-qc-border text-[10px] leading-none text-qc-muted shrink-0">
					SSH
				</span>
			{/if}
		{:else}
			<span>v{version}</span>
		{/if}
		{#if !isDesktop}
			<span>Updates available in desktop app</span>
		{:else if status === 'checking'}
			<span class="inline-flex items-center gap-1.5 leading-none">
				<Loader2 size={12} class="animate-spin" />
				<span>Checking…</span>
			</span>
		{:else if status === 'available'}
			<span class="text-qc-fg font-medium" data-tip={updateBody}>v{updateVersion} available</span>
		{:else if status === 'downloading'}
			<span class="inline-flex items-center gap-1.5 text-qc-subtle leading-none">
				<Loader2 size={12} class="animate-spin" />
				<span>Downloading{#if total} {Math.round((downloaded / total) * 100)}%{/if}</span>
			</span>
		{:else if status === 'ready'}
			<span class="text-qc-fg inline-flex items-center gap-1 leading-none">
				<Check size={12} /> Restart to apply update
			</span>
		{:else if status === 'error'}
			<span class="text-qc-danger inline-flex items-center gap-1 leading-none" data-tip={errorMsg}>
				<AlertCircle size={12} /> Update failed
			</span>
		{/if}
	</div>

	<div class="flex items-center gap-1.5">
		{#if connectionStatus?.connected}
			<span class="text-qc-muted/80 tabular-nums mr-0.5">v{version}</span>
		{/if}
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
					<X size={12} />
				</button>
			{:else if status === 'checking'}
				<button
					disabled
					class="h-5 w-5 rounded text-qc-muted inline-flex items-center justify-center opacity-70 cursor-wait"
					aria-label="Checking for updates"
					data-tip="Checking for updates…"
				>
					<Loader2 size={12} class="animate-spin" />
				</button>
			{:else if status === 'downloading'}
				<span
					class="h-5 px-2 rounded bg-qc-elevated border border-qc-border text-qc-muted inline-flex items-center gap-1 text-[11px]"
				>
					{total ? `${Math.round((downloaded / total) * 100)}%` : '…'}
				</span>
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
