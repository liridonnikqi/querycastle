<script lang="ts">
	import { onMount } from 'svelte';
	import { fade, fly, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import {
		ArrowLeft,
		ArrowRight,
		Check,
		CircleAlert,
		FolderOpen,
		Loader2,
		MoreVertical,
		Plus,
		Search,
		SquarePen,
		Trash2,
	} from '@lucide/svelte';
	import { open } from '@tauri-apps/plugin-dialog';
	import { getVersion } from '@tauri-apps/api/app';
	import { isTauri } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import type { ConnectionInput, DatabaseType } from '$lib/rpc';
	import { rpc } from '$lib/rpc-client';
	import DatabaseIcon from '$lib/components/ui/DatabaseIcon.svelte';
	import QueryCastleLogo from '$lib/components/ui/QueryCastleLogo.svelte';
	import ThemeToggle from '$lib/components/ui/ThemeToggle.svelte';
	import WindowControls from '$lib/components/ui/WindowControls.svelte';
	import {
		connectionMetaLine,
		defaultsForType,
		generateConnectionString,
		loadRecentConnectionNames,
		normalizeConnectionInput,
		parseConnectionString,
		rememberRecentConnection,
	} from '$lib/utils/connection';

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
		{ value: 'postgres', label: 'PostgreSQL' },
		{ value: 'mysql', label: 'MySQL' },
		{ value: 'sqlite', label: 'SQLite' },
	];

	const providers: Array<{ value: DatabaseType; label: string }> = [
		{ value: 'postgres', label: 'PostgreSQL' },
		{ value: 'mysql', label: 'MySQL' },
		{ value: 'sqlite', label: 'SQLite' },
	];
	const isSqlite = $derived(form.databaseType === 'sqlite');

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
		form = defaultsForType(next);
		connectionString = generateConnectionString(form);
		useString = false;
		picked = true;
		clearTest();
	}

	function updateField<K extends keyof ConnectionInput>(
		key: K,
		value: ConnectionInput[K],
	) {
		form = { ...form, [key]: value };
		connectionString = generateConnectionString(form);
		clearTest();
	}

	function applyString(value: string) {
		connectionString = value;
		clearTest();
		const parsed = parseConnectionString(value, form);
		if (parsed) {
			form = normalizeConnectionInput({ ...form, ...parsed });
			useString = true;
			picked = true;
		} else {
			useString = value.trim().length > 0;
		}
	}

	async function chooseSqliteFile() {
		const selected = await open({
			multiple: false,
			directory: false,
			filters: [
				{ name: 'SQLite Database', extensions: ['db', 'sqlite', 'sqlite3'] },
				{ name: 'All Files', extensions: ['*'] },
			],
		});
		if (!selected || Array.isArray(selected)) return;
		const normalized = selected.replaceAll('\\', '/');
		const fileName = normalized.split('/').pop() ?? '';
		const nextName =
			!form.name.trim() || form.name === 'local_sqlite'
				? fileName.replace(/\.(sqlite|sqlite3|db)$/i, '') || form.name
				: form.name;
		form = { ...form, database: selected, name: nextName };
		connectionString = generateConnectionString(form);
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

	async function testConnection() {
		isTesting = true;
		testMessage = '';
		try {
			const payload = connectionPayload();
			const engineLabel =
				payload.databaseType === 'mysql'
					? 'MySQL'
					: payload.databaseType === 'sqlite'
						? 'SQLite'
						: 'PostgreSQL';
			const response = await rpc.testConnection(payload);
			testOk = response.ok;
			testMessage = response.ok
				? response.serverVersion
					? `${engineLabel} ${response.serverVersion}`
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
		class="hub-keep w-[clamp(300px,42vw,620px)] min-w-[300px] flex flex-col justify-end px-8 pb-10 pt-16"
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
				Your personal castle for managing and exploring databases.
			</p>
			<p class="mt-4 text-[11px] text-white/50">Version {appVersion}</p>
		</div>
	</aside>

	<div
		class="flex-1 flex flex-col min-w-0 min-h-0 pl-2 sm:pl-4 xl:pl-10 2xl:pl-16"
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="relative z-20 flex items-center justify-end px-2 h-10 shrink-0"
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
				<div
					class="w-full max-w-[720px] mx-auto px-8 pb-16 pt-8 animate-in fade-in slide-in-from-bottom-1 duration-200"
				>
					<label
						class="w-full h-11 rounded-sm border border-qc-border bg-qc-panel flex items-center gap-2.5 px-4"
					>
						<Search size={16} class="text-qc-muted shrink-0" />
						<input
							value={hubSearch}
							oninput={(event) => (hubSearch = event.currentTarget.value)}
							placeholder="Search connections..."
							class="flex-1 bg-transparent text-[13px] text-qc-fg placeholder:text-qc-muted outline-none"
						/>
					</label>

					{#if !query}
						<section class="mt-8 mb-8">
							<h2
								class="text-[13px] font-semibold tracking-wide uppercase text-qc-muted mb-3"
							>
								Recents
							</h2>
							<div>
									{#if filteredRecents.length > 0}
										<div class="flex flex-wrap gap-2">
											{#each filteredRecents as connection (connection.name)}
												<button
													type="button"
													onclick={() => connectSaved(connection)}
													class="h-8 pl-2 pr-3 rounded-sm border border-qc-border bg-qc-panel hover:bg-qc-hover inline-flex items-center gap-2 text-[12px] text-qc-subtle active:scale-[0.97] transition-transform duration-100"
												>
												<DatabaseIcon
													type={connection.databaseType}
													size={14}
													tone={connection.databaseType === 'sqlite' ? 'ink' : 'brand'}
												/>
													<span class="truncate max-w-[160px]"
														>{connection.name}</span
													>
												</button>
											{/each}
										</div>
									{:else}
										<p class="text-[13px] text-qc-muted">
											{engineFilter === 'all'
												? 'No recent connections.'
												: `No recent ${engineFilters.find((item) => item.value === engineFilter)?.label ?? 'provider'} connections.`}
										</p>
									{/if}
								</div>
						</section>
					{:else}
						<div class="h-8"></div>
					{/if}

					<section>
						<div class="flex flex-wrap items-center gap-2 mb-4">
							{#each engineFilters as filter}
								<button
									type="button"
									onclick={() => (engineFilter = filter.value)}
									class={`h-8 px-3 rounded-sm text-[12px] font-medium transition-colors duration-150 inline-flex items-center gap-1.5 ${
										engineFilter === filter.value
											? 'border border-qc-cell bg-qc-cell text-white'
											: 'border border-qc-border bg-qc-panel text-qc-subtle hover:bg-qc-hover'
									}`}
								>
									{#if filter.value !== 'all'}
										<DatabaseIcon
											type={filter.value}
											size={13}
											tone={engineFilter === filter.value ? 'white' : 'ink'}
										/>
									{/if}
									{filter.label}
								</button>
							{/each}
						</div>

						<div class="flex items-center justify-between gap-3 mb-3">
							<h2 class="text-[15px] font-semibold">Saved Connections</h2>
							<button
								type="button"
								onclick={openNew}
								class="btn-primary h-8 px-3.5 text-[12px] font-medium inline-flex items-center gap-1.5"
							>
								New <Plus size={14} />
							</button>
						</div>

							<div class="space-y-2">
								{#if filteredConnections.length === 0}
									<div
										class="rounded-sm border border-qc-border bg-qc-panel px-4 py-10 text-center text-[13px] text-qc-muted"
									>
										{savedConnections.length === 0
											? 'No saved connections yet.'
											: 'No connections match search.'}
									</div>
								{:else}
									{#each filteredConnections as connection, i (connection.name)}
										{@const isBusy = connectingName === connection.name}
										{@const menuOpen = activeMenuName === connection.name}
										<div
											class={`relative animate-in fade-in slide-in-from-bottom-1 duration-200 ${menuOpen ? 'z-30' : ''}`}
											style="animation-delay: {i * 18}ms; animation-fill-mode: both"
										>
											<button
												type="button"
												onclick={() => {
													if (isBusy) return;
													connectSaved(connection);
												}}
												class="conn-card w-full flex items-center gap-3 rounded-sm border border-qc-border bg-qc-panel px-3.5 py-3 text-left"
											>
												<div
													class="w-9 h-9 rounded-sm bg-qc-elevated border border-qc-border flex items-center justify-center shrink-0"
												>
												<DatabaseIcon
													type={connection.databaseType}
													size={18}
													tone={connection.databaseType === 'sqlite' ? 'ink' : 'brand'}
												/>
												</div>
												<div class="min-w-0 flex-1">
													<div class="text-[13px] font-medium truncate">
														{connection.name}
													</div>
													<div class="text-[11px] text-qc-muted truncate">
														{connectionMetaLine(connection)}
													</div>
												</div>
												{#if isBusy}
													<div
														class="w-3.5 h-3.5 border-2 border-qc-muted border-t-transparent rounded-full animate-spin mr-8"
													></div>
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
												class="absolute right-2 top-1/2 -translate-y-1/2 w-8 h-8 rounded-sm text-qc-muted hover:bg-qc-hover hover:text-qc-fg inline-flex items-center justify-center"
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
													class="ctx-menu absolute right-3 top-12 z-50 origin-top-right"
													transition:scale={{ start: 0.96, duration: 140, easing: cubicOut }}
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
								{/if}
							</div>
					</section>
				</div>
			{:else}
				<div
					class="w-full max-w-[640px] mx-auto px-8 pb-16 pt-6 animate-in fade-in slide-in-from-bottom-1 duration-200"
				>
					<button
						type="button"
						onclick={backHome}
						class="inline-flex items-center gap-2 text-[15px] font-semibold mb-8 hover:text-qc-subtle"
					>
						<ArrowLeft size={16} class="text-qc-muted" />
						<span class="text-qc-muted font-medium">Back</span>
						<span>New Connection</span>
					</button>

					<div class="mb-8">
						<label
							class="text-[13px] font-medium text-qc-fg"
							for="hub-connection-string">Connection String</label
						>
						<input
							id="hub-connection-string"
							value={connectionString}
							oninput={(event) => applyString(event.currentTarget.value)}
							class="field-input w-full h-11 px-3.5 mt-2 text-[13px] font-mono placeholder:text-qc-muted"
							placeholder="protocol://user:password@host:port/database"
						/>
						<p class="mt-2 text-[12px] text-qc-muted">
							Paste your connection string to auto-detect database type
						</p>
					</div>

					<div class="flex items-center gap-3 mb-5">
						<div class="flex-1 h-px bg-qc-border"></div>
						<span class="text-[11px] text-qc-muted">or select database</span>
						<div class="flex-1 h-px bg-qc-border"></div>
					</div>

					<div class="grid grid-cols-2 gap-2.5">
						{#each providers as provider}
							<button
								type="button"
								onclick={() => selectProvider(provider.value)}
								class={`provider-tile h-12 px-3 rounded-sm border border-qc-border bg-qc-panel flex items-center gap-3 text-left ${form.databaseType === provider.value && picked ? 'selected' : ''}`}
							>
								<DatabaseIcon type={provider.value} size={22} />
								<span class="text-[13px] font-medium">{provider.label}</span>
							</button>
						{/each}
					</div>

					{#if picked}
						<form
							class="mt-8 space-y-3.5"
							in:fly={{ y: 6, duration: 200, easing: cubicOut }}
							out:fade={{ duration: 120 }}
							onsubmit={(event) => {
								event.preventDefault();
								submit();
							}}
						>
							<div class={isSqlite ? '' : 'grid grid-cols-2 gap-3'}>
								<div>
									<div class="text-[11px] font-medium text-qc-subtle mb-1.5">
										Name
									</div>
									<input
										value={form.name}
										oninput={(event) =>
											updateField('name', event.currentTarget.value)}
										class="field-input w-full h-9 px-3 text-[13px] placeholder:text-qc-muted"
										placeholder="My database"
									/>
								</div>
								{#if !isSqlite}
									<div>
										<div class="text-[11px] font-medium text-qc-subtle mb-1.5">
											Host
										</div>
										<input
											value={form.host}
											oninput={(event) =>
												updateField('host', event.currentTarget.value)}
											class="field-input w-full h-9 px-3 text-[13px] placeholder:text-qc-muted"
											placeholder="localhost"
										/>
									</div>
								{/if}
							</div>
							{#if isSqlite}
								<div>
									<div class="text-[11px] font-medium text-qc-subtle mb-1.5">
										Database file
									</div>
									<div class="flex gap-2">
										<input
											value={form.database}
											oninput={(event) =>
												updateField('database', event.currentTarget.value)}
											class="field-input w-full h-9 px-3 text-[13px] placeholder:text-qc-muted font-mono"
											placeholder="C:/data/analytics.db"
										/>
										<button
											type="button"
											onclick={chooseSqliteFile}
											class="h-9 px-3 rounded-md border border-qc-border bg-qc-elevated text-qc-subtle text-[12px] inline-flex items-center gap-1.5 hover:bg-qc-hover"
										>
											<FolderOpen size={14} /> Open
										</button>
									</div>
								</div>
							{:else}
								<div class="grid grid-cols-2 gap-3">
									<div>
										<div class="text-[11px] font-medium text-qc-subtle mb-1.5">
											Port
										</div>
										<input
											value={String(form.port)}
											oninput={(event) =>
												updateField(
													'port',
													Number(event.currentTarget.value) ||
														(form.databaseType === 'mysql' ? 3306 : 5432),
												)}
											class="field-input w-full h-9 px-3 text-[13px] font-mono"
										/>
									</div>
									<div>
										<div class="text-[11px] font-medium text-qc-subtle mb-1.5">
											Database
										</div>
										<input
											value={form.database}
											oninput={(event) =>
												updateField('database', event.currentTarget.value)}
											class="field-input w-full h-9 px-3 text-[13px]"
										/>
									</div>
								</div>
								<div class="grid grid-cols-2 gap-3">
									<div>
										<div class="text-[11px] font-medium text-qc-subtle mb-1.5">
											User
										</div>
										<input
											value={form.user}
											oninput={(event) =>
												updateField('user', event.currentTarget.value)}
											class="field-input w-full h-9 px-3 text-[13px]"
										/>
									</div>
									<div>
										<div class="text-[11px] font-medium text-qc-subtle mb-1.5">
											Password
										</div>
										<input
											type="password"
											value={form.password}
											oninput={(event) =>
												updateField('password', event.currentTarget.value)}
											class="field-input w-full h-9 px-3 text-[13px] placeholder:text-qc-muted"
											placeholder="••••••••"
										/>
									</div>
								</div>
								<label
									class="flex items-center gap-2 text-[12px] text-qc-subtle"
								>
									<input
										type="checkbox"
										class="qc-check"
										checked={form.ssl}
										onchange={(event) => {
											const checked = event.currentTarget.checked;
											updateField('ssl', checked);
											if (!checked) updateField('sslInsecure', false);
										}}
									/>
									Use SSL
								</label>
								<label
									class="flex items-center gap-2 text-[12px] text-qc-subtle"
								>
									<input
										type="checkbox"
										class="qc-check"
										checked={form.sslInsecure ?? false}
										disabled={!form.ssl}
										onchange={(event) =>
											updateField('sslInsecure', event.currentTarget.checked)}
									/>
									Allow insecure TLS (self-signed)
								</label>
							{/if}

							<div class="flex flex-row items-center justify-end gap-2 pt-2">
								{#if connectError || testMessage}
									<div
										class={`mr-auto min-w-0 flex items-center gap-1.5 text-[12px] ${
											connectError || !testOk ? 'text-qc-danger' : 'text-qc-muted'
										}`}
										in:fly={{ y: 4, duration: 160, easing: cubicOut }}
										out:fade={{ duration: 120 }}
									>
										{#if connectError || !testOk}
											<CircleAlert size={14} class="shrink-0" />
											<span class="truncate">{connectError || testMessage}</span>
										{:else}
											<Check size={14} class="shrink-0 text-qc-subtle" />
											<span class="truncate">{testMessage}</span>
										{/if}
									</div>
								{/if}
								<button
									type="button"
									onclick={testConnection}
									disabled={isTesting || isConnecting}
									class="btn-secondary h-9 px-4 text-[13px] font-medium inline-flex items-center justify-center gap-2 disabled:opacity-60 box-border min-w-[88px]"
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
									class="btn-primary h-9 px-4 text-[13px] font-medium inline-flex items-center justify-center gap-2 disabled:opacity-60 box-border"
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
