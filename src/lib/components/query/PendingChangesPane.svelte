<script lang="ts">
	import { Loader2, X } from '$lib/icons';
	import { diffText, type PendingChangeCard } from '$lib/utils/pending-changes';

	let {
		open = true,
		changeCount,
		cards,
		sqlPreview,
		syncing = false,
		canUndo = false,
		onClose,
		onClear,
		onUndo,
		onCommit,
	}: {
		open?: boolean;
		changeCount: number;
		cards: PendingChangeCard[];
		sqlPreview: string;
		syncing?: boolean;
		canUndo?: boolean;
		onClose: () => void;
		onClear: () => void;
		onUndo?: () => void;
		onCommit: () => void;
	} = $props();

	let view = $state<'visual' | 'sql'>('visual');
</script>

{#if open}
	<aside class="w-[clamp(300px,28vw,350px)] shrink-0 border-l border-qc-border bg-qc-panel flex flex-col min-h-0">
		<div class="h-9 px-3 border-b border-qc-border flex items-center justify-between gap-2 shrink-0">
			<div class="text-[12px] font-semibold text-qc-fg truncate">Pending Changes</div>
			<div class="flex items-center gap-1 shrink-0">
				<div class="qc-segmented">
					<button
						type="button"
						class={`qc-seg-btn ${view === 'visual' ? 'is-on' : ''}`}
						onclick={() => (view = 'visual')}
					>
						Visual
					</button>
					<button
						type="button"
						class={`qc-seg-btn ${view === 'sql' ? 'is-on' : ''}`}
						onclick={() => (view = 'sql')}
					>
						SQL
					</button>
				</div>
				<button
					type="button"
					class="w-6 h-6 rounded flex items-center justify-center text-qc-muted hover:text-qc-fg hover:bg-qc-hover shrink-0"
					onclick={onClose}
					aria-label="Close pending changes"
				>
					<X size={13} />
				</button>
			</div>
		</div>

		<div class="flex-1 overflow-y-auto min-h-0">
			{#if changeCount === 0}
				<div class="text-xs text-qc-muted px-3 py-8 text-center">No pending changes.</div>
			{:else if view === 'sql'}
				<pre class="px-3 py-3 font-mono-code text-[11px] leading-relaxed text-qc-data whitespace-pre-wrap break-words select-text">{sqlPreview || 'Nothing to commit.'}</pre>
			{:else}
				<div class="divide-y divide-qc-border-subtle">
					{#each cards as card (card.id)}
						{@const afterEmpty = card.after === ''}
						{@const beforeEmpty = card.before === ''}
						{@const hunks =
							card.kind === 'update' && card.before != null && card.after != null && !afterEmpty && !beforeEmpty
								? diffText(card.before, card.after)
								: []}
						<div>
							<div class="px-3 pt-2.5 pb-1.5 flex items-center gap-2">
								<span
									class={`h-4 min-w-4 px-1 rounded text-[10px] font-bold leading-4 text-center shrink-0 ${
										card.kind === 'update'
											? 'bg-qc-amber-bg text-qc-amber'
											: card.kind === 'insert'
												? 'bg-emerald-500/15 text-emerald-400'
												: 'bg-qc-danger/15 text-qc-danger'
									}`}
								>
									{card.badge}
								</span>
								<div class="truncate text-[11px] text-qc-muted" data-tip={card.title}>{card.title}</div>
							</div>
							<div class="pending-diff pb-2">
								{#if card.kind === 'update' && hunks.length > 0}
									<div class="pending-diff-line pending-diff-del">
										<span class="pending-diff-sign">-</span>{#each hunks as hunk}{#if hunk.kind !== 'add'}<span class={hunk.kind === 'del' ? 'pending-diff-mark-del' : ''}>{hunk.text}</span>{/if}{/each}
									</div>
									<div class="pending-diff-line pending-diff-add">
										<span class="pending-diff-sign">+</span>{#each hunks as hunk}{#if hunk.kind !== 'del'}<span class={hunk.kind === 'add' ? 'pending-diff-mark-add' : ''}>{hunk.text}</span>{/if}{/each}
									</div>
								{:else}
									{#if card.before != null}
										<div class="pending-diff-line pending-diff-del">
											<span class="pending-diff-sign">-</span>{#if beforeEmpty}<span class="italic opacity-80">(empty)</span>{:else}{card.before}{/if}
										</div>
									{/if}
									{#if card.after != null}
										<div class="pending-diff-line pending-diff-add">
											<span class="pending-diff-sign">+</span>{#if afterEmpty}<span class="italic opacity-80">(empty)</span>{:else}{card.after}{/if}
										</div>
									{/if}
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<div class="h-10 px-3 border-t border-qc-border-subtle flex items-center justify-between gap-2 shrink-0 bg-qc-panel">
			<div class="flex items-center gap-3">
				<button
					type="button"
					class="text-xs text-qc-muted hover:text-qc-fg disabled:opacity-40"
					disabled={!canUndo || syncing}
					onclick={onUndo}
				>
					Undo
				</button>
				<button
					type="button"
					class="text-xs text-qc-muted hover:text-qc-fg disabled:opacity-40"
					disabled={changeCount === 0 || syncing}
					onclick={onClear}
				>
					Clear
				</button>
			</div>
			<button
				type="button"
				class="h-7 px-3 btn-primary text-xs font-medium inline-flex items-center justify-center gap-1.5 disabled:opacity-60"
				disabled={changeCount === 0 || syncing}
				onclick={onCommit}
			>
				{#if syncing}
					<Loader2 size={12} class="animate-spin" />
					Committing…
				{:else}
					Commit ({changeCount})
				{/if}
			</button>
		</div>
	</aside>
{/if}
