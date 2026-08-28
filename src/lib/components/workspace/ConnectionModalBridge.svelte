<script lang="ts">
	import ConnectionModal from '$lib/components/connection/ConnectionModal.svelte';
	import type { ConnectionInput } from '$lib/rpc';

	let {
		show,
		editing,
		connectionForm,
		connectionStringInput,
		testConnectionMessage,
		testConnectionOk,
		isTestingConnection,
		isConnecting,
		onClose,
		onModeChange,
		onConnectionFormChange,
		onConnectionStringChange,
		onTest,
		onSaveAndConnect,
	}: {
		show: boolean;
		editing: boolean;
		connectionForm: ConnectionInput;
		connectionStringInput: string;
		testConnectionMessage: string;
		testConnectionOk: boolean;
		isTestingConnection: boolean;
		isConnecting: boolean;
		onClose: () => void;
		onModeChange: (mode: 'fields' | 'string') => void;
		onConnectionFormChange: (next: ConnectionInput) => void;
		onConnectionStringChange: (value: string) => void;
		onTest: () => void;
		onSaveAndConnect: () => void;
	} = $props();
</script>

<ConnectionModal
	visible={show}
	{editing}
	{connectionForm}
	{connectionStringInput}
	{testConnectionMessage}
	{testConnectionOk}
	{isTestingConnection}
	{isConnecting}
	{onClose}
	{onModeChange}
	{onConnectionFormChange}
	{onConnectionStringChange}
	onTest={onTest}
	onSaveAndConnect={onSaveAndConnect}
/>
