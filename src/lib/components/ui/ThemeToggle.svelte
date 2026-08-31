<script lang="ts">
	import { Moon, Sun } from '@lucide/svelte';
	import { theme } from '$lib/theme.svelte';

	let {
		class: className = 'w-7 h-7 rounded-md flex items-center justify-center text-qc-muted hover:bg-qc-hover hover:text-qc-subtle',
		title = 'Toggle theme',
	}: {
		class?: string;
		title?: string;
	} = $props();
</script>

<button
	type="button"
	onclick={() => theme.toggle()}
	onmousedown={(event) => event.stopPropagation()}
	data-tauri-drag-region="false"
	class={className}
	{title}
	aria-label={title}
>
	{#if theme.value === 'light'}
		<Sun size={14} />
	{:else}
		<Moon size={14} />
	{/if}
</button>
