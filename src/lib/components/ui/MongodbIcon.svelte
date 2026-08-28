<script lang="ts">
	let { size = 22, class: className = '' }: { size?: number; class?: string } = $props();
</script>

<svg viewBox="0 0 24 24" width={size} height={size} class={className} aria-hidden="true">
	<path fill="#47a248" d="M12 2.3c.5 2.1 2.9 3.6 3.9 6.3 1.5 4.1-.6 8.2-3.9 11.2-3.3-3-5.4-7.1-3.9-11.2C9.1 5.9 11.5 4.4 12 2.3Zm0 17.5c.5-2.8.5-6.3 0-9.5-.5 3.2-.5 6.7 0 9.5Z"/>
</svg>
