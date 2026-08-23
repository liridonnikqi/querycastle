<script lang="ts">
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
		class="fixed inset-0 z-[90] flex items-center justify-center bg-black/40 backdrop-blur-[2px] p-4"
		onclick={onCancel}
		onkeydown={handleBackdropKeydown}
		role="presentation"
	>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="w-full max-w-sm rounded-xl border border-gray-200 bg-white shadow-[0_20px_60px_rgba(0,0,0,0.2)] overflow-hidden"
			onclick={(e) => e.stopPropagation()}
		>
			<div class="p-5">
				<div class="flex items-start gap-3">
					<div class={`w-9 h-9 rounded-full flex items-center justify-center shrink-0 ${variant === 'danger' ? 'bg-red-100 text-red-600' : 'bg-gray-100 text-gray-600'}`}>
						<AlertTriangle size={18} />
					</div>
					<div class="flex-1 min-w-0">
						<h3 class="text-sm font-semibold text-gray-900">{title}</h3>
						<p class="text-xs text-gray-600 mt-1 leading-relaxed">{message}</p>
					</div>
					<button
						onclick={onCancel}
						class="w-7 h-7 rounded-md flex items-center justify-center text-gray-400 hover:text-gray-700 hover:bg-gray-100 shrink-0"
						aria-label="Close"
					>
						<X size={14} />
					</button>
				</div>
			</div>
			<div class="px-5 py-3 bg-gray-50 border-t border-gray-200 flex items-center justify-end gap-2">
				<button
					onclick={onCancel}
					class="h-8 px-4 rounded-md border border-gray-200 bg-white text-xs text-gray-700 hover:bg-gray-100"
				>
					{cancelLabel}
				</button>
				<button
					onclick={onConfirm}
					class={`h-8 px-4 rounded-md text-xs font-medium text-white ${variant === 'danger' ? 'bg-red-600 hover:bg-red-700 border border-red-600' : 'bg-[#1c1c1e] hover:bg-black border border-[#1c1c1e]'}`}
				>
					{confirmLabel}
				</button>
			</div>
		</div>
	</div>
{/if}
