<script lang="ts">
	import { fade } from 'svelte/transition';
</script>

<div
	in:fade={{ duration: 180 }}
	out:fade={{ duration: 260 }}
	class="fixed inset-0 z-[200] flex flex-col items-center justify-center gap-4 bg-[#1c1c1e] select-none"
	aria-label="Loading QueryCastle"
>
	<img src="/icon.svg" alt="QueryCastle" class="w-14 h-14" draggable="false" />
	<div class="text-sm text-gray-300 tracking-wide">Loading</div>
</div>

<style>
	:global(html, body) {
		overscroll-behavior: none;
	}
</style>
