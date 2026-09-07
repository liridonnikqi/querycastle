<script lang="ts">
	let {
		size = 24,
		class: className = '',
		mono = false,
	}: { size?: number; class?: string; mono?: boolean } = $props();
</script>

<svg
	width={size}
	height={size}
	class={className}
	xmlns="http://www.w3.org/2000/svg"
	viewBox="0 0 24 24"
	aria-hidden="true"
>
	<path
		fill={mono ? 'currentColor' : '#CC2927'}
		d="M12 2c4.4 0 8 1.3 8 3s-3.6 3-8 3-8-1.3-8-3 3.6-3 8-3m8 5.2V10c0 1.7-3.6 3-8 3s-8-1.3-8-3V7.2c1.9 1.3 4.8 2 8 2s6.1-.7 8-2m0 5.1V15c0 1.7-3.6 3-8 3s-8-1.3-8-3v-2.7c1.9 1.3 4.8 2 8 2s6.1-.7 8-2m0 5.1V20c0 1.7-3.6 2-8 2s-8-.3-8-2v-2.6c1.9 1.3 4.8 2 8 2s6.1-.7 8-2"
	/>
</svg>
