<script lang="ts">
	import { fade, scale } from 'svelte/transition';
	import { X, AlertTriangle } from '@lucide/svelte';

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

	function handleBackdropKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onCancel();
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[90] flex items-center justify-center bg-black/50 backdrop-blur-[2px] p-4"
		onclick={onCancel}
		onkeydown={handleBackdropKeydown}
		role="presentation"
		transition:fade={{ duration: 140 }}
	>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="w-full max-w-sm rounded-xl border border-qc-border bg-qc-elevated shadow-[0_20px_60px_rgba(0,0,0,0.35)] overflow-hidden"
			onclick={(e) => e.stopPropagation()}
			transition:scale={{ duration: 180, start: 0.96 }}
		>
			<div class="p-5">
				<div class="flex items-start gap-3">
					<div class={`w-9 h-9 rounded-full flex items-center justify-center shrink-0 ${variant === 'danger' ? 'bg-qc-danger/15 text-qc-danger' : 'bg-qc-hover text-qc-subtle'}`}>
						<AlertTriangle size={18} />
					</div>
					<div class="flex-1 min-w-0">
						<h3 class="text-sm font-semibold text-qc-fg">{title}</h3>
						<p class="text-xs text-qc-muted mt-1 leading-relaxed">{message}</p>
					</div>
					<button
						onclick={onCancel}
						class="w-7 h-7 rounded-md flex items-center justify-center text-qc-muted hover:text-qc-fg hover:bg-qc-hover shrink-0"
						aria-label="Close"
					>
						<X size={14} />
					</button>
				</div>
			</div>
			<div class="px-5 py-3 bg-qc-panel border-t border-qc-border flex items-center justify-end gap-2">
				<button
					onclick={onCancel}
					class="btn-secondary h-8 px-4 text-xs font-medium"
				>
					{cancelLabel}
				</button>
				<button
					onclick={onConfirm}
					class={`h-8 px-4 text-xs font-medium ${variant === 'danger' ? 'btn-danger' : 'btn-primary'}`}
				>
					{confirmLabel}
				</button>
			</div>
		</div>
	</div>
{/if}
