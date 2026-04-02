<script lang="ts">
	import { Database, Pencil, Plug, Plus, Trash2 } from '@lucide/svelte';
	import type { ConnectionInput } from '../lib/rpc';

	let {
		savedConnections,
		onConnect,
		onCreate,
		onEdit,
		onDelete,
		connectingName,
		searchQuery,
	}: {
		savedConnections: ConnectionInput[];
		onConnect: (connection: ConnectionInput) => void;
		onCreate: () => void;
		onEdit: (connection: ConnectionInput) => void;
		onDelete: (name: string) => void;
		connectingName: string | null;
		searchQuery: string;
	} = $props();

	function parseConnectionString(
		connectionString: string,
	): { host: string; port: number; user: string } | null {
		try {
			const url = new URL(connectionString);
			const host = url.hostname;
			const port = url.port ? parseInt(url.port) : 5432;
			const user = url.username;
			return { host, port, user };
		} catch {
			return null;
		}
	}

	function getConnectionDisplay(connection: ConnectionInput): string {
		if (connection.useConnectionString && connection.connectionString) {
			const parsed = parseConnectionString(connection.connectionString);
			if (parsed) {
				return `${parsed.user}@${parsed.host}:${parsed.port}`;
			}
		}
		return `${connection.user}@${connection.host}:${connection.port}`;
	}

	let filteredConnections = $derived.by(() => {
		const query = searchQuery.trim().toLowerCase();
		if (!query) return savedConnections;
		return savedConnections.filter((connection) => {
			return (
				connection.name.toLowerCase().includes(query) ||
				connection.host.toLowerCase().includes(query) ||
				connection.database.toLowerCase().includes(query)
			);
		});
	});
</script>

<section
	class="flex-1 overflow-auto px-8 py-10 bg-transparent flex flex-col items-center justify-center"
>
	<div class="w-full max-w-5xl mx-auto">
		<div class="flex items-start justify-between gap-4 mb-8">
			<div>
				<p
					class="text-xs uppercase tracking-[0.12em] text-gray-500 font-semibold mb-3 flex items-center gap-2"
				>
					<span
						class="inline-flex items-center justify-center w-5 h-5 rounded bg-emerald-100"
					>
						<Database size={14} strokeWidth={2} color="#059669" />
					</span>
					Database Workspace
				</p>
				<h1 class="text-gray-900 text-4xl font-semibold tracking-[-0.02em]">
					Choose a connection
				</h1>
				<p class="text-gray-500 text-[15px] mt-2 max-w-md leading-relaxed">
					Start from a saved connection, then browse tables or run SQL in one
					place.
				</p>
			</div>
		</div>

		{#if savedConnections.length === 0}
			<div
				class="rounded-2xl border border-gray-200 bg-white p-12 text-center shadow-sm"
			>
				<div
					class="w-14 h-14 mx-auto rounded-xl bg-gray-50 border border-gray-200 flex items-center justify-center text-gray-700 mb-5 shadow-inner"
				>
					<Database size={24} strokeWidth={1.5} />
				</div>
				<h2 class="text-gray-900 text-xl font-semibold tracking-tight">
					No saved connections
				</h2>
				<p class="text-gray-500 text-[15px] mt-2">
					Create your first PostgreSQL connection to continue.
				</p>
				<button
					onclick={onCreate}
					class="mt-6 h-10 px-6 rounded-lg text-sm font-medium shadow-sm border border-emerald-500 bg-emerald-500 text-white hover:bg-emerald-600 hover:border-emerald-600"
					>Create Connection</button
				>
			</div>
		{:else if filteredConnections.length === 0}
			<div
				class="rounded-2xl border border-dashed border-gray-300 bg-white p-12 text-center text-gray-500 text-[15px] font-medium"
			>
				No saved connections match your search.
			</div>
		{:else}
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
				{#each filteredConnections as connection}
					<div
						role="button"
						tabindex="0"
						onclick={() => {
							if (connectingName === connection.name) return;
							onConnect(connection);
						}}
						onkeydown={(event) => {
							if (connectingName === connection.name) return;
							if (event.key === 'Enter' || event.key === ' ') {
								event.preventDefault();
								onConnect(connection);
							}
						}}
						class="group rounded-xl border border-gray-200 bg-white p-5 hover:border-gray-300 hover:shadow-md transition-all duration-300 relative overflow-hidden flex flex-col"
					>
						<div
							class="absolute top-0 right-0 p-4 opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1"
						>
							<button
								onclick={(event) => {
									event.stopPropagation();
									onEdit(connection);
								}}
								class="text-gray-500 hover:text-gray-900 bg-white/80 backdrop-blur rounded p-1.5 shadow-sm border border-gray-200"
								aria-label="Edit connection"
								title="Edit connection"
							>
								<Pencil size={14} />
							</button>
							<button
								onclick={(event) => {
									event.stopPropagation();
									onDelete(connection.name);
								}}
								class="text-gray-500 hover:text-red-600 bg-white/80 backdrop-blur rounded p-1.5 shadow-sm border border-gray-200"
								aria-label="Delete connection"
								title="Delete connection"
							>
								<Trash2 size={14} />
							</button>
						</div>
						<div class="flex items-start gap-4 mb-6">
							<div
								class="w-10 h-10 rounded-lg bg-gray-50 border border-gray-200 flex items-center justify-center text-gray-700 shrink-0"
							>
								<Database size={18} strokeWidth={1.5} />
							</div>
							<div class="min-w-0 flex-1">
								<div
									class="text-gray-900 text-[15px] font-semibold truncate tracking-tight"
								>
									{connection.name}
								</div>
								<div
									class="text-gray-500 text-[13px] mt-1 truncate font-mono-code"
								>
									{getConnectionDisplay(connection)}
								</div>
							</div>
						</div>

						<div class="mt-auto">
							<div
								class="flex items-center gap-2 px-3 py-2 bg-gray-50 rounded-lg mb-4 text-[13px] text-gray-500 font-mono-code border border-gray-200"
							>
								<div class="w-2 h-2 rounded-full bg-emerald-500"></div>
								<span class="truncate">{connection.database}</span>
							</div>
							<div
								class="w-full h-10 rounded-lg text-[13px] font-semibold inline-flex items-center justify-center gap-2 border border-gray-200 bg-white text-gray-700"
							>
								{#if connectingName === connection.name}
									<div
										class="w-4 h-4 border-2 border-gray-300 border-t-gray-700 rounded-full animate-spin"
									></div>
									Connecting...
								{:else}
									<Plug size={14} />
									Click card to connect
								{/if}
							</div>
						</div>
					</div>
				{/each}

				{#if savedConnections.length === 1}
					<button
						onclick={onCreate}
						class="rounded-xl border border-dashed border-gray-300 bg-white p-5 hover:border-emerald-500 hover:shadow-md transition-all duration-300 flex flex-col items-center justify-center text-center min-h-[210px]"
					>
						<div
							class="w-10 h-10 rounded-lg bg-emerald-500 text-white flex items-center justify-center mb-4"
						>
							<Plus size={18} />
						</div>
						<div class="text-gray-900 text-[15px] font-semibold tracking-tight">
							Create New Connection
						</div>
						<div class="text-gray-500 text-[13px] mt-1">
							Add another PostgreSQL connection
						</div>
					</button>
				{/if}
			</div>
		{/if}
	</div>
</section>
