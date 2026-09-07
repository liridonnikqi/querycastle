<script lang="ts">
	import { onMount } from 'svelte';
	import { Check, ChevronsUpDown, Search } from '@lucide/svelte';
	import type { FkOption } from '$lib/utils/fk-lookup';
	import type { GridColumnKind } from '$lib/utils/grid-editors';

	let {
		kind,
		value,
		nullable = true,
		placeholder = '',
		disabled = false,
		fkOptions = [],
		fkLoading = false,
		autofocus = false,
		flush = false,
		onChange,
		onCommit,
		onCancel,
		onSearch,
		onEnter,
		startOpen = false,
	}: {
		kind: GridColumnKind;
		value: string;
		nullable?: boolean;
		placeholder?: string;
		disabled?: boolean;
		fkOptions?: FkOption[];
		fkLoading?: boolean;
		autofocus?: boolean;
		flush?: boolean;
		onChange: (next: string) => void;
		onCommit?: () => void;
		onCancel?: () => void;
		onSearch?: (query: string) => void;
		onEnter?: () => void;
		startOpen?: boolean;
	} = $props();

	let inputEl: HTMLInputElement | HTMLSelectElement | null = $state(null);
	let triggerEl: HTMLButtonElement | null = $state(null);
	let open = $state(false);
	let highlight = $state(0);
	let filter = $state('');
	let panelStyle = $state('');

	$effect(() => {
		if (!autofocus) return;
		const node = inputEl;
		if (!node) return;
		queueMicrotask(() => node.focus());
	});

	$effect(() => {
		if (!open || kind !== 'fk') return;
		queueMicrotask(() => inputEl?.focus());
	});

	onMount(() => {
		if (startOpen && kind === 'fk') {
			open = true;
			requestAnimationFrame(() => placePanel());
		}
	});

	let filteredOptions = $derived.by(() => {
		const q = filter.trim().toLowerCase();
		if (!q) return fkOptions;
		return fkOptions.filter((option) => {
			const idText = String(option.id).toLowerCase();
			return option.label.toLowerCase().includes(q) || idText.includes(q);
		});
	});

	let selectedOption = $derived.by(() => {
		if (value === '') return null;
		return (
			fkOptions.find((option) => String(option.id) === value) ??
			filteredOptions.find((option) => String(option.id) === value) ??
			null
		);
	});

	function hasDistinctLabel(option: FkOption): boolean {
		return option.label.trim().length > 0 && option.label !== String(option.id);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			if (kind === 'fk' && open) {
				event.stopPropagation();
				open = false;
				return;
			}
			open = false;
			onCancel?.();
			return;
		}
		if (kind === 'fk' && open) {
			if (event.key === 'ArrowDown') {
				event.preventDefault();
				highlight = Math.min(highlight + 1, Math.max(filteredOptions.length - 1, 0));
				return;
			}
			if (event.key === 'ArrowUp') {
				event.preventDefault();
				highlight = Math.max(highlight - 1, 0);
				return;
			}
			if (event.key === 'Enter') {
				event.preventDefault();
				const option = filteredOptions[highlight];
				if (option) chooseOption(option);
				else onCommit?.();
				releaseFocus();
				return;
			}
		}
		if (event.key === 'Enter') {
			event.preventDefault();
			(onEnter ?? onCommit)?.();
			// Commit unmounts this editor; blur so focus lands on the grid
			// instead of a detached input (which left the editor visibly
			// stuck open with focus lost to <body>).
			releaseFocus();
		}
	}

	function releaseFocus() {
		const active = document.activeElement;
		if (active instanceof HTMLElement) active.blur();
	}

	function placePanel() {
		const rect = triggerEl?.getBoundingClientRect();
		if (!rect) return;
		const width = Math.max(rect.width, 240);
		const estimatedHeight = 220;
		const top =
			rect.bottom + 4 + estimatedHeight > window.innerHeight
				? Math.max(8, rect.top - estimatedHeight)
				: rect.bottom + 4;
		const left = Math.min(rect.left, window.innerWidth - width - 8);
		panelStyle = `left:${Math.max(8, left)}px;top:${top}px;width:${width}px`;
	}

	function toggleOpen() {
		if (disabled) return;
		open = !open;
		filter = '';
		highlight = 0;
		if (open) placePanel();
	}

	function chooseOption(option: FkOption) {
		onChange(String(option.id));
		filter = '';
		open = false;
		onCommit?.();
	}

	function clearValue() {
		onChange('');
		filter = '';
		open = false;
		onCommit?.();
	}

	let triggerText = $derived.by(() => {
		if (flush) return value || placeholder || 'Choose…';
		if (selectedOption && hasDistinctLabel(selectedOption)) return selectedOption.label;
		return value || placeholder || 'Choose…';
	});

	let controlClass = $derived(
		flush
			? 'w-full h-full min-h-0 max-h-8 overflow-hidden text-ellipsis whitespace-nowrap px-4 py-0 rounded-none border-0 bg-transparent text-[12px] text-qc-fg outline-none shadow-none ring-0 focus:ring-0 focus:outline-none disabled:text-qc-muted'
			: 'w-full h-8 px-2 rounded-md border border-qc-border bg-qc-bg text-[12px] text-qc-fg outline-none focus:border-qc-focus-border disabled:bg-qc-elevated disabled:text-qc-muted',
	);
</script>

{#if kind === 'boolean'}
	<select
		bind:this={inputEl}
		class={controlClass}
		{disabled}
		value={value}
		onchange={(event) => {
			onChange((event.currentTarget as HTMLSelectElement).value);
			onCommit?.();
		}}
		onkeydown={handleKeydown}
	>
		{#if nullable || value === ''}
			<option value="">Empty</option>
		{/if}
		<option value="true">True</option>
		<option value="false">False</option>
	</select>
{:else if kind === 'fk'}
	<div class="relative h-full">
		<button
			bind:this={triggerEl}
			type="button"
			class={`${controlClass} flex items-center gap-1 text-left ${open && !flush ? 'border-qc-focus-border ring-1 ring-qc-focus-border' : ''}`}
			{disabled}
			onclick={toggleOpen}
			onkeydown={handleKeydown}
		>
			<span class={`truncate flex-1 ${value ? 'text-qc-fg' : 'text-qc-muted'}`}>{triggerText}</span>
			<ChevronsUpDown size={12} class="shrink-0 text-qc-muted" />
		</button>
		{#if open}
			<button
				type="button"
				class="fixed inset-0 z-[90] cursor-default"
				aria-label="Close list"
				onclick={() => (open = false)}
			></button>
			<div
				class="fixed z-[100] overflow-hidden rounded-md border border-qc-border bg-qc-elevated shadow-[0_10px_30px_rgba(0,0,0,0.28)]"
				style={panelStyle}
			>
				<div class="flex items-center gap-2 border-b border-qc-border px-2">
					<Search size={12} class="text-qc-muted shrink-0" />
					<input
						bind:this={inputEl}
						value={filter}
						oninput={(event) => {
							filter = (event.currentTarget as HTMLInputElement).value;
							highlight = 0;
							onSearch?.(filter);
						}}
						onkeydown={handleKeydown}
						placeholder="Search…"
						class="h-8 w-full bg-transparent text-[12px] outline-none"
					/>
				</div>
				<div class="max-h-48 overflow-y-auto py-1">
					{#if fkLoading && filteredOptions.length === 0}
						<div class="px-3 py-2 text-[12px] text-qc-muted">Loading…</div>
					{:else if filteredOptions.length === 0}
						<div class="px-3 py-2 text-[12px] text-qc-muted">No matching records</div>
					{:else}
						{#each filteredOptions as option, index}
							<button
								type="button"
								class={`w-full px-3 py-2 text-left text-[12px] flex items-center gap-2 ${index === highlight ? 'bg-qc-hover text-qc-fg' : 'text-qc-subtle hover:bg-qc-hover'}`}
								onmousedown={(event) => event.preventDefault()}
								onmouseenter={() => (highlight = index)}
								onclick={() => chooseOption(option)}
							>
								<span class="min-w-0 flex-1">
									<span class="block truncate">
										{hasDistinctLabel(option) ? option.label : String(option.id)}
									</span>
									{#if hasDistinctLabel(option)}
										<span class="block truncate font-mono text-[10px] text-qc-muted"
											>{option.id}</span
										>
									{/if}
								</span>
								{#if String(option.id) === value}
									<Check size={12} class="shrink-0 text-qc-fg" />
								{/if}
							</button>
						{/each}
					{/if}
				</div>
				{#if nullable}
					<button
						type="button"
						class="w-full border-t border-qc-border px-3 py-1.5 text-left text-[12px] text-qc-muted hover:bg-qc-hover"
						onmousedown={(event) => event.preventDefault()}
						onclick={clearValue}
					>
						Clear
					</button>
				{/if}
			</div>
		{/if}
	</div>
{:else if kind === 'number'}
	<input
		bind:this={inputEl}
		type="number"
		{disabled}
		{placeholder}
		value={value}
		class={controlClass}
		oninput={(event) => onChange((event.currentTarget as HTMLInputElement).value)}
		onkeydown={handleKeydown}
		onblur={() => onCommit?.()}
	/>
{:else if kind === 'date'}
	<input
		bind:this={inputEl}
		type="date"
		{disabled}
		value={value}
		class={controlClass}
		oninput={(event) => onChange((event.currentTarget as HTMLInputElement).value)}
		onkeydown={handleKeydown}
		onblur={() => onCommit?.()}
	/>
{:else if kind === 'datetime'}
	<input
		bind:this={inputEl}
		type="datetime-local"
		{disabled}
		value={value}
		class={controlClass}
		oninput={(event) => onChange((event.currentTarget as HTMLInputElement).value)}
		onkeydown={handleKeydown}
		onblur={() => onCommit?.()}
	/>
{:else}
	<input
		bind:this={inputEl}
		type="text"
		{disabled}
		{placeholder}
		value={value}
		class={controlClass}
		oninput={(event) => onChange((event.currentTarget as HTMLInputElement).value)}
		onkeydown={handleKeydown}
		onblur={() => onCommit?.()}
	/>
{/if}
