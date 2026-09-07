<script lang="ts">
	import { X } from '@lucide/svelte';
	import type { ConnectionInput } from '$lib/rpc';
	import DatabaseIcon from '$lib/components/ui/DatabaseIcon.svelte';
	import ConnectionFields from '$lib/components/connection/ConnectionFields.svelte';
	import {
		DATABASE_ENGINES,
		defaultsForType,
		generateConnectionString,
		withDatabaseType,
	} from '$lib/utils/connection';
	import { engineDisplayName } from '$lib/utils/dialect';

	let {
		visible,
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
		visible: boolean;
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

	const engineLabel = $derived(engineDisplayName(connectionForm.databaseType));
	const isSqlite = $derived(connectionForm.databaseType === 'sqlite');

	let step = $state(1);
	let wasVisible = $state(false);
	let nameInput: HTMLInputElement | null = $state(null);

	$effect(() => {
		if (visible && !wasVisible) {
			step = editing ? 2 : 1;
			if (!editing) {
				const freshForm = defaultsForType('postgres');
				onConnectionFormChange(freshForm);
				onConnectionStringChange(generateConnectionString(freshForm));
			}
		}
		wasVisible = visible;
	});

	$effect(() => {
		if (visible && step === 2) {
			const node = nameInput;
			if (node) queueMicrotask(() => node.focus());
		}
	});

	function changeDatabaseType(nextType: ConnectionInput['databaseType']) {
		const nextForm = withDatabaseType(connectionForm, nextType);
		onConnectionFormChange(nextForm);
		onConnectionStringChange(generateConnectionString(nextForm));
	}

	function handleFormChange(next: ConnectionInput) {
		onConnectionFormChange(next);
	}

	function handleStringChange(value: string) {
		onConnectionStringChange(value);
		onModeChange(value.trim() ? 'string' : 'fields');
	}

	function handleBackdropClick(event: MouseEvent) {
		if (event.target === event.currentTarget) onClose();
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (!visible) return;
		if (event.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if visible}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions: backdrop click mirrors Cancel; Escape is handled globally above -->
	<div
		role="presentation"
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-[2px] p-4 cursor-default"
		onclick={handleBackdropClick}
	>
		<div
			class="w-full max-w-[560px] overflow-hidden rounded-lg border border-qc-border bg-qc-elevated shadow-[0_24px_60px_rgba(0,0,0,0.35)]"
			role="dialog"
			aria-modal="true"
			aria-label={editing ? `Edit ${engineLabel} connection` : `New ${engineLabel} connection`}
		>
			<div class="border-b border-qc-border bg-qc-elevated px-4 py-3 text-qc-fg">
				<div class="flex items-center justify-between gap-3">
					<div class="flex min-w-0 items-center gap-2.5">
						<span
							class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-qc-hover text-qc-subtle"
						>
							<DatabaseIcon type={connectionForm.databaseType} size={18} />
						</span>
						<div class="min-w-0">
							<h3 class="truncate text-[13px] font-semibold text-qc-fg">
								{editing
									? `Edit ${engineLabel} Connection`
									: `New ${engineLabel} Connection`}
							</h3>
							<div class="mt-1 flex items-center gap-1 text-[11px]">
								<span
									class={`rounded-full px-2 py-0.5 ${step === 1 ? 'bg-qc-hover font-medium text-qc-fg' : 'text-qc-muted'}`}
									>1. Database</span
								>
								<span
									class={`rounded-full px-2 py-0.5 ${step === 2 ? 'bg-qc-hover font-medium text-qc-fg' : 'text-qc-muted'}`}
									>2. Details</span
								>
							</div>
						</div>
					</div>
					<button
						aria-label="Close modal"
						title="Close"
						onclick={onClose}
						class="flex h-7 w-7 shrink-0 items-center justify-center rounded-sm text-qc-muted hover:bg-qc-hover hover:text-qc-fg"
					>
						<X size={16} />
					</button>
				</div>
			</div>

			<div class="space-y-3 p-4 text-[13px] bg-qc-elevated text-qc-fg">
				{#if step === 1}
					<div>
						<div class="mb-2 flex items-center justify-between">
							<span
								class="text-[10px] font-semibold uppercase tracking-wider text-qc-muted"
								>Database Type</span
							>
							<span class="text-[11px] text-qc-muted">Choose an engine</span>
						</div>
						<div class="grid grid-cols-2 gap-2 sm:grid-cols-3">
							{#each DATABASE_ENGINES as database (database.value)}
								<button
									type="button"
									title={database.label}
									onclick={() => changeDatabaseType(database.value)}
									class={`relative flex min-w-0 flex-col items-center justify-center gap-1 rounded-md border px-2 py-2 text-center transition-colors ${
										connectionForm.databaseType === database.value
											? 'border-qc-fg bg-qc-hover text-qc-fg'
											: 'border-qc-border bg-qc-panel text-qc-subtle hover:border-qc-muted hover:bg-qc-hover'
									}`}
								>
									<span
										class="inline-flex h-8 w-8 items-center justify-center rounded-md bg-qc-elevated"
									>
										<DatabaseIcon type={database.value} size={20} />
									</span>
									<span class="min-w-0">
										<span class="block truncate text-[12px] font-semibold"
											>{database.label}</span
										>
										<span class="block text-[10px] text-qc-muted"
											>{database.detail}</span
										>
									</span>
								</button>
							{/each}
						</div>
					</div>
				{:else}
					<div
						class="flex items-center gap-3 rounded-md border border-qc-border bg-qc-panel px-3 py-2"
					>
						<DatabaseIcon type={connectionForm.databaseType} size={22} />
						<div class="min-w-0">
							<div class="text-[12px] font-semibold text-qc-fg">{engineLabel}</div>
							<div class="text-[11px] text-qc-muted">
								{isSqlite
									? 'Connect to a local database file'
									: 'Connect to a database server'}
							</div>
						</div>
						<button
							type="button"
							onclick={() => (step = 1)}
							class="ml-auto text-[11px] font-medium text-qc-muted hover:text-qc-fg"
							>Change</button
						>
					</div>

					<ConnectionFields
						form={connectionForm}
						connectionString={connectionStringInput}
						bind:nameInput
						onFormChange={handleFormChange}
						onStringChange={handleStringChange}
					/>
				{/if}

				{#if testConnectionMessage}
					<div
						class={`rounded-md border px-3 py-2 text-xs ${
							testConnectionOk
								? 'border-qc-border bg-qc-panel text-qc-subtle'
								: 'border-qc-danger/30 bg-qc-danger/10 text-qc-danger'
						}`}
					>
						{testConnectionMessage}
					</div>
				{/if}
			</div>

			<div
				class="flex flex-wrap items-center justify-between gap-2 border-t border-qc-border bg-qc-panel px-4 py-3"
			>
				{#if step === 2}
					<button
						type="button"
						onclick={() => (step = 1)}
						class="btn-secondary h-8 px-3 text-[13px] font-medium">Back</button
					>
				{:else}
					<span></span>
				{/if}
				<div class="flex flex-wrap items-center justify-end gap-2">
					<button onclick={onClose} class="btn-secondary h-8 px-3 text-[13px] font-medium">
						Cancel
					</button>
					{#if step === 1}
						<button
							type="button"
							onclick={() => (step = 2)}
							class="h-8 btn-primary px-3 text-[13px] font-medium"
						>
							Next
						</button>
					{:else}
						<button
							onclick={onTest}
							disabled={isTestingConnection || isConnecting}
							class="btn-secondary h-8 px-3 text-[13px] font-medium disabled:opacity-60"
						>
							{isTestingConnection ? 'Testing...' : 'Test'}
						</button>
						<button
							onclick={onSaveAndConnect}
							disabled={isTestingConnection || isConnecting}
							class="h-8 btn-primary px-3 text-[13px] font-medium disabled:opacity-60"
						>
							{isConnecting
								? 'Connecting...'
								: editing
									? 'Save Changes and Connect'
									: 'Save and Connect'}
						</button>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}
