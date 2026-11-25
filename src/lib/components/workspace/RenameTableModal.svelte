<script lang="ts">
	import { X } from '$lib/icons';

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

	let inputEl: HTMLInputElement | null = $state(null);

	$effect(() => {
		if (!visible || !target) return;
		const node = inputEl;
		if (node) queueMicrotask(() => node.focus());
	});

	let backdropPointerDown = false;

	function handleBackdropPointerDown(event: PointerEvent) {
		backdropPointerDown = event.target === event.currentTarget;
	}

	function handleBackdropClick(event: MouseEvent) {
		const startedOnBackdrop = backdropPointerDown;
		backdropPointerDown = false;
		if (startedOnBackdrop && event.target === event.currentTarget) onClose();
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (!visible || !target) return;
		if (event.key === 'Escape') {
			event.preventDefault();
			onClose();
		}
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if visible && target}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-70 bg-black/55 backdrop-blur-[1px] flex items-center justify-center p-4"
		onpointerdown={handleBackdropPointerDown}
		onclick={handleBackdropClick}
		role="presentation"
	>
		<form
			class="w-full max-w-md overflow-hidden rounded-xl border border-qc-border bg-qc-elevated shadow-[0_24px_64px_rgba(0,0,0,0.45)]"
			onsubmit={(event) => {
				event.preventDefault();
				onSubmit();
			}}
		>
			<div class="px-5 py-3.5 border-b border-qc-border-subtle flex items-center justify-between">
				<h3 class="text-[14px] font-semibold text-qc-fg">Rename Table</h3>
				<button
					type="button"
					onclick={onClose}
					class="w-7 h-7 rounded-md inline-flex items-center justify-center text-qc-muted hover:bg-qc-hover hover:text-qc-fg"
					aria-label="Close rename modal"
				>
					<X size={14} />
				</button>
			</div>
			<div class="px-5 py-4 space-y-3">
				<div class="text-xs text-qc-muted">
					Current table: <span class="font-mono text-qc-fg">{target.schema}.{target.table}</span>
				</div>
				<label class="flex flex-col gap-1">
					<span class="text-[11px] font-medium text-qc-subtle">New table name</span>
					<input
						bind:this={inputEl}
						value={value}
						oninput={(event) =>
							onValueChange((event.currentTarget as HTMLInputElement).value)}
						placeholder="New table name"
						class="ui-input w-full h-8 text-[13px] px-2.5"
					/>
				</label>
			</div>
			<div class="px-5 py-3 border-t border-qc-border-subtle flex items-center justify-end gap-2">
				<button type="button" onclick={onClose} class="btn-secondary h-8 px-3 text-xs font-medium">Cancel</button>
				<button type="submit" disabled={value.trim().length === 0} class="btn-primary h-8 px-3 text-xs font-medium disabled:opacity-60">Rename</button>
			</div>
		</form>
	</div>
{/if}
