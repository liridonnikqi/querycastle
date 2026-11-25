<script lang="ts">
	import { fade, scale } from 'svelte/transition';
	import { X, AlertTriangle } from '$lib/icons';

	let {
		open,
		title,
		message,
		confirmLabel = 'Confirm',
		cancelLabel = 'Cancel',
		variant = 'danger',
		onConfirm,
		onCancel
	}: {
		open: boolean;
		title: string;
		message: string;
		confirmLabel?: string;
		cancelLabel?: string;
		variant?: 'danger' | 'default';
		onConfirm: () => void;
		onCancel: () => void;
	} = $props();

	let backdropPointerDown = false;

	function handleBackdropPointerDown(e: PointerEvent) {
		backdropPointerDown = e.target === e.currentTarget;
	}

	function handleBackdropClick(e: MouseEvent) {
		const startedOnBackdrop = backdropPointerDown;
		backdropPointerDown = false;
		if (startedOnBackdrop && e.target === e.currentTarget) onCancel();
	}

	function handleWindowKeydown(e: KeyboardEvent) {
		if (!open) return;
		if (e.key === 'Escape') {
			e.preventDefault();
			onCancel();
		}
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[90] flex items-center justify-center bg-black/50 backdrop-blur-[2px] p-4"
		onpointerdown={handleBackdropPointerDown}
		onclick={handleBackdropClick}
		role="presentation"
		transition:fade={{ duration: 140 }}
	>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="w-full max-w-sm rounded-xl border border-qc-border bg-qc-elevated shadow-[0_24px_64px_rgba(0,0,0,0.45)] overflow-hidden"
			transition:scale={{ duration: 180, start: 0.96 }}
		>
			<div class="px-5 py-3.5 border-b border-qc-border-subtle flex items-center justify-between gap-2">
				<div class="flex items-center gap-2 min-w-0">
					<AlertTriangle
						size={15}
						class={`shrink-0 ${variant === 'danger' ? 'text-qc-danger' : 'text-qc-muted'}`}
					/>
					<h3 class="text-[14px] font-semibold text-qc-fg truncate">{title}</h3>
				</div>
				<button
					onclick={onCancel}
					class="w-7 h-7 rounded-md flex items-center justify-center text-qc-muted hover:text-qc-fg hover:bg-qc-hover shrink-0"
					aria-label="Close"
				>
					<X size={14} />
				</button>
			</div>
			<div class="px-5 py-4">
				<p class="text-[12.5px] text-qc-subtle leading-relaxed">{message}</p>
			</div>
			<div class="px-5 py-3 border-t border-qc-border-subtle flex items-center justify-end gap-2">
				<button
					onclick={onCancel}
					class="btn-secondary h-8 px-3.5 text-[12px] font-medium"
				>
					{cancelLabel}
				</button>
				<button
					onclick={onConfirm}
					class={`h-8 px-3.5 text-[12px] font-medium ${variant === 'danger' ? 'btn-danger' : 'btn-primary'}`}
				>
					{confirmLabel}
				</button>
			</div>
		</div>
	</div>
{/if}
