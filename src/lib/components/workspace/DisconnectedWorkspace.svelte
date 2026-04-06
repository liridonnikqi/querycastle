<script lang="ts">
	import ConnectionHub from '$lib/components/connection/ConnectionHub.svelte';
	import type { ConnectionInput } from '$lib/rpc';

	let {
		savedConnections,
		connectingName,
		connectionSearch,
		onConnect,
		onCreate,
		onEdit,
		onDelete,
	}: {
		savedConnections: ConnectionInput[];
		connectingName: string | null;
		connectionSearch: string;
		onConnect: (connection: ConnectionInput) => void | Promise<void>;
		onCreate: () => void;
		onEdit: (connection: ConnectionInput) => void;
		onDelete: (name: string) => void;
	} = $props();
</script>

<div class="flex flex-1 overflow-hidden">
	<section class="flex-1 overflow-auto bg-white">
		<ConnectionHub
			{savedConnections}
			{connectingName}
			searchQuery={connectionSearch}
			{onConnect}
			{onCreate}
			{onEdit}
			{onDelete}
		/>
	</section>
</div>
