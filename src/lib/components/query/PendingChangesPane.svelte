<script lang="ts">
	import { diffText, type PendingChangeCard } from '$lib/utils/pending-changes';

	let {
		open = true,
		changeCount,
		cards,
		sqlPreview,
		syncing = false,
		onClose,
		onClear,
		onCommit,
	}: {
		open?: boolean;
		changeCount: number;
		cards: PendingChangeCard[];
		sqlPreview: string;
		syncing?: boolean;
		onClose: () => void;
		onClear: () => void;
		onCommit: () => void;
	} = $props();

	let view = $state<'visual' | 'sql'>('visual');
</script>

{#if open}
	<aside class="w-[320px] shrink-0 border-l border-qc-border bg-qc-panel flex flex-col min-h-0">
		<div class="h-11 px-3 border-b border-qc-border flex items-center justify-between gap-2 shrink-0">
			<div class="text-sm font-semibold text-qc-fg">Pending Changes</div>
			<div class="flex items-center gap-1">
				<div class="flex rounded-md border border-qc-border p-0.5 text-[11px]">
					<button
						type="button"
						class={`h-6 px-2 rounded ${view === 'visual' ? 'bg-qc-fg text-qc-primary-fg' : 'text-qc-muted hover:bg-qc-hover'}`}
						onclick={() => (view = 'visual')}
					>
						Visual
					</button>
					<button
						type="button"
						class={`h-6 px-2 rounded ${view === 'sql' ? 'bg-qc-fg text-qc-primary-fg' : 'text-qc-muted hover:bg-qc-hover'}`}
						onclick={() => (view = 'sql')}
					>
						SQL
					</button>
				</div>
				<button
					type="button"
					class="h-6 w-6 rounded text-qc-muted hover:text-qc-fg hover:bg-qc-hover"
					onclick={onClose}
					aria-label="Close pending changes"
				>
					×
				</button>
			</div>
		</div>
		<div class="flex-1 overflow-y-auto p-3 space-y-2 min-h-0">
			{#if changeCount === 0}
				<div class="text-xs text-qc-muted px-1 py-6 text-center">No pending changes.</div>
			{:else if view === 'sql'}
				<pre class="font-mono-code text-[11px] text-qc-data whitespace-pre-wrap break-words rounded-md border border-qc-border bg-qc-bg p-2.5">{sqlPreview || 'Nothing to commit.'}</pre>
			{:else}
				{#each cards as card (card.id)}
					{@const afterEmpty = card.after === ''}
					{@const beforeEmpty = card.before === ''}
					{@const hunks =
						card.kind === 'update' && card.before != null && card.after != null && !afterEmpty && !beforeEmpty
							? diffText(card.before, card.after)
							: []}
					<div class="rounded-md border border-qc-border overflow-hidden">
						<div class="px-2.5 py-1.5 bg-qc-elevated border-b border-qc-border flex items-center gap-2">
							<span
								class={`h-4 min-w-4 px-1 rounded text-[10px] font-bold leading-4 text-center ${card.kind === 'update' ? 'bg-qc-amber-bg text-qc-amber' : card.kind === 'insert' ? 'bg-emerald-500/15 text-emerald-400' : 'bg-qc-danger/15 text-qc-danger'}`}
							>
								{card.badge}
							</span>
							<div class="truncate text-[11px] text-qc-muted" title={card.title}>{card.title}</div>
						</div>
						<div class="pending-diff">
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
			{/if}
		</div>
		<div class="h-12 px-3 border-t border-qc-border flex items-center justify-between gap-2 shrink-0 bg-qc-elevated">
			<button
				type="button"
				class="text-xs text-qc-muted hover:text-qc-fg disabled:opacity-40"
				disabled={changeCount === 0 || syncing}
				onclick={onClear}
			>
				Clear all
			</button>
			<button
				type="button"
				class="h-8 px-3 btn-primary text-xs font-medium disabled:opacity-60"
				disabled={changeCount === 0 || syncing}
				onclick={onCommit}
			>
				{syncing ? 'Committing…' : `Commit all (${changeCount})`}
			</button>
		</div>
	</aside>
{/if}
