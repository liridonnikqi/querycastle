<script lang="ts">
	import { Check, CircleAlert } from '$lib/icons';

	let {
		message,
		ok = false,
	}: {
		message: string;
		ok?: boolean;
	} = $props();
</script>

{#if message}
	<div
		class={`flex items-start gap-2 rounded-md px-3 py-2.5 text-[12px] leading-5 ${
			ok
				? 'bg-qc-panel text-qc-subtle'
				: 'bg-qc-danger/10 text-qc-danger'
		}`}
		role={ok ? 'status' : 'alert'}
	>
		{#if ok}
			<Check size={14} class="mt-0.5 shrink-0 text-qc-subtle" />
		{:else}
			<CircleAlert size={14} class="mt-0.5 shrink-0" />
		{/if}
		<div class="min-w-0 flex-1 whitespace-pre-wrap break-words">{message}</div>
	</div>
{/if}
