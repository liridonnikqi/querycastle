<script lang="ts">
	import type { DatabaseType } from '$lib/rpc';
	import MssqlIcon from './MssqlIcon.svelte';
	import MysqlIcon from './MysqlIcon.svelte';
	import PostgresqlIcon from './PostgresqlIcon.svelte';
	import SqliteIcon from './SqliteIcon.svelte';

	let {
		type,
		size = 24,
		class: className = '',
		tone = 'brand',
	}: {
		type: DatabaseType;
		size?: number;
		class?: string;
		tone?: 'brand' | 'white' | 'ink';
	} = $props();

	const mono = $derived(tone !== 'brand');
	const wrapClass = $derived(
		tone === 'white' ? 'text-white' : tone === 'ink' ? 'qc-db-mono-ink' : '',
	);
</script>

<span class={`inline-flex items-center justify-center shrink-0 ${wrapClass} ${className}`}>
	{#if type === 'mysql'}
		<MysqlIcon {size} {mono} />
	{:else if type === 'sqlite'}
		<SqliteIcon {size} {mono} />
	{:else if type === 'mssql'}
		<MssqlIcon {size} {mono} />
	{:else}
		<PostgresqlIcon {size} {mono} />
	{/if}
</span>
