<script lang="ts">
	import { Calendar, Hash, KeyRound, Link } from '@lucide/svelte';
	import type { GridColumnMeta } from '$lib/utils/grid-editors';

	let { meta }: { meta: GridColumnMeta | undefined } = $props();
</script>

{#if meta?.isPrimary}
	<KeyRound size={12} class="text-amber-500 shrink-0" />
{:else if meta?.fk}
	<Link size={12} class="text-gray-400 shrink-0" />
{:else if meta?.kind === 'number'}
	<Hash size={12} class="text-sky-500 shrink-0" />
{:else if meta?.kind === 'date' || meta?.kind === 'datetime'}
	<Calendar size={12} class="text-violet-400 shrink-0" />
{:else if meta?.kind === 'boolean'}
	<span
		class="shrink-0 w-3.5 h-3.5 rounded-[3px] border border-gray-300 text-[7px] leading-3 text-center font-semibold text-gray-500"
		title={meta.dataType}>01</span
	>
{:else}
	<span
		class="shrink-0 w-3.5 h-3.5 rounded-[3px] border border-gray-300 text-[9px] leading-3 text-center font-serif italic text-gray-500"
		title={meta?.dataType ?? 'text'}>T</span
	>
{/if}
