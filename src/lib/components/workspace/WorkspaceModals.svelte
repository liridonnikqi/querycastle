<script lang="ts">
	import ConnectionModalBridge from '$lib/components/workspace/ConnectionModalBridge.svelte';
	import RenameTableModal from '$lib/components/workspace/RenameTableModal.svelte';
	import type { ConnectionInput } from '$lib/rpc';

	let {
		showConnectionModal,
		editingConnectionName,
		connectionForm,
		connectionStringInput,
		testConnectionMessage,
		testConnectionOk,
		isTestingConnection,
		isConnecting,
		showRenameModal,
		renameTarget,
		renameValue,
		onCloseConnectionModal,
		onModeChange,
		onConnectionFormChange,
		onConnectionStringChange,
		onTestConnection,
		onSaveAndConnect,
		onRenameValueChange,
		onCloseRenameModal,
		onSubmitRename,
	}: {
		showConnectionModal: boolean;
		editingConnectionName: string | null;
		connectionForm: ConnectionInput;
		connectionStringInput: string;
		testConnectionMessage: string;
		testConnectionOk: boolean;
		isTestingConnection: boolean;
		isConnecting: boolean;
		showRenameModal: boolean;
		renameTarget: { schema: string; table: string } | null;
		renameValue: string;
		onCloseConnectionModal: () => void;
		onModeChange: (mode: 'fields' | 'string') => void;
		onConnectionFormChange: (next: ConnectionInput) => void;
		onConnectionStringChange: (value: string) => void;
		onTestConnection: () => void;
		onSaveAndConnect: () => void;
		onRenameValueChange: (value: string) => void;
		onCloseRenameModal: () => void;
		onSubmitRename: () => void;
	} = $props();
</script>

<ConnectionModalBridge
	show={showConnectionModal}
	editing={editingConnectionName !== null}
	{connectionForm}
	{connectionStringInput}
	{testConnectionMessage}
	{testConnectionOk}
	{isTestingConnection}
	{isConnecting}
	onClose={onCloseConnectionModal}
	onModeChange={onModeChange}
	onConnectionFormChange={onConnectionFormChange}
	onConnectionStringChange={onConnectionStringChange}
	onTest={onTestConnection}
	onSaveAndConnect={onSaveAndConnect}
/>

<RenameTableModal
	visible={showRenameModal}
	target={renameTarget}
	value={renameValue}
	onValueChange={onRenameValueChange}
	onClose={onCloseRenameModal}
	onSubmit={onSubmitRename}
/>
