<script lang="ts">
	import { ChevronDown, ChevronRight } from '@lucide/svelte';
	import type { Snippet } from 'svelte';

	let {
		title,
		count,
		accentClass,
		open,
		onToggle,
		icon,
		children,
	}: {
		title: string;
		count: number;
		accentClass: string;
		open: boolean;
		onToggle: () => void;
		icon: Snippet;
		children: Snippet;
	} = $props();
</script>

{#if count > 0}
	<button
		type="button"
		onclick={onToggle}
		class="flex items-center w-full min-w-0 px-2 py-1 mt-1 hover:bg-gray-50 rounded-md text-gray-700 group text-left"
	>
		{#if open}
			<ChevronDown size={14} class="mr-1 text-gray-400 group-hover:text-gray-600 shrink-0" />
		{:else}
			<ChevronRight size={14} class="mr-1 text-gray-400 group-hover:text-gray-600 shrink-0" />
		{/if}
		<div class={`w-4 h-4 mr-2 shrink-0 flex items-center justify-center rounded text-white ${accentClass}`}>
			{@render icon()}
		</div>
		<span class="font-medium text-gray-800 truncate min-w-0">{title}</span>
		<span
			class="ml-auto px-1.5 py-0.5 rounded-full bg-gray-100 border border-gray-200 text-[10px] leading-none font-medium text-gray-500"
		>
			{count}
		</span>
	</button>
	{#if open}
		<div class="flex flex-col ml-7 mt-0.5 space-y-0.5">
			{@render children()}
		</div>
	{/if}
{/if}
