<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import AppHeader from '$lib/components/workspace/AppHeader.svelte';
	import ConnectionTabsBar from '$lib/components/workspace/ConnectionTabsBar.svelte';
	import DisconnectedWorkspace from '$lib/components/workspace/DisconnectedWorkspace.svelte';
	import SqlWorkspace from '$lib/components/workspace/SqlWorkspace.svelte';
	import WorkspaceModals from '$lib/components/workspace/WorkspaceModals.svelte';
	import StatusBar from '$lib/components/workspace/StatusBar.svelte';
	import ToastHost from '$lib/components/ui/ToastHost.svelte';
	import SearchPalette from '$lib/components/workspace/SearchPalette.svelte';
	import { Workspace } from '$lib/workspace/controller.svelte';
	import { clampResultsHeight } from '$lib/utils/workspace';

	const workspace = new Workspace();

	onMount(() => {
		const cleanup = workspace.init();
		const onWindowResize = () => {
			if (!workspace.sqlSplitContainer) return;
			const total = workspace.sqlSplitContainer.clientHeight;
			workspace.resultsPaneHeight = clampResultsHeight(
				workspace.resultsPaneHeight,
				total,
			);
		};
		const isEditableTarget = (target: EventTarget | null) => {
			if (!(target instanceof HTMLElement)) return false;
			if (target.closest('.cm-editor, .cm-content')) return false;
			if (target.isContentEditable) return true;
			const tag = target.tagName.toLowerCase();
			return tag === 'input' || tag === 'textarea' || tag === 'select';
		};
		const onGlobalShortcuts = (event: KeyboardEvent) => {
			if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
			const key = event.key.toLowerCase();
			if (key === 'k') {
				event.preventDefault();
				workspace.showSearchPalette = !workspace.showSearchPalette;
				return;
			}
			if (isEditableTarget(event.target)) return;
			if (key === 'n') {
				event.preventDefault();
				workspace.addQueryTab();
				return;
			}
			if (key === 'x') {
				event.preventDefault();
				if (workspace.activeTabId) workspace.closeTab(workspace.activeTabId);
			}
		};
		const onContextMenu = (event: MouseEvent) => event.preventDefault();
		window.addEventListener('contextmenu', onContextMenu);
		window.addEventListener('resize', onWindowResize);
		window.addEventListener('keydown', onGlobalShortcuts);
		return () => {
			cleanup();
			window.removeEventListener('contextmenu', onContextMenu);
			window.removeEventListener('resize', onWindowResize);
			window.removeEventListener('keydown', onGlobalShortcuts);
		};
	});

	onDestroy(() => {
		workspace.stopResultsResize();
	});
</script>

<main
	class="h-screen w-full flex flex-col bg-qc-bg overflow-hidden text-[13px] text-qc-fg antialiased"
>
	{#if workspace.connectionStatus.connected || workspace.forceWorkspaceOnDisconnect}
		<AppHeader
			connectionStatus={workspace.connectionStatus}
			onDisconnect={() => void workspace.handleDisconnect()}
			onOpenSearch={() => (workspace.showSearchPalette = true)}
		>
			{#snippet leading()}
				<ConnectionTabsBar
					embedded
					sessions={workspace.openSessions}
					activeSessionId={workspace.activeSessionId}
					savedConnections={workspace.savedConnections}
					onSelect={(id) => void workspace.switchOpenSession(id)}
					onClose={(id) => void workspace.closeOpenSession(id)}
					onConnectSaved={(connection) => void workspace.connectSaved(connection)}
					onNew={() => workspace.openNewConnectionModal()}
				/>
			{/snippet}
		</AppHeader>
	{/if}
	{#if !workspace.connectionStatus.connected && !workspace.forceWorkspaceOnDisconnect}
		<DisconnectedWorkspace
			savedConnections={workspace.savedConnections}
			connectingName={workspace.connectingName}
			connectionSearch={workspace.connectionSearch}
			isConnecting={workspace.isConnecting}
			connectError={workspace.testConnectionMessage && !workspace.testConnectionOk
				? workspace.testConnectionMessage
				: ''}
			onConnect={(connection) => void workspace.connectSaved(connection)}
			onEdit={(connection) => workspace.startEditConnection(connection)}
			onDelete={(name) => void workspace.removeSavedConnection(name)}
			onSaveAndConnect={(payload) => {
				workspace.applyConnectionToForm(payload);
				workspace.editingConnectionName = null;
				void workspace.handleConnect(true);
			}}
		/>
	{:else}
		<div class="flex flex-1 overflow-hidden">
			<SqlWorkspace {workspace} />
		</div>
	{/if}
	<WorkspaceModals {workspace} />
	<StatusBar />
	<ToastHost />
	<SearchPalette
		open={workspace.showSearchPalette}
		searchQuery={workspace.explorerSearch}
		explorer={workspace.explorer}
		onSearchChange={(value) => (workspace.explorerSearch = value)}
		onClose={() => (workspace.showSearchPalette = false)}
		onSelectTable={(schema, table) => workspace.handlePaletteSelectTable(schema, table)}
		onSelectSchema={() => {}}
		onSelectRoutine={(routine) => {
			void workspace.openObjectDefinition({
				kind: routine.kind,
				schema: routine.schema,
				name: routine.name,
				objectId: routine.objectId,
				identityArgs: routine.identityArgs,
			});
		}}
		onSelectSequence={(sequence) => {
			void workspace.viewSequence(sequence.schema, sequence.name);
		}}
		onSelectIndex={(schema, table, name) => {
			void workspace.openObjectDefinition({ kind: 'index', schema, name, table });
		}}
		onSelectTrigger={(schema, table, name) => {
			void workspace.openObjectDefinition({ kind: 'trigger', schema, name, table });
		}}
	/>
</main>
