<script lang="ts">
	import '../app.css';
	import type { Snippet } from 'svelte';
	import IconContext from 'phosphor-svelte/lib/IconContext';
	import TooltipHost from '$lib/components/ui/TooltipHost.svelte';

	let { children }: { children: Snippet } = $props();
</script>

<IconContext values={{ weight: 'bold', color: 'currentColor' }}>
	{@render children()}
</IconContext>
<TooltipHost />
