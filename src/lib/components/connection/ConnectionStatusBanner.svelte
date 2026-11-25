<script lang="ts">
	import { Check, CircleAlert, X } from '$lib/icons';

	let {
		message,
		ok = false,
		onDismiss,
	}: {
		message: string;
		ok?: boolean;
		onDismiss?: () => void;
	} = $props();
</script>

{#if message}
	<div
		class={`flex items-start gap-2.5 rounded-lg border px-3 py-2.5 text-[12px] leading-5 ${
			ok ? 'status-banner-ok' : 'status-banner-error'
		}`}
		role={ok ? 'status' : 'alert'}
	>
		{#if ok}
			<Check size={14} class="mt-[3px] shrink-0" />
		{:else}
			<CircleAlert size={14} class="mt-[3px] shrink-0" />
		{/if}
		<div class="min-w-0 flex-1 whitespace-pre-wrap break-words">{message}</div>
		{#if onDismiss}
			<button
				type="button"
				onclick={onDismiss}
				class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded opacity-70 hover:bg-black/10 hover:opacity-100 dark:hover:bg-white/10"
				aria-label="Dismiss message"
			>
				<X size={12} />
			</button>
		{/if}
	</div>
{/if}

<style>
	.status-banner-ok {
		background: color-mix(in srgb, var(--qc-insert-accent) 14%, var(--qc-elevated));
		border-color: color-mix(in srgb, var(--qc-insert-accent) 30%, var(--qc-border));
		color: var(--qc-insert-fg);
	}

	.status-banner-error {
		background: color-mix(in srgb, var(--qc-danger) 14%, var(--qc-elevated));
		border-color: color-mix(in srgb, var(--qc-danger) 35%, var(--qc-border));
		color: var(--qc-delete-fg);
	}
</style>
