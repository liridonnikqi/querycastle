<script lang="ts">
	let { size = 22, class: className = '' }: { size?: number; class?: string } = $props();
</script>

<svg viewBox="0 0 24 24" width={size} height={size} class={className} aria-hidden="true">
	<path fill="#f28e2b" d="M4 7.2 12 3l8 4.2v9.6L12 21l-8-4.2V7.2Z"/>
	<path fill="#fff" d="M8 8.2h8v2H8v3.6h6v2H8v-7.6Z"/>
</svg>
