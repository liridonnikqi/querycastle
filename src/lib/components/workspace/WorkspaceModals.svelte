<script lang="ts">
	import ConnectionModal from '$lib/components/connection/ConnectionModal.svelte';
	import RenameTableModal from '$lib/components/workspace/RenameTableModal.svelte';
	import type { Workspace } from '$lib/workspace/controller.svelte';

	let { workspace }: { workspace: Workspace } = $props();
</script>

<ConnectionModal
	visible={workspace.showConnectionModal}
	editing={workspace.editingConnectionName !== null}
	connectionForm={workspace.connectionForm}
	connectionStringInput={workspace.connectionStringInput}
	testConnectionMessage={workspace.testConnectionMessage}
	testConnectionOk={workspace.testConnectionOk}
	isTestingConnection={workspace.isTestingConnection}
	isConnecting={workspace.isConnecting}
	onClose={() => {
		workspace.showConnectionModal = false;
		workspace.editingConnectionName = null;
	}}
	onModeChange={(mode) => {
		workspace.connectionInputMode =
			workspace.connectionForm.databaseType === 'sqlite' ? 'fields' : mode;
	}}
	onConnectionFormChange={(next) => (workspace.connectionForm = next)}
	onConnectionStringChange={(value) => (workspace.connectionStringInput = value)}
	onTest={() => void workspace.handleTestConnection()}
	onSaveAndConnect={() => void workspace.handleConnect(true)}
/>

<RenameTableModal
	visible={workspace.showRenameModal}
	target={workspace.renameTarget}
	value={workspace.renameValue}
	onValueChange={(value) => (workspace.renameValue = value)}
	onClose={() => (workspace.showRenameModal = false)}
	onSubmit={() => void workspace.submitRename()}
/>
