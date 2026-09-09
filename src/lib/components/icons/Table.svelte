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
	<rect width="18" height="18" x="3" y="3" rx="2" />
	<path d="M3 9h18" />
	<path d="M3 15h18" />
	<path d="M9 3v18" />
</svg>
