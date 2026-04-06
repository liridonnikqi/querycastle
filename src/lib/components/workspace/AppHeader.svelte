<script lang="ts">
	import { ChevronRight, Database, Plus, Unplug } from '@lucide/svelte';
	import type { ConnectionStatus } from '$lib/rpc';

	let {
		connectionStatus,
		onCreateConnection,
		onDisconnect,
	}: {
		connectionStatus: ConnectionStatus;
		onCreateConnection: () => void;
		onDisconnect: () => void;
	} = $props();
</script>

<header
	class="flex items-center h-12 bg-[#1c1c1e] text-gray-300 px-4 shrink-0 shadow-sm z-30 relative"
>
	<div class="flex items-center space-x-3 min-w-0">
		<div class="bg-emerald-500 rounded p-1 flex items-center justify-center">
			<Database size={16} class="text-[#1c1c1e]" />
		</div>
		<div class="flex items-center text-sm min-w-0">
			<span class="text-gray-400 cursor-default hover:text-gray-200 truncate">
				{connectionStatus.connected ? connectionStatus.host : 'Disconnected'}
			</span>

			<ChevronRight size={14} class="mx-1 text-gray-600 shrink-0" />

			<span class="font-medium text-gray-100 cursor-default flex items-center truncate">
				{connectionStatus.connected
					? connectionStatus.database || 'No database selected'
					: 'No database selected'}
			</span>
		</div>
	</div>
	<div class="flex-1"></div>
	<div class="flex items-center gap-2">
		<button
			onclick={onCreateConnection}
			class="h-7 px-2 rounded border border-white/15 text-xs text-gray-300 hover:text-white hover:bg-white/10 inline-flex items-center gap-1"
		>
			<Plus size={12} />
			Connection
		</button>
		{#if connectionStatus.connected}
			<button
				onclick={onDisconnect}
				class="h-7 px-2 rounded border border-white/15 text-xs text-gray-300 hover:text-white hover:bg-white/10 inline-flex items-center gap-1"
			>
				<Unplug size={12} />
				Disconnect
			</button>
		{/if}
	</div>
</header>
