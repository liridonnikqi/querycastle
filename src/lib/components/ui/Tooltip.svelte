<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		text = '',
		children,
	}: {
		text?: string;
		children?: Snippet;
	} = $props();
</script>

<span class="contents" data-tip={text || undefined}>
	{@render children?.()}
</span>
