<script lang="ts">
	import { snackbars, dismissSnackbar } from '$lib/stores/snackbar';
	import { fly } from 'svelte/transition';
	import { Check, AlertCircle, Info, Download, X } from '@lucide/svelte';
</script>

<div
	class="fixed bottom-4 right-4 z-[100] flex flex-col gap-2.5 max-w-sm w-[360px] pointer-events-none"
	aria-live="polite"
>
	{#each $snackbars as sb (sb.id)}
		<div
			in:fly={{ y: 12, duration: 180 }}
			out:fly={{ y: 6, duration: 120 }}
			class="pointer-events-auto flex items-center gap-2.5 rounded-md shadow-lg border px-3.5 py-2 text-sm backdrop-blur
				{sb.type === 'error'
				? 'bg-red-50 border-red-200 text-red-900'
				: sb.type === 'success'
					? 'bg-emerald-50 border-emerald-200 text-emerald-900'
					: sb.type === 'update'
						? 'bg-[#1c1c1e] border-white/10 text-gray-100'
						: 'bg-white border-gray-200 text-gray-900'}"
		>
			<div class="shrink-0 flex items-center">
				{#if sb.type === 'success'}
					<Check size={16} class="text-emerald-600" />
				{:else if sb.type === 'error'}
					<AlertCircle size={16} class="text-red-500" />
				{:else if sb.type === 'update'}
					<Download size={16} class="text-emerald-400" />
				{:else}
					<Info size={16} class="text-gray-500" />
				{/if}
			</div>
			<div class="flex-1 min-w-0">
				<div class="font-medium leading-none text-sm">{sb.message}</div>
				{#if sb.description}
					<div
						class="text-xs mt-0.5 leading-snug opacity-80 line-clamp-2"
						class:text-gray-400={sb.type === 'update'}
						class:text-gray-600={sb.type !== 'update'}
					>
						{sb.description}
					</div>
				{/if}
			</div>
			<div class="flex items-center gap-1 shrink-0">
				{#if sb.actionLabel && sb.onAction}
					<button
						onclick={sb.onAction}
						class="h-6 px-2.5 rounded text-xs font-medium transition
							{sb.type === 'update'
							? 'bg-white text-[#1c1c1e] hover:bg-gray-100'
							: sb.type === 'error'
								? 'bg-red-600 text-white hover:bg-red-700'
								: 'bg-gray-900 text-white hover:bg-black'}"
					>
						{sb.actionLabel}
					</button>
				{/if}
				{#if sb.showClose}
					<button
						onclick={() => dismissSnackbar(sb.id)}
						class="w-6 h-6 rounded flex items-center justify-center transition
							{sb.type === 'update'
							? 'text-gray-400 hover:text-white hover:bg-white/10'
							: 'text-gray-500 hover:text-gray-900 hover:bg-black/5'}"
						aria-label="Dismiss"
					>
						<X size={12} />
					</button>
				{/if}
			</div>
		</div>
	{/each}
</div>
