<script lang="ts">
	import { Plus, X } from '@lucide/svelte';
	import type { ConnectionInput, ConnectionStatus } from '$lib/rpc';
	import DatabaseIcon from '$lib/components/ui/DatabaseIcon.svelte';
	import { connectionMetaLine } from '$lib/utils/connection';
	import { isSavedConnectionOpen, type OpenSession } from '$lib/utils/open-session';

	let {
		sessions,
		activeSessionId,
		savedConnections = [],
		onSelect,
		onClose,
		onConnectSaved,
		onNew,
		embedded = false,
	}: {
		sessions: OpenSession[];
		activeSessionId: string;
		savedConnections?: ConnectionInput[];
		onSelect: (sessionId: string) => void;
		onClose: (sessionId: string) => void;
		onConnectSaved: (connection: ConnectionInput) => void;
		onNew: () => void;
		embedded?: boolean;
	} = $props();

	let menuOpen = $state(false);
	let menuStyle = $state('');
	let scroller = $state<HTMLDivElement | null>(null);

	function label(status: ConnectionStatus) {
		return status.name?.trim() || status.database || 'Connection';
	}

	function scrollSessionIntoView(sessionId: string) {
		const container = scroller;
		if (!container) return;
		const node = container.querySelector(`[data-session-id="${CSS.escape(sessionId)}"]`);
		if (!(node instanceof HTMLElement)) return;
		const pad = 8;
		const left = node.offsetLeft;
		const right = left + node.offsetWidth;
		const viewLeft = container.scrollLeft;
		const viewRight = viewLeft + container.clientWidth;
		if (right > viewRight) {
			container.scrollLeft = right - container.clientWidth + pad;
		} else if (left < viewLeft) {
			container.scrollLeft = Math.max(0, left - pad);
		}
	}

	function onWheel(event: WheelEvent) {
		const container = scroller;
		if (!container) return;
		if (container.scrollWidth <= container.clientWidth) return;
		if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
		event.preventDefault();
		container.scrollLeft += event.deltaY;
	}

	function openMenu(event: MouseEvent) {
		if (menuOpen) {
			menuOpen = false;
			return;
		}
		const btn = event.currentTarget as HTMLElement;
		const rect = btn.getBoundingClientRect();
		const menuWidth = 280;
		const left = Math.max(8, Math.min(rect.left, window.innerWidth - menuWidth - 8));
		menuStyle = `top:${Math.round(rect.bottom)}px;left:${Math.round(left)}px`;
		menuOpen = true;
	}

	function pickSaved(connection: ConnectionInput) {
		if (isSavedConnectionOpen(connection, sessions)) return;
		menuOpen = false;
		onConnectSaved(connection);
	}

	function pickNew() {
		menuOpen = false;
		onNew();
	}

	$effect(() => {
		const id = activeSessionId;
		requestAnimationFrame(() => scrollSessionIntoView(id));
	});
</script>

<div
	class={embedded
		? 'h-7 flex items-center min-w-0 max-w-full w-max overflow-hidden relative'
		: 'h-9 flex items-center border-b border-qc-border bg-qc-panel shrink-0 min-w-0 w-full overflow-hidden relative'}
>
	<div
		bind:this={scroller}
		onwheel={onWheel}
		class="min-w-0 h-full overflow-x-auto overflow-y-hidden hide-scrollbar"
	>
		<div class="flex items-stretch h-full w-max">
			{#each sessions as session (session.id)}
				<div
					data-session-id={session.id}
					class={`tab-item group flex items-center gap-1 text-[12px] whitespace-nowrap h-full shrink-0 ${embedded ? 'px-1.5 rounded-md' : 'px-3 border-b'} ${session.id === activeSessionId ? (embedded ? 'bg-qc-hover text-qc-fg' : 'active text-qc-fg border-qc-fg') : embedded ? 'text-qc-muted hover:text-qc-subtle hover:bg-qc-hover/60' : 'text-qc-muted border-transparent hover:text-qc-subtle hover:bg-qc-hover/60'}`}
				>
					<button
						type="button"
						onclick={() => onSelect(session.id)}
						class="inline-flex items-center gap-1.5 min-w-0"
						title={`${label(session.status)} · ${session.status.database}`}
					>
						<DatabaseIcon
						type={session.status.databaseType}
						size={12}
						tone={session.status.databaseType === 'sqlite' ? 'ink' : 'brand'}
					/>
						<span class={`truncate ${embedded ? 'max-w-24' : 'max-w-40'}`}>{label(session.status)}</span>
					</button>
					<button
						type="button"
						onclick={(event) => {
							event.stopPropagation();
							onClose(session.id);
						}}
						class="w-3.5 h-3.5 opacity-50 hover:opacity-100"
						aria-label={`Disconnect ${label(session.status)}`}
					>
						<X size={12} />
					</button>
				</div>
			{/each}
		</div>
	</div>
	<button
		type="button"
		onclick={openMenu}
		class={`flex items-center justify-center text-qc-muted hover:text-qc-subtle hover:bg-qc-hover/60 shrink-0 ${embedded ? 'w-7 h-7 rounded-md' : 'w-8 h-full'}`}
		title="Open connection"
		aria-label="Open connection"
	>
		<Plus size={14} />
	</button>
</div>

{#if menuOpen}
	<button type="button" class="fixed inset-0 z-40 cursor-default" aria-label="Close connection picker" onclick={() => (menuOpen = false)}></button>
	<div
		class="fixed z-50 w-[280px] max-h-[min(360px,70vh)] overflow-y-auto rounded-sm border border-qc-border bg-qc-elevated shadow-[0_8px_24px_rgba(0,0,0,0.28)] py-1"
		style={menuStyle}
	>
		{#if savedConnections.length === 0}
			<div class="px-3 py-2 text-[12px] text-qc-muted">No saved connections.</div>
		{:else}
			{#each savedConnections as connection (connection.name)}
				{@const alreadyOpen = isSavedConnectionOpen(connection, sessions)}
				<button
					type="button"
					disabled={alreadyOpen}
					onclick={() => pickSaved(connection)}
					class={`w-full px-3 py-2 text-left flex items-center gap-2 ${alreadyOpen ? 'opacity-50 cursor-default' : 'hover:bg-qc-hover'}`}
				>
					<DatabaseIcon
					type={connection.databaseType}
					size={14}
					tone={connection.databaseType === 'sqlite' ? 'ink' : 'brand'}
				/>
					<div class="min-w-0 flex-1">
						<div class="text-[12px] text-qc-fg truncate">{connection.name}</div>
						<div class="text-[11px] text-qc-muted truncate">{connectionMetaLine(connection)}</div>
					</div>
					{#if alreadyOpen}
						<span class="text-[10px] uppercase tracking-wide text-qc-muted shrink-0">Open</span>
					{/if}
				</button>
			{/each}
		{/if}
		<div class="my-1 border-t border-qc-border-subtle"></div>
		<button
			type="button"
			onclick={pickNew}
			class="w-full px-3 py-2 text-left text-[12px] text-qc-fg hover:bg-qc-hover"
		>
			New connection…
		</button>
	</div>
{/if}
