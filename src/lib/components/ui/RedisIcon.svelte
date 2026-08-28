<script lang="ts">
	let { size = 22, class: className = '' }: { size?: number; class?: string } = $props();
</script>

<svg viewBox="0 0 24 24" width={size} height={size} class={className} aria-hidden="true">
	<path fill="#dc382d" d="m3.2 8.3 8.8-3.6 8.8 3.6v2.1L12 14l-8.8-3.6V8.3Zm0 5.2 8.8 3.6 8.8-3.6v2.2L12 19.3l-8.8-3.6v-2.2Z"/>
</svg>
