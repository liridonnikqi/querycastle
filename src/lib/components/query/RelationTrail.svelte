<script lang="ts">
	import { ChevronRight } from '@lucide/svelte';
	import { buildTrailCrumbs } from '$lib/utils/relation-sql';
	import type { RelationHop } from '$lib/utils/workspace';

	let {
		trail,
		onActivate,
	}: {
		trail: RelationHop[];
		onActivate: (index: number) => void;
	} = $props();

	let crumbs = $derived(buildTrailCrumbs(trail));
</script>

{#if crumbs.length > 0}
	<nav
		class="flex items-center gap-0.5 px-3 py-1.5 border-b border-qc-border bg-qc-panel text-xs min-w-0 overflow-x-auto hide-scrollbar shrink-0"
		aria-label="Relation trail"
	>
		{#each crumbs as crumb, index (crumb.index)}
			{#if index > 0}
				<ChevronRight size={12} class="text-qc-muted shrink-0" />
			{/if}
			{#if crumb.isCurrent}
				<span class="truncate text-qc-fg font-medium px-1" title={crumb.tooltip}
					>{crumb.label}</span
				>
			{:else}
				<button
					type="button"
					class="truncate text-qc-cell hover:underline px-1"
					title={crumb.tooltip}
					onclick={() => onActivate(crumb.index)}
				>
					{crumb.label}
				</button>
			{/if}
		{/each}
	</nav>
{/if}
