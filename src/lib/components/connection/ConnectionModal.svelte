<script lang="ts">
	import { fade, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { Loader2, X } from '$lib/icons';
	import type { ConnectionInput } from '$lib/rpc';
	import DatabaseIcon from '$lib/components/ui/DatabaseIcon.svelte';
	import ConnectionFields from '$lib/components/connection/ConnectionFields.svelte';
	import ConnectionStatusBanner from '$lib/components/connection/ConnectionStatusBanner.svelte';
	import {
		DATABASE_ENGINES,
		connectionStringPlaceholder,
		defaultsForType,
		generateConnectionString,
		withDatabaseType,
		withParsedConnectionString,
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
	const busy = $derived(isTestingConnection || isConnecting);

	let wasVisible = $state(false);
	let nameInput: HTMLInputElement | null = $state(null);

	$effect(() => {
		if (visible && !wasVisible) {
			if (!editing) {
				const freshForm = defaultsForType('postgres');
				onConnectionFormChange(freshForm);
				onConnectionStringChange(generateConnectionString(freshForm));
			}
		}
		wasVisible = visible;
	});

	$effect(() => {
		if (!visible) return;
		const node = nameInput;
		if (node) queueMicrotask(() => node.focus());
	});

	function changeDatabaseType(nextType: ConnectionInput['databaseType']) {
		const nextForm = withDatabaseType(connectionForm, nextType);
		onConnectionFormChange(nextForm);
		onConnectionStringChange(generateConnectionString(nextForm));
		onModeChange('fields');
	}

	function handleFormChange(next: ConnectionInput) {
		onConnectionFormChange(next);
	}

	function handleStringChange(value: string) {
		onConnectionStringChange(value);
		const parsed = withParsedConnectionString(connectionForm, value);
		if (parsed) onConnectionFormChange(parsed);
		onModeChange(value.trim() ? 'string' : 'fields');
	}

	function handleBackdropClick(event: MouseEvent) {
		if (busy) return;
		if (event.target === event.currentTarget) onClose();
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (!visible) return;
		if (event.key === 'Escape' && !busy) onClose();
	}

	function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		onSaveAndConnect();
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if visible}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions: backdrop click mirrors Cancel; Escape is handled globally above -->
	<div
		role="presentation"
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/55 backdrop-blur-[1px] p-4 cursor-default"
		onclick={handleBackdropClick}
		transition:fade={{ duration: 120 }}
	>
		<div
			class="w-full max-w-[520px] max-h-[min(720px,90vh)] flex flex-col overflow-hidden rounded-xl border border-qc-border bg-qc-elevated shadow-[0_24px_60px_rgba(0,0,0,0.35)]"
			role="dialog"
			aria-modal="true"
			aria-label={editing ? `Edit ${engineLabel} connection` : 'New connection'}
			transition:scale={{ start: 0.98, duration: 160, easing: cubicOut }}
		>
			<div
				class="h-10 px-4 border-b border-qc-border flex items-center justify-between gap-3 bg-qc-panel shrink-0"
			>
				<div class="flex min-w-0 items-center gap-2.5">
					<DatabaseIcon type={connectionForm.databaseType} size={16} />
					<h3 class="truncate text-[13px] font-semibold text-qc-fg">
						{editing ? `Edit ${connectionForm.name || engineLabel}` : 'New connection'}
					</h3>
				</div>
				<button
					type="button"
					aria-label="Close modal"
					title="Close"
					onclick={onClose}
					disabled={busy}
					class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-qc-muted hover:bg-qc-hover hover:text-qc-fg disabled:opacity-50"
				>
					<X size={16} />
				</button>
			</div>

			<form
				class="flex min-h-0 flex-1 flex-col"
				onsubmit={handleSubmit}
			>
				<div class="min-h-0 flex-1 overflow-y-auto px-4 py-4 space-y-4">
					<div>
						<div class="text-[11px] font-medium text-qc-subtle mb-2">Database</div>
						<div class="grid grid-cols-2 gap-2">
							{#each DATABASE_ENGINES as database (database.value)}
								<button
									type="button"
									title={database.label}
									onclick={() => changeDatabaseType(database.value)}
									class={`provider-tile h-11 px-3 rounded-sm border border-qc-border bg-qc-panel flex items-center gap-2.5 text-left ${
										connectionForm.databaseType === database.value ? 'selected' : ''
									}`}
								>
									<DatabaseIcon type={database.value} size={18} />
									<span class="min-w-0">
										<span class="block truncate text-[12px] font-medium"
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

					<div>
						<label class="text-[11px] font-medium text-qc-subtle" for="modal-connection-string"
							>Connection string</label
						>
						<input
							id="modal-connection-string"
							value={connectionStringInput}
							oninput={(event) => handleStringChange(event.currentTarget.value)}
							class="field-input w-full h-9 px-3 mt-1.5 text-[13px] font-mono placeholder:text-qc-muted"
							placeholder={connectionStringPlaceholder(connectionForm.databaseType)}
						/>
						<p class="mt-1.5 text-[11px] text-qc-muted">
							Paste a string to auto-fill, or enter the details below.
						</p>
					</div>

					<ConnectionFields
						variant="hub"
						form={connectionForm}
						connectionString={connectionStringInput}
						showString={false}
						passwordStored={editing}
						bind:nameInput
						onFormChange={handleFormChange}
						onStringChange={handleStringChange}
					/>

					<ConnectionStatusBanner
						message={testConnectionMessage}
						ok={testConnectionOk}
					/>
				</div>

				<div
					class="flex items-center justify-end gap-2 border-t border-qc-border bg-qc-panel px-4 py-3 shrink-0"
				>
					<button
						type="button"
						onclick={onClose}
						disabled={busy}
						class="btn-secondary h-8 px-3 text-[12px] font-medium disabled:opacity-60"
					>
						Cancel
					</button>
					<button
						type="button"
						onclick={onTest}
						disabled={busy}
						class="btn-secondary h-8 px-3 text-[12px] font-medium inline-flex items-center gap-1.5 disabled:opacity-60"
					>
						{#if isTestingConnection}
							<Loader2 size={13} class="animate-spin" />
							Testing…
						{:else}
							Test
						{/if}
					</button>
					<button
						type="submit"
						disabled={busy}
						class="btn-primary h-8 px-3 text-[12px] font-medium inline-flex items-center gap-1.5 disabled:opacity-60"
					>
						{#if isConnecting}
							<Loader2 size={13} class="animate-spin" />
							Connecting…
						{:else}
							Save and connect
						{/if}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}
