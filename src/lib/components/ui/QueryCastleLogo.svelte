<script lang="ts">
	let {
		size = 48,
		class: className = 'text-white',
		alt = 'QueryCastle',
	}: {
		size?: number;
		class?: string;
		alt?: string;
	} = $props();
</script>

<span
	role="img"
	aria-label={alt}
	class={`inline-block shrink-0 ${className}`}
	style={`width:${size}px;height:${size}px;background-color:currentColor;-webkit-mask:url('/icon.svg?v=3') center / contain no-repeat;mask:url('/icon.svg?v=3') center / contain no-repeat;`}
></span>
