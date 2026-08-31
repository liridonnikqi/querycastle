<script lang="ts">
	import ConnectionHub from '$lib/components/connection/ConnectionHub.svelte';
	import type { ConnectionInput } from '$lib/rpc';

	let {
		savedConnections,
		connectingName,
		connectionSearch,
		isConnecting = false,
		connectError = '',
		onConnect,
		onEdit,
		onDelete,
		onSaveAndConnect,
	}: {
		savedConnections: ConnectionInput[];
		connectingName: string | null;
		connectionSearch: string;
		isConnecting?: boolean;
		connectError?: string;
		onConnect: (connection: ConnectionInput) => void | Promise<void>;
		onEdit: (connection: ConnectionInput) => void;
		onDelete: (name: string) => void;
		onSaveAndConnect: (connection: ConnectionInput) => void | Promise<void>;
	} = $props();
</script>

<div class="flex flex-1 overflow-hidden min-h-0">
	<section class="flex-1 overflow-hidden bg-qc-hub">
		<ConnectionHub
			{savedConnections}
			{connectingName}
			searchQuery={connectionSearch}
			{isConnecting}
			{connectError}
			{onConnect}
			{onEdit}
			{onDelete}
			{onSaveAndConnect}
		/>
	</section>
</div>
