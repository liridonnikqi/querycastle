<script lang="ts">
	import { fade, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { ArrowRight, Loader2, X } from '$lib/icons';
	import type { ConnectionInput } from '$lib/rpc';
	import DatabaseIcon from '$lib/components/ui/DatabaseIcon.svelte';
	import ConnectionFields from '$lib/components/connection/ConnectionFields.svelte';
	import ConnectionStatusBanner from '$lib/components/connection/ConnectionStatusBanner.svelte';
	import {
		DATABASE_ENGINES,
		ENGINE_KEY,
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
	const title = $derived(
		editing ? `Edit ${connectionForm.name || engineLabel}` : 'New connection',
	);

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
			class="w-full max-w-[36rem] max-h-[min(780px,92vh)] flex flex-col overflow-hidden rounded-xl border border-qc-border bg-qc-hub shadow-[0_24px_60px_rgba(0,0,0,0.35)]"
			role="dialog"
			aria-modal="true"
			aria-label={title}
			transition:scale={{ start: 0.98, duration: 160, easing: cubicOut }}
		>
			<form class="flex min-h-0 flex-col" onsubmit={handleSubmit}>
				<div class="flex items-center justify-between gap-3 px-8 pt-6 pb-2 shrink-0">
					<h3 class="truncate text-[18px] font-semibold tracking-tight text-qc-fg">
						{title}
					</h3>
					<button
						type="button"
						aria-label="Close modal"
						data-tip="Close"
						onclick={onClose}
						disabled={busy}
						class="flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-qc-muted hover:bg-qc-hover hover:text-qc-fg disabled:opacity-50"
					>
						<X size={16} />
					</button>
				</div>

				<div class="min-h-0 overflow-y-auto px-8 pt-4 pb-5">
					<div class="hub-provider-grid mb-6">
						{#each DATABASE_ENGINES as provider (provider.value)}
							<button
								type="button"
								data-tip={provider.label}
								onclick={() => changeDatabaseType(provider.value)}
								class="hub-provider"
								class:selected={connectionForm.databaseType === provider.value}
							>
								<div
									class="hub-engine-mark hub-engine-mark-sm"
									style="--bg: {ENGINE_KEY[provider.value]}"
								>
									<DatabaseIcon
										type={provider.value}
										size={14}
										tone="white"
									/>
								</div>
								<span class="text-[13px] font-medium">{provider.label}</span>
							</button>
						{/each}
					</div>

					<div class="mb-6">
						<label
							class="text-[12px] font-medium text-qc-subtle"
							for="modal-connection-string">Connection string</label
						>
						<input
							id="modal-connection-string"
							value={connectionStringInput}
							oninput={(event) => handleStringChange(event.currentTarget.value)}
							class="field-input w-full h-10 px-3.5 mt-1.5 text-[13px] font-mono placeholder:text-qc-muted"
							placeholder={connectionStringPlaceholder(connectionForm.databaseType)}
						/>
						<p class="mt-1.5 text-[12px] text-qc-muted">
							Paste a URL to auto-detect the database type
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
				</div>

				<div
					class="relative flex items-center justify-between gap-3 px-8 py-3 border-t border-qc-border shrink-0"
				>
					{#if testConnectionMessage}
						<div
							class="pointer-events-none absolute inset-x-8 bottom-full mb-3 z-10"
							transition:fade={{ duration: 120 }}
						>
							<div class="pointer-events-auto shadow-lg">
								<ConnectionStatusBanner
									message={testConnectionMessage}
									ok={testConnectionOk}
								/>
							</div>
						</div>
					{/if}
					<button
						type="button"
						onclick={onClose}
						disabled={busy}
						class="h-10 px-1 text-[13px] font-medium text-qc-muted hover:text-qc-fg disabled:opacity-60"
					>
						Cancel
					</button>
					<div class="flex items-center gap-2">
						<button
							type="button"
							onclick={onTest}
							disabled={busy}
							class="btn-secondary hub-action-btn disabled:opacity-60 min-w-[88px]"
						>
							{#if isTestingConnection}
								<Loader2 size={14} class="animate-spin" />
								Testing…
							{:else}
								Test
							{/if}
						</button>
						<button
							type="submit"
							disabled={busy}
							class="btn-primary hub-action-btn disabled:opacity-60"
						>
							{#if isConnecting}
								<Loader2 size={14} class="animate-spin" />
								Connecting…
							{:else}
								Save and connect
								<ArrowRight size={14} />
							{/if}
						</button>
					</div>
				</div>
			</form>
		</div>
	</div>
{/if}
