<script lang="ts">
	import {
		Braces,
		Calendar,
		CalendarClock,
		Fingerprint,
		Hash,
		KeyRound,
		Link2,
		ToggleLeft,
		Type,
	} from '@lucide/svelte';
	import type { GridColumnMeta } from '$lib/utils/grid-editors';

	let { meta }: { meta: GridColumnMeta | undefined } = $props();

	const Icon = $derived.by(() => {
		const typeName = meta?.dataType?.toLowerCase() ?? '';
		if (meta?.fk) return Link2;
		if (meta?.isPrimary) return KeyRound;
		if (/\b(json|jsonb|xml)\b/.test(typeName)) return Braces;
		if (/\b(uuid|uniqueidentifier)\b/.test(typeName)) return Fingerprint;
		if (meta?.kind === 'boolean') return ToggleLeft;
		if (meta?.kind === 'datetime') return CalendarClock;
		if (meta?.kind === 'date') return Calendar;
		if (meta?.kind === 'number') return Hash;
		return Type;
	});

	const label = $derived.by(() => {
		if (meta?.fk) return `Foreign key · ${meta.dataType}`;
		if (meta?.isPrimary) return `Primary key · ${meta.dataType}`;
		return meta?.dataType ?? 'text';
	});
</script>

<span class="inline-flex items-center" title={label}>
	<Icon size={11} strokeWidth={1.75} class="shrink-0 text-qc-muted" />
</span>
