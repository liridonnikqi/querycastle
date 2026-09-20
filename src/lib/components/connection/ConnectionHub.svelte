<script lang="ts">
	import { onMount } from 'svelte';
	import { fade, fly, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import {
		ArrowLeft,
		ArrowRight,
		Loader2,
		MoreVertical,
		Plus,
		Search,
		SquarePen,
		Trash2,
	} from '$lib/icons';
	import { getVersion } from '@tauri-apps/api/app';
	import { isTauri } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import type { ConnectionInput, DatabaseType } from '$lib/rpc';
	import { rpc } from '$lib/rpc-client';
	import DatabaseIcon from '$lib/components/ui/DatabaseIcon.svelte';
	import QueryCastleLogo from '$lib/components/ui/QueryCastleLogo.svelte';
	import ThemeToggle from '$lib/components/ui/ThemeToggle.svelte';
	import WindowControls from '$lib/components/ui/WindowControls.svelte';
	import ConnectionFields from '$lib/components/connection/ConnectionFields.svelte';
	import ConnectionStatusBanner from '$lib/components/connection/ConnectionStatusBanner.svelte';
	import {
		DATABASE_ENGINES,
		ENGINE_KEY,
		connectionMetaLine,
		connectionStringPlaceholder,
		decodeSshSecrets,
		defaultsForType,
		generateConnectionString,
		injectConnectionPassword,
		injectSshSecrets,
		loadRecentConnectionNames,
		rememberRecentConnection,
		sshSecretName,
		withDatabaseType,
		withParsedConnectionString,
	} from '$lib/utils/connection';
	import { engineDisplayName } from '$lib/utils/dialect';

	let {
		savedConnections,
		onConnect,
		onEdit,
		onDelete,
		onSaveAndConnect,
		connectingName,
		searchQuery,
		isConnecting = false,
		connectError = '',
	}: {
		savedConnections: ConnectionInput[];
		onConnect: (connection: ConnectionInput) => void;
		onEdit: (connection: ConnectionInput) => void;
		onDelete: (name: string) => void;
		onSaveAndConnect: (connection: ConnectionInput) => void;
		connectingName: string | null;
		searchQuery: string;
		isConnecting?: boolean;
		connectError?: string;
	} = $props();

	let view = $state<'home' | 'new'>('home');
	let picked = $state(false);
	let form = $state<ConnectionInput>(defaultsForType('postgres'));
	let connectionString = $state('');
	let useString = $state(false);
	let activeMenuName = $state<string | null>(null);
	let hubSearch = $state('');
	let recentNames = $state<string[]>([]);
	let appVersion = $state('0.1.6');
	let isTesting = $state(false);
	let testMessage = $state('');
	let testOk = $state(false);
	let engineFilter = $state<'all' | DatabaseType>('all');

	const engineFilters: Array<{ value: 'all' | DatabaseType; label: string }> = [
		{ value: 'all', label: 'All' },
		...DATABASE_ENGINES.map((engine) => ({
			value: engine.value,
			label: engine.label,
		})),
	];

	let query = $derived((hubSearch || searchQuery).trim().toLowerCase());

	let filteredConnections = $derived(
		savedConnections.filter((connection) => {
			if (engineFilter !== 'all' && connection.databaseType !== engineFilter)
				return false;
			if (!query) return true;
			return (
				connection.name.toLowerCase().includes(query) ||
				connection.host.toLowerCase().includes(query) ||
				connection.database.toLowerCase().includes(query)
			);
		}),
	);

	let recentConnections = $derived(
		recentNames
			.map((name) => savedConnections.find((item) => item.name === name))
			.filter((item): item is ConnectionInput => Boolean(item))
			.slice(0, 6),
	);

	let filteredRecents = $derived(
		recentConnections.filter(
			(connection) =>
				engineFilter === 'all' || connection.databaseType === engineFilter,
		),
	);

	onMount(() => {
		recentNames = loadRecentConnectionNames();
		if (!isTauri()) return;
		void getVersion()
			.then((value) => {
				appVersion = value;
			})
			.catch(() => {});
	});

	function openNew() {
		view = 'new';
		picked = false;
		form = defaultsForType('postgres');
		connectionString = '';
		useString = false;
		clearTest();
	}

	function openNewFromEmpty() {
		const engine = engineFilter;
		openNew();
		if (engine !== 'all') selectProvider(engine);
	}

	function backHome() {
		view = 'home';
		picked = false;
		clearTest();
	}

	function clearTest() {
		testMessage = '';
		testOk = false;
	}

	function connectionPayload(): ConnectionInput {
		return useString
			? {
					...form,
					useConnectionString: form.databaseType !== 'sqlite',
					connectionString: connectionString.trim(),
				}
			: { ...form, useConnectionString: false, connectionString: '' };
	}

	function selectProvider(next: DatabaseType) {
		form = withDatabaseType(form, next);
		connectionString = generateConnectionString(form);
		useString = false;
		picked = true;
		clearTest();
	}

	function applyString(value: string) {
		connectionString = value;
		clearTest();
		const parsed = withParsedConnectionString(form, value);
		if (parsed) {
			form = parsed;
			useString = true;
			picked = true;
		} else {
			useString = value.trim().length > 0;
		}
	}

	function handleFormChange(next: ConnectionInput) {
		form = next;
		clearTest();
	}

	function connectSaved(connection: ConnectionInput) {
		recentNames = rememberRecentConnection(connection.name);
		onConnect(connection);
	}

	function handleDeleteConnection(connection: ConnectionInput) {
		activeMenuName = null;
		const ok = confirm(
			`Delete connection "${connection.name}"?\nThis only removes it from QueryCastle. Your database is not affected.`,
		);
		if (!ok) return;
		onDelete(connection.name);
	}

	function submit() {
		recentNames = rememberRecentConnection(form.name);
		onSaveAndConnect(connectionPayload());
	}

	async function resolvedPayload(): Promise<ConnectionInput> {
		let payload = connectionPayload();
		if (!payload.password && payload.name.trim()) {
			try {
				const stored = await rpc.secretGet(payload.name);
				if (stored) payload = injectConnectionPassword(payload, stored);
			} catch {
				// Keychain misses are non-fatal.
			}
		}
		if (payload.sshEnabled && !payload.sshPassword && !payload.sshKeyPassphrase && payload.name.trim()) {
			try {
				const stored = await rpc.secretGet(sshSecretName(payload.name));
				if (stored) payload = injectSshSecrets(payload, decodeSshSecrets(stored));
			} catch {
				// Keychain misses are non-fatal.
			}
		}
		return payload;
	}

	async function testConnection() {
		isTesting = true;
		testMessage = '';
		try {
			const payload = await resolvedPayload();
			const response = await rpc.testConnection(payload);
			testOk = response.ok;
			testMessage = response.ok
				? response.serverVersion
					? `${engineDisplayName(payload.databaseType)} ${response.serverVersion}`
					: 'Connection succeeded'
				: response.message;
		} catch (error) {
			testOk = false;
			testMessage = error instanceof Error ? error.message : String(error);
		} finally {
			isTesting = false;
		}
	}

	async function handleTitlebarDoubleClick() {
		if (!isTauri()) return;
		try {
			await getCurrentWindow().toggleMaximize();
		} catch {
			// ignore
		}
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (event.key !== 'Escape') return;
		if (activeMenuName) activeMenuName = null;
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="h-full w-full flex overflow-hidden bg-qc-hub text-qc-fg">
	<aside
		class="hub-keep w-[clamp(300px,40vw,680px)] min-w-[260px] flex flex-col justify-end px-8 pb-10 pt-16"
		data-tauri-drag-region
	>
		<img src="/hero-poster.avif" alt="" class="hub-keep-art" />
		<div class="hub-keep-veil"></div>
		<div class="hub-keep-ember"></div>
		<div class="hub-keep-mark" data-tauri-drag-region="false">
			<QueryCastleLogo size={48} />
			<h1 class="mt-4 text-[28px] font-semibold tracking-tight text-white">
				QueryCastle
			</h1>
			<p class="mt-1.5 text-[13px] text-white/80 leading-snug">
				The Swiss Army knife of SQL clients.
			</p>
			<p class="mt-4 text-[11px] text-white/50">Version {appVersion}</p>
		</div>
	</aside>

	<div class="flex-1 flex flex-col min-w-0 min-h-0">
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="relative z-20 flex items-center justify-end px-3 h-10 shrink-0"
			data-tauri-drag-region
			ondblclick={handleTitlebarDoubleClick}
		>
			<div class="flex items-center gap-0.5" data-tauri-drag-region="false">
				<ThemeToggle
					class="w-8 h-7 rounded-md flex items-center justify-center text-qc-muted hover:bg-qc-hover hover:text-qc-fg"
				/>
				<WindowControls />
			</div>
		</div>

		<main class="flex-1 overflow-y-auto min-w-0">
			{#if view === 'home'}
				<div class="hub-stage">
					<div class="hub-home animate-in fade-in slide-in-from-bottom-1 duration-200">
						<div class="hub-toolbar">
							<label class="hub-search">
								<Search size={15} class="text-qc-muted shrink-0" />
								<input
									value={hubSearch}
									oninput={(event) => (hubSearch = event.currentTarget.value)}
									placeholder="Search connections..."
									class="flex-1 bg-transparent text-[13px] text-qc-fg placeholder:text-qc-muted outline-none"
								/>
							</label>
						</div>

						{#if connectError}
							<div class="mt-3">
								<ConnectionStatusBanner message={connectError} />
							</div>
						{/if}

						{#if !query}
							<section class="mt-6">
								<h2
									class="text-[11px] font-semibold tracking-wide uppercase text-qc-muted mb-2.5"
								>
									Recents
								</h2>
								<div class="hub-recents-row">
									{#if filteredRecents.length > 0}
										{#each filteredRecents as connection (connection.name)}
											<button
												type="button"
												onclick={() => connectSaved(connection)}
												class="hub-recent-chip"
											>
												<div
													class="hub-engine-mark hub-engine-mark-xs"
													style="--bg: {ENGINE_KEY[connection.databaseType]}"
												>
													<DatabaseIcon
														type={connection.databaseType}
														size={11}
														tone="white"
													/>
												</div>
												<span class="truncate max-w-[160px]"
													>{connection.name}</span
												>
											</button>
										{/each}
									{:else}
										<button
											type="button"
											onclick={openNewFromEmpty}
											class="hub-recent-chip hub-recent-ghost"
										>
											{#if engineFilter !== 'all'}
												<div
													class="hub-engine-mark hub-engine-mark-xs hub-ghost-engine"
													style="--bg: {ENGINE_KEY[engineFilter]}"
												>
													<DatabaseIcon
														type={engineFilter}
														size={11}
														tone="white"
													/>
												</div>
											{:else}
												<Plus size={12} class="text-qc-muted" />
											{/if}
											<span>None yet</span>
										</button>
									{/if}
								</div>
							</section>
						{/if}

						<div class="hub-list-head">
							<div
								class="hub-filters"
								role="tablist"
								aria-label="Filter by engine"
							>
								{#each engineFilters as filter}
									<button
										type="button"
										role="tab"
										aria-selected={engineFilter === filter.value}
										onclick={() => (engineFilter = filter.value)}
										class="hub-filter"
										class:active={engineFilter === filter.value}
									>
										{#if filter.value !== 'all'}
											<DatabaseIcon
												type={filter.value}
												size={12}
												tone={engineFilter === filter.value ? 'white' : 'ink'}
											/>
										{/if}
										{filter.label}
									</button>
								{/each}
							</div>
							<button
								type="button"
								onclick={openNew}
								class="btn-primary hub-new-btn"
							>
								<Plus size={13} />
								New
							</button>
						</div>

						{#if filteredConnections.length === 0}
							<div class="hub-conn-grid mt-5">
								{#if query}
									<button
										type="button"
										class="hub-card hub-ghost-card"
										onclick={() => (hubSearch = '')}
									>
										<div class="hub-ghost-mark">
											<Search size={16} />
										</div>
										<div class="min-w-0 flex-1 text-left">
											<div class="text-[13px] font-medium leading-tight">
												Nothing matches
											</div>
											<div class="mt-0.5 text-[11px] text-qc-muted">
												Clear search
											</div>
										</div>
									</button>
								{:else}
									<button
										type="button"
										class="hub-card hub-ghost-card"
										onclick={openNewFromEmpty}
									>
										{#if engineFilter !== 'all'}
											<div
												class="hub-engine-mark hub-ghost-engine"
												style="--bg: {ENGINE_KEY[engineFilter]}"
											>
												<DatabaseIcon
													type={engineFilter}
													size={16}
													tone="white"
												/>
											</div>
										{:else}
											<div class="hub-ghost-mark">
												<Plus size={16} />
											</div>
										{/if}
										<div class="min-w-0 flex-1 text-left">
											<div class="text-[13px] font-medium leading-tight">
												Add a connection
											</div>
											<div class="mt-0.5 text-[11px] text-qc-muted">
												Host, file, or connection string
											</div>
										</div>
									</button>
								{/if}
							</div>
						{:else}
							<div class="hub-conn-grid mt-5">
								{#each filteredConnections as connection, i (connection.name)}
									{@const isBusy = connectingName === connection.name}
									{@const menuOpen = activeMenuName === connection.name}
									{@const keyBg = ENGINE_KEY[connection.databaseType]}
									<div
										class={`relative ${menuOpen ? 'z-30' : ''}`}
										style="animation-delay: {i * 18}ms"
									>
										<button
											type="button"
											onclick={() => {
												if (isBusy) return;
												connectSaved(connection);
											}}
											class="hub-card"
										>
											<div class="hub-engine-mark" style="--bg: {keyBg}">
												<DatabaseIcon
													type={connection.databaseType}
													size={16}
													tone="white"
												/>
											</div>
											<div class="min-w-0 flex-1 text-left">
												<div
													class="text-[13px] font-medium truncate leading-tight"
												>
													{connection.name}
												</div>
												<div class="mt-0.5 text-[11px] text-qc-muted truncate">
													{connectionMetaLine(connection)}
												</div>
											</div>
											{#if isBusy}
												<Loader2
													size={14}
													class="animate-spin text-qc-muted mr-6 shrink-0"
												/>
											{/if}
										</button>
										<button
											type="button"
											onclick={(event) => {
												event.stopPropagation();
												activeMenuName =
													activeMenuName === connection.name
														? null
														: connection.name;
											}}
											class="absolute right-1.5 top-1/2 -translate-y-1/2 w-7 h-7 rounded-md text-qc-muted hover:bg-qc-hover hover:text-qc-fg inline-flex items-center justify-center"
											aria-label="Connection options"
										>
											<MoreVertical size={15} />
										</button>
										{#if menuOpen}
											<button
												type="button"
												class="fixed inset-0 z-40 cursor-default"
												aria-label="Close menu"
												onclick={() => (activeMenuName = null)}
											></button>
											<div
												class="ctx-menu absolute right-2 top-[calc(100%-6px)] z-50 origin-top-right"
												transition:scale={{
													start: 0.96,
													duration: 140,
													easing: cubicOut,
												}}
											>
												<button
													type="button"
													class="ctx-item"
													onclick={() => {
														activeMenuName = null;
														onEdit(connection);
													}}
												>
													<SquarePen size={12} class="text-qc-muted" />
													Edit
												</button>
												<div class="ctx-separator"></div>
												<button
													type="button"
													class="ctx-item ctx-item-danger"
													onclick={() => handleDeleteConnection(connection)}
												>
													<Trash2 size={12} />
													Delete
												</button>
											</div>
										{/if}
									</div>
								{/each}
							</div>
						{/if}
					</div>
				</div>
			{:else}
				<div
					class="hub-new animate-in fade-in slide-in-from-bottom-1 duration-200"
				>
					<button
						type="button"
						onclick={backHome}
						class="inline-flex items-center gap-2 text-[13px] mb-8 hover:text-qc-subtle"
					>
						<ArrowLeft size={15} class="text-qc-muted" />
						<span class="text-qc-muted">Back</span>
						
					</button>

					<div class="hub-provider-grid mb-6">
						{#each DATABASE_ENGINES as provider (provider.value)}
							{@const selected =
								form.databaseType === provider.value && picked}
							<button
								type="button"
								onclick={() => selectProvider(provider.value)}
								class="hub-provider"
								class:selected
							>
								<div
									class="hub-engine-mark hub-engine-mark-sm"
									style="--bg: {ENGINE_KEY[provider.value]}"
								>
									<DatabaseIcon
										type={provider.value}
										size={14}
										tone="white"
									/>
								</div>
								<span class="text-[13px] font-medium">{provider.label}</span>
							</button>
						{/each}
					</div>

					<div class="mb-6">
						<label
							class="text-[12px] font-medium text-qc-subtle"
							for="hub-connection-string">Connection string</label
						>
						<input
							id="hub-connection-string"
							value={connectionString}
							oninput={(event) => applyString(event.currentTarget.value)}
							class="field-input w-full h-10 px-3.5 mt-1.5 text-[13px] font-mono placeholder:text-qc-muted"
							placeholder={connectionStringPlaceholder(form.databaseType)}
						/>
						<p class="mt-1.5 text-[12px] text-qc-muted">
							Paste a URL to auto-detect the database type
						</p>
					</div>

					{#if picked}
						<form
							class="space-y-3.5"
							in:fly={{ y: 6, duration: 200, easing: cubicOut }}
							out:fade={{ duration: 120 }}
							onsubmit={(event) => {
								event.preventDefault();
								submit();
							}}
						>
							<ConnectionFields
								variant="hub"
								{form}
								{connectionString}
								showString={false}
								onFormChange={handleFormChange}
								onStringChange={(value) => (connectionString = value)}
							/>

							{#if connectError || testMessage}
								<ConnectionStatusBanner
									message={connectError || testMessage}
									ok={!connectError && testOk}
								/>
							{/if}

							<div class="flex flex-row items-center justify-end gap-2">
								<button
									type="button"
									onclick={testConnection}
									disabled={isTesting || isConnecting}
									class="btn-secondary hub-action-btn disabled:opacity-60 min-w-[88px]"
								>
									{#if isTesting}
										<Loader2 size={14} class="animate-spin" />
										Testing…
									{:else}
										Test
									{/if}
								</button>
								<button
									type="submit"
									disabled={isConnecting || isTesting}
									class="btn-primary hub-action-btn disabled:opacity-60"
								>
									{#if isConnecting}
										<Loader2 size={14} class="animate-spin" />
										Connecting…
									{:else}
										Connect
										<ArrowRight size={14} />
									{/if}
								</button>
							</div>
						</form>
					{/if}
				</div>
			{/if}
		</main>
	</div>
</div>

<style>
	.hub-stage {
		min-height: 100%;
	}

	.hub-home,
	.hub-new {
		width: 100%;
		box-sizing: border-box;
		padding: 24px clamp(20px, 3vw, 40px) 40px;
	}

	.hub-home {
		max-width: none;
	}

	.hub-toolbar {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 40px;
	}

	.hub-search {
		flex: 1;
		height: 40px;
		min-height: 40px;
		max-height: 40px;
		box-sizing: border-box;
		border-radius: 10px;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 0 14px;
		border: 1px solid var(--qc-border);
		background: var(--qc-panel);
	}

	.hub-search:focus-within {
		border-color: var(--qc-focus-border);
		box-shadow: 0 0 0 3px var(--qc-focus-ring);
	}

	.hub-new {
		max-width: 36rem;
		margin-inline: auto;
		padding-top: clamp(8px, 2vh, 24px);
	}

	.hub-recent-chip {
		height: 32px;
		padding: 0 10px 0 6px;
		border-radius: 8px;
		border: 1px solid var(--qc-border);
		background: var(--qc-panel);
		display: inline-flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		color: var(--qc-subtle);
		transition:
			border-color 140ms ease,
			background-color 140ms ease;
	}

	.hub-recent-chip:hover {
		background: var(--qc-hover);
		border-color: var(--qc-conn-hover-border);
	}

	.hub-recent-ghost {
		border-style: dashed;
		background: transparent;
		color: var(--qc-muted);
	}

	.hub-recent-ghost:hover {
		color: var(--qc-fg);
	}

	.hub-recents-row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		min-height: 32px;
	}

	.hub-filters {
		display: flex;
		flex-wrap: wrap;
		gap: 2px;
		width: fit-content;
		padding: 3px;
		border: 1px solid var(--qc-border);
		border-radius: 10px;
		background: var(--qc-panel);
	}

	.hub-list-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		margin-top: 20px;
	}

	.hub-new-btn {
		height: 34px;
		min-height: 34px;
		padding: 0 12px;
		font-size: 12px;
		font-weight: 500;
		border-radius: 8px;
		display: inline-flex;
		align-items: center;
		gap: 5px;
		flex-shrink: 0;
	}

	.hub-filter {
		height: 28px;
		padding: 0 10px;
		border-radius: 7px;
		border: 0;
		background: transparent;
		color: var(--qc-muted);
		font-size: 12px;
		font-weight: 500;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		transition:
			background-color 140ms ease,
			color 140ms ease;
	}

	.hub-filter:hover {
		color: var(--qc-fg);
		background: var(--qc-sidebar-active);
	}

	.hub-filter.active {
		background: var(--qc-sidebar-active);
		color: var(--qc-fg);
	}

	.hub-conn-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(100%, 280px), 1fr));
		gap: 10px;
	}

	.hub-card {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 36px 12px 12px;
		border-radius: 10px;
		border: 1px solid var(--qc-border);
		background: var(--qc-panel);
		text-align: left;
		transition:
			border-color 160ms ease,
			background-color 160ms ease;
	}

	.hub-card:hover {
		border-color: var(--qc-conn-hover-border);
		background: var(--qc-conn-hover-bg);
	}

	.hub-ghost-card {
		border-style: dashed;
		background: transparent;
		color: var(--qc-subtle);
		padding-right: 12px;
	}

	.hub-ghost-card:hover {
		border-color: var(--qc-conn-hover-border);
		background: var(--qc-conn-hover-bg);
		color: var(--qc-fg);
	}

	.hub-ghost-mark {
		width: 32px;
		height: 32px;
		border-radius: 8px;
		border: 1px dashed var(--qc-border);
		display: grid;
		place-items: center;
		color: var(--qc-muted);
		flex-shrink: 0;
	}

	.hub-ghost-engine {
		opacity: 0.72;
	}
</style>
