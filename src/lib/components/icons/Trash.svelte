<script lang="ts">
	let {
		size = 12,
		class: className = '',
		color = 'currentColor'
	}: { size?: number | string; class?: string; color?: string } = $props();
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	viewBox="0 0 24 24"
	width={size}
	height={size}
	fill="none"
	stroke={color}
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	aria-hidden="true"
	role="img"
>
	<path d="M3 6h18" />
	<path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
	<path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
	<path d="M10 11v6" />
	<path d="M14 11v6" />
</svg>
