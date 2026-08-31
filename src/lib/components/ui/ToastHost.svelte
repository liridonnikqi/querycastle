<script lang="ts">
	import { Check, CircleAlert, X } from '@lucide/svelte';
	import { toast } from '$lib/stores/toast.svelte';
</script>

<div
	class="pointer-events-none fixed bottom-10 right-3 z-80 flex w-[min(22rem,calc(100vw-1.5rem))] flex-col gap-2"
	aria-live="polite"
	aria-relevant="additions"
>
	{#each toast.items as item (item.id)}
		<div
			class={`toast pointer-events-auto flex items-start gap-2 rounded-md px-3 py-2.5 shadow-lg ${
				item.kind === 'error' ? 'toast-error' : 'toast-success'
			}`}
			role="status"
		>
			{#if item.kind === 'error'}
				<CircleAlert size={14} class="mt-0.5 shrink-0" />
			{:else}
				<Check size={14} class="mt-0.5 shrink-0" />
			{/if}
			<div class="min-w-0 flex-1 text-xs leading-5 whitespace-pre-wrap">
				{item.message}
			</div>
			<button
				type="button"
				class="toast-dismiss shrink-0 rounded p-0.5"
				aria-label="Dismiss"
				onclick={() => toast.dismiss(item.id)}
			>
				<X size={12} />
			</button>
		</div>
	{/each}
</div>

<style>
	.toast-success {
		background: color-mix(in srgb, var(--qc-insert-accent) 18%, var(--qc-elevated));
		color: var(--qc-insert-fg);
	}

	.toast-error {
		background: color-mix(in srgb, var(--qc-danger) 18%, var(--qc-elevated));
		color: var(--qc-delete-fg);
	}

	.toast-dismiss {
		color: inherit;
		opacity: 0.65;
	}

	.toast-dismiss:hover {
		opacity: 1;
		background: color-mix(in srgb, currentColor 14%, transparent);
	}
</style>
