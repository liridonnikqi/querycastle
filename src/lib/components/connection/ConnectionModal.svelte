<script lang="ts">
	import { fade, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { ArrowRight, Loader2, X } from '$lib/icons';
	import type { ConnectionInput } from '$lib/rpc';
	import DatabaseIcon from '$lib/components/ui/DatabaseIcon.svelte';
	import ConnectionFields, {
		type ConnectionTab,
	} from '$lib/components/connection/ConnectionFields.svelte';
	import ConnectionStatusBanner from '$lib/components/connection/ConnectionStatusBanner.svelte';
	import {
		DATABASE_ENGINES,
		ENGINE_KEY,
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
		onClearMessage,
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
		onClearMessage?: () => void;
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

	let activeTab = $state<ConnectionTab>('general');
	let wasVisible = $state(false);
	let nameInput: HTMLInputElement | null = $state(null);

	$effect(() => {
		if (visible && !wasVisible) {
			activeTab = 'general';
		}
		wasVisible = visible;
	});

	$effect(() => {
		if (connectionForm.databaseType === 'sqlite' && activeTab === 'ssh') {
			activeTab = 'general';
		}
	});

	$effect(() => {
		if (!visible || activeTab !== 'general') return;
		const node = nameInput;
		if (node) queueMicrotask(() => node.focus());
	});

	function changeDatabaseType(nextType: ConnectionInput['databaseType']) {
		onClearMessage?.();
		const nextForm = withDatabaseType(connectionForm, nextType);
		onConnectionFormChange(nextForm);
		onConnectionStringChange(generateConnectionString(nextForm));
		onModeChange('fields');
	}

	function handleFormChange(next: ConnectionInput) {
		onClearMessage?.();
		onConnectionFormChange(next);
	}

	function handleStringChange(value: string) {
		onClearMessage?.();
		onConnectionStringChange(value);
		const parsed = withParsedConnectionString(connectionForm, value);
		if (parsed) onConnectionFormChange(parsed);
		onModeChange(value.trim() ? 'string' : 'fields');
	}

	let backdropPointerDown = false;

	function handleBackdropPointerDown(event: PointerEvent) {
		backdropPointerDown = event.target === event.currentTarget;
	}

	function handleBackdropClick(event: MouseEvent) {
		const startedOnBackdrop = backdropPointerDown;
		backdropPointerDown = false;
		if (busy) return;
		if (startedOnBackdrop && event.target === event.currentTarget) onClose();
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
		onpointerdown={handleBackdropPointerDown}
		onclick={handleBackdropClick}
		transition:fade={{ duration: 120 }}
	>
		<div
			class="w-full max-w-[33rem] max-h-[min(720px,92vh)] flex flex-col overflow-hidden rounded-xl border border-qc-border bg-qc-elevated shadow-[0_24px_64px_rgba(0,0,0,0.45)]"
			role="dialog"
			aria-modal="true"
			aria-label={title}
			transition:scale={{ start: 0.98, duration: 160, easing: cubicOut }}
		>
			<form class="flex min-h-0 flex-1 flex-col overflow-hidden" onsubmit={handleSubmit}>
				<div
					class="px-5 py-3.5 border-b border-qc-border-subtle flex items-center justify-between gap-3 shrink-0"
				>
					<div class="flex items-center gap-2.5 min-w-0">
						<div
							class="hub-engine-mark hub-engine-mark-sm"
							style="--bg: {ENGINE_KEY[connectionForm.databaseType]}"
						>
							<DatabaseIcon
								type={connectionForm.databaseType}
								size={14}
								tone="white"
							/>
						</div>
						<div class="flex items-baseline gap-2 min-w-0">
							<h3 class="truncate text-[15px] font-semibold tracking-tight text-qc-fg">
								{title}
							</h3>
							<span class="text-[11.5px] text-qc-muted shrink-0">{engineLabel}</span>
						</div>
					</div>
					<button
						type="button"
						aria-label="Close modal"
						data-tip="Close"
						onclick={onClose}
						disabled={busy}
						class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-qc-muted hover:bg-qc-hover hover:text-qc-fg disabled:opacity-50"
					>
						<X size={15} />
					</button>
				</div>

				<div class="px-5 pt-3.5 shrink-0">
					<div
						class="grid grid-cols-4 gap-[2px] p-[3px] rounded-[10px] border border-qc-border bg-qc-panel"
						role="radiogroup"
						aria-label="Database engine"
					>
						{#each DATABASE_ENGINES as provider (provider.value)}
							{@const selected = connectionForm.databaseType === provider.value}
							<button
								type="button"
								role="radio"
								aria-checked={selected}
								onclick={() => changeDatabaseType(provider.value)}
								class={`h-7 px-2 rounded-[7px] flex items-center justify-center gap-1.5 text-[12px] font-medium transition-colors min-w-0 ${
									selected
										? 'bg-qc-sidebar-active text-qc-fg'
										: 'text-qc-muted hover:text-qc-fg hover:bg-qc-sidebar-active/60'
								}`}
							>
								<DatabaseIcon
									type={provider.value}
									size={14}
									tone={selected ? (provider.value === 'sqlite' ? 'ink' : 'brand') : 'ink'}
								/>
								<span class="truncate">{provider.label}</span>
							</button>
						{/each}
					</div>
				</div>

				<div
					class="h-9 px-5 mt-2 border-b border-qc-border-subtle flex items-center gap-5 shrink-0"
					role="tablist"
					aria-label="Connection sections"
				>
					<button
						type="button"
						role="tab"
						aria-selected={activeTab === 'general'}
						onclick={() => (activeTab = 'general')}
						class={`relative h-full text-[12px] font-medium inline-flex items-center transition-colors ${
							activeTab === 'general'
								? 'text-qc-fg'
								: 'text-qc-muted hover:text-qc-subtle'
						}`}
					>
						General
						{#if activeTab === 'general'}
							<span class="absolute inset-x-0 bottom-0 h-[1.5px] bg-qc-fg rounded-full"
							></span>
						{/if}
					</button>
					{#if connectionForm.databaseType !== 'sqlite'}
						<button
							type="button"
							role="tab"
							aria-selected={activeTab === 'ssh'}
							onclick={() => (activeTab = 'ssh')}
							class={`relative h-full text-[12px] font-medium inline-flex items-center gap-1.5 transition-colors ${
								activeTab === 'ssh'
									? 'text-qc-fg'
									: 'text-qc-muted hover:text-qc-subtle'
							}`}
						>
							SSH Tunnel
							{#if connectionForm.sshEnabled}
								<span class="w-1.5 h-1.5 rounded-full bg-[#3c86ff]" aria-hidden="true"
								></span>
							{/if}
							{#if activeTab === 'ssh'}
								<span
									class="absolute inset-x-0 bottom-0 h-[1.5px] bg-qc-fg rounded-full"
								></span>
							{/if}
						</button>
					{/if}
					<button
						type="button"
						role="tab"
						aria-selected={activeTab === 'url'}
						onclick={() => (activeTab = 'url')}
						class={`relative h-full text-[12px] font-medium inline-flex items-center transition-colors ${
							activeTab === 'url'
								? 'text-qc-fg'
								: 'text-qc-muted hover:text-qc-subtle'
						}`}
					>
						Import URL
						{#if activeTab === 'url'}
							<span class="absolute inset-x-0 bottom-0 h-[1.5px] bg-qc-fg rounded-full"
							></span>
						{/if}
					</button>
				</div>

				<div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
					<ConnectionFields
						tab={activeTab}
						form={connectionForm}
						connectionString={connectionStringInput}
						passwordStored={editing}
						bind:nameInput
						onFormChange={handleFormChange}
						onStringChange={handleStringChange}
					/>
				</div>

				{#if testConnectionMessage}
					<div class="px-5 pb-3 shrink-0" transition:fade={{ duration: 120 }}>
						<ConnectionStatusBanner
							message={testConnectionMessage}
							ok={testConnectionOk}
							onDismiss={() => onClearMessage?.()}
						/>
					</div>
				{/if}

				<div
					class="px-5 py-2.5 border-t border-qc-border-subtle bg-qc-panel/50 flex items-center justify-between gap-3 shrink-0"
				>
					<button
						type="button"
						onclick={onClose}
						disabled={busy}
						class="h-[30px] px-2.5 rounded-[7px] text-[12px] font-medium text-qc-muted hover:bg-qc-hover hover:text-qc-fg disabled:opacity-60"
					>
						Cancel
					</button>
					<div class="flex items-center gap-2">
						<button
							type="button"
							onclick={onTest}
							disabled={busy}
							class="btn-secondary hub-action-btn disabled:opacity-60"
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
							class="btn-primary hub-action-btn disabled:opacity-60"
						>
							{#if isConnecting}
								<Loader2 size={13} class="animate-spin" />
								Connecting…
							{:else}
								{editing ? 'Save & connect' : 'Connect'}
								<ArrowRight size={13} />
							{/if}
						</button>
					</div>
				</div>
			</form>
		</div>
	</div>
{/if}
