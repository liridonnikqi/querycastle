<script lang="ts">
	let { size = 22, class: className = '' }: { size?: number; class?: string } = $props();
</script>

<svg viewBox="0 0 24 24" width={size} height={size} class={className} aria-hidden="true">
	<path fill="#1377b9" d="M4 5.5 12 3l8 2.5v3L12 11 4 8.5v-3Zm0 5.4 8 2.5 8-2.5v3L12 16.4 4 13.9v-3Zm0 5.4 8 2.5 8-2.5v3L12 21l-8-2.7v-2Z"/>
</svg>
