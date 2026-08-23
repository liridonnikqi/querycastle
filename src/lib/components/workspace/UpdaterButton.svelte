<script lang="ts">
	import { onMount } from 'svelte';
	import { isTauri } from '@tauri-apps/api/core';
	import { check } from '@tauri-apps/plugin-updater';
	import { Download, RefreshCw, Check, AlertCircle, Loader2 } from '@lucide/svelte';
	import { downloadAndInstallUpdate } from '$lib/updater';

	let status = $state<
		'idle' | 'checking' | 'available' | 'downloading' | 'uptodate' | 'error' | 'ready'
	>('idle');
	let errorMsg = $state('');
	let updateVersion = $state('');
	let updateBody = $state('');
	let downloaded = $state(0);
	let total = $state<number | undefined>(undefined);
	let isDesktop = $state(false);
	// hold the Update object between check and install
	let pendingUpdate: Awaited<ReturnType<typeof check>> = null;

	onMount(() => {
		isDesktop = isTauri();
		if (!isDesktop) return;
		// silent auto-check after 3s
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
		pendingUpdate = null;
	}
</script>

{#if isDesktop}
	<div class="flex items-center gap-2">
		{#if status === 'idle'}
			<button
				onclick={handleCheck}
				class="h-7 px-2.5 rounded border border-white/15 text-xs text-gray-300 hover:text-white hover:bg-white/10 inline-flex items-center gap-1.5"
				title="Check for updates"
			>
				<Download size={12} /> Check for updates
			</button>
		{:else if status === 'checking'}
			<span class="h-7 px-2.5 rounded border border-white/10 text-xs text-gray-400 inline-flex items-center gap-1.5">
				<Loader2 size={12} class="animate-spin" /> Checking...
			</span>
		{:else if status === 'available'}
			<div class="flex items-center gap-1.5">
				<span
					class="h-7 px-2.5 rounded bg-emerald-600 text-white text-xs inline-flex items-center gap-1.5"
					title={updateBody}
				>
					v{updateVersion} available
				</span>
				<button
					onclick={handleInstall}
					class="h-7 px-2.5 rounded bg-white text-[#1c1c1e] text-xs font-medium hover:bg-gray-100 inline-flex items-center gap-1"
				>
					<Download size={12} /> Update
				</button>
				<button
					onclick={dismiss}
					class="h-7 w-7 rounded border border-white/15 text-gray-400 hover:text-white hover:bg-white/10 inline-flex items-center justify-center"
					aria-label="Dismiss"
				>
					×
				</button>
			</div>
		{:else if status === 'downloading'}
			<span class="h-7 px-3 rounded border border-white/10 text-xs text-gray-300 inline-flex items-center gap-1.5">
				<Loader2 size={12} class="animate-spin" />
				Downloading{#if total}
					{Math.round((downloaded / total) * 100)}%
				{/if}
			</span>
		{:else if status === 'ready'}
			<span
				class="h-7 px-2.5 rounded bg-emerald-600 text-white text-xs inline-flex items-center gap-1.5"
			>
				<Check size={12} /> Restart to apply
			</span>
		{:else if status === 'uptodate'}
			<span
				class="h-7 px-2.5 rounded border border-emerald-500/30 bg-emerald-500/10 text-emerald-300 text-xs inline-flex items-center gap-1"
			>
				<Check size={12} /> Up to date
			</span>
		{:else if status === 'error'}
			<span
				class="h-7 px-2.5 rounded border border-red-500/30 bg-red-500/10 text-red-300 text-xs inline-flex items-center gap-1.5"
				title={errorMsg}
			>
				<AlertCircle size={12} /> Update failed
			</span>
			<button
				onclick={handleCheck}
				class="h-7 w-7 rounded border border-white/15 text-gray-400 hover:text-white hover:bg-white/10 inline-flex items-center justify-center"
				title="Retry"
			>
				<RefreshCw size={12} />
			</button>
		{/if}
	</div>
{/if}
