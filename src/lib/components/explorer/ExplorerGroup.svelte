<script lang="ts">
	import { ChevronDown, ChevronRight } from '@lucide/svelte';
	import type { Snippet } from 'svelte';

	let {
		title,
		count,
		open,
		onToggle,
		children,
	}: {
		title: string;
		count: number;
		open: boolean;
		onToggle: () => void;
		children: Snippet;
	} = $props();
</script>

{#if count > 0}
	<button
		type="button"
		onclick={onToggle}
		class="w-full flex items-center gap-1 px-3 pt-2 pb-0.5 text-[10px] uppercase tracking-wide text-qc-muted hover:text-qc-subtle text-left"
	>
		{#if open}
			<ChevronDown size={12} class="shrink-0" />
		{:else}
			<ChevronRight size={12} class="shrink-0" />
		{/if}
		<span class="truncate">{title}</span>
		<span class="ml-auto tabular-nums">{count}</span>
	</button>
	{#if open}
		<div class="flex flex-col">
			{@render children()}
		</div>
	{/if}
{/if}
