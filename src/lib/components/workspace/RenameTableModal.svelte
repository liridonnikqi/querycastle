<script lang="ts">
	let {
		visible,
		target,
		value,
		onValueChange,
		onClose,
		onSubmit,
	}: {
		visible: boolean;
		target: { schema: string; table: string } | null;
		value: string;
		onValueChange: (value: string) => void;
		onClose: () => void;
		onSubmit: () => void;
	} = $props();
</script>

{#if visible && target}
	<div class="fixed inset-0 z-70 bg-black/55 backdrop-blur-[1px] flex items-center justify-center p-4">
		<div class="w-full max-w-md rounded-xl border border-gray-200 bg-white shadow-[0_24px_60px_rgba(16,37,70,0.26)]">
			<div class="h-10 px-4 border-b border-gray-200 flex items-center justify-between bg-gray-50">
				<h3 class="text-sm font-semibold text-gray-900">Rename Table</h3>
				<button onclick={onClose} class="text-gray-500 hover:text-gray-900">x</button>
			</div>
			<div class="p-4 space-y-3">
				<div class="text-xs text-gray-500">
					Current table: <span class="text-gray-900">{target.schema}.{target.table}</span>
				</div>
				<input
					value={value}
					oninput={(event) =>
						onValueChange((event.currentTarget as HTMLInputElement).value)}
					placeholder="New table name"
					class="ui-input w-full h-9 text-sm px-2"
				/>
			</div>
			<div class="h-12 px-4 border-t border-gray-200 flex items-center justify-end gap-2 bg-gray-50">
				<button onclick={onClose} class="btn-secondary h-8 px-3 rounded-md text-xs">Cancel</button>
				<button onclick={onSubmit} class="btn-primary h-8 px-3 rounded-md text-xs font-medium">Rename</button>
			</div>
		</div>
	</div>
{/if}
