<script lang="ts">
	import { onMount } from 'svelte';
	import { isTauri } from '@tauri-apps/api/core';
	import { getVersion } from '@tauri-apps/api/app';
	import { check } from '@tauri-apps/plugin-updater';
	import { Download, RefreshCw, Check, AlertCircle, Loader2 } from '@lucide/svelte';
	import { downloadAndInstallUpdate } from '$lib/updater';

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
		if (!isDesktop) return;
		const t = setTimeout(() => void silentCheck(), 3000);
		return () => clearTimeout(t);
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
</script>

<footer
	class="h-6 flex items-center justify-between px-3 bg-white text-gray-600 text-xs shrink-0 border-t border-gray-200 select-none"
>
	<div class="flex items-center gap-2 min-w-0">
		<span class="text-gray-800 font-medium">QueryCastle</span>
		<span class="text-gray-500">v{version}</span>
		<span class="text-gray-300">|</span>
		{#if !isDesktop}
			<span class="text-gray-500">Updates available in desktop app</span>
		{:else if status === 'idle'}
			<span class="text-gray-500">Up to date</span>
		{:else if status === 'checking'}
			<span class="inline-flex items-center gap-1 text-gray-500">
				<Loader2 size={10} class="animate-spin" /> Checking for updates...
			</span>
		{:else if status === 'available'}
			<span class="text-emerald-600 font-medium" title={updateBody}>v{updateVersion} available</span>
		{:else if status === 'downloading'}
			<span class="inline-flex items-center gap-1 text-gray-700">
				<Loader2 size={10} class="animate-spin" />
				Downloading{#if total} {Math.round((downloaded / total) * 100)}%{/if}
			</span>
		{:else if status === 'ready'}
			<span class="text-emerald-600 inline-flex items-center gap-1">
				<Check size={10} /> Restart to apply update
			</span>
		{:else if status === 'uptodate'}
			<span class="text-emerald-600 inline-flex items-center gap-1">
				<Check size={10} /> Up to date
			</span>
		{:else if status === 'error'}
			<span class="text-red-600 inline-flex items-center gap-1" title={errorMsg}>
				<AlertCircle size={10} /> Update failed
			</span>
		{/if}
	</div>

	{#if isDesktop}
		<div class="flex items-center gap-1.5">
			{#if status === 'idle' || status === 'uptodate' || status === 'error'}
				<button
					onclick={handleCheck}
					class="h-5 px-2 rounded bg-gray-50 hover:bg-gray-100 border border-gray-200 text-gray-700 inline-flex items-center gap-1 text-[11px]"
				>
					<RefreshCw size={10} /> Check for updates
				</button>
			{:else if status === 'available'}
				<button
					onclick={handleInstall}
					class="h-5 px-2.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white inline-flex items-center gap-1 text-[11px] font-medium"
				>
					<Download size={10} /> Update to v{updateVersion}
				</button>
				<button
					onclick={dismiss}
					class="h-5 w-5 rounded hover:bg-gray-100 text-gray-500 hover:text-gray-700 inline-flex items-center justify-center"
					aria-label="Dismiss"
				>
					×
				</button>
			{:else if status === 'checking'}
				<button
					disabled
					class="h-5 px-2 rounded bg-gray-50 border border-gray-200 text-gray-500 inline-flex items-center gap-1 text-[11px] opacity-70 cursor-wait"
				>
					<Loader2 size={10} class="animate-spin" /> Checking...
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
					class="h-5 px-2.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white text-[11px] font-medium"
				>
					Restart now
				</button>
			{/if}
		</div>
	{/if}
</footer>
