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
		<div class="w-full max-w-md rounded-xl border border-qc-border bg-qc-elevated shadow-[0_24px_60px_rgba(0,0,0,0.35)]">
			<div class="h-10 px-4 border-b border-qc-border flex items-center justify-between bg-qc-panel">
				<h3 class="text-sm font-semibold text-qc-fg">Rename Table</h3>
				<button onclick={onClose} class="text-qc-muted hover:text-qc-fg">x</button>
			</div>
			<div class="p-4 space-y-3">
				<div class="text-xs text-qc-muted">
					Current table: <span class="text-qc-fg">{target.schema}.{target.table}</span>
				</div>
				<input
					value={value}
					oninput={(event) =>
						onValueChange((event.currentTarget as HTMLInputElement).value)}
					placeholder="New table name"
					class="ui-input w-full h-9 text-sm px-2"
				/>
			</div>
			<div class="h-12 px-4 border-t border-qc-border flex items-center justify-end gap-2 bg-qc-panel">
				<button onclick={onClose} class="btn-secondary h-8 px-3 text-xs font-medium">Cancel</button>
				<button onclick={onSubmit} class="btn-primary h-8 px-3 text-xs font-medium">Rename</button>
			</div>
		</div>
	</div>
{/if}
