<script lang="ts">
	import type { DatabaseType } from '$lib/rpc';
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
		tone === 'white'
			? type === 'mysql'
				? 'qc-db-tone-white'
				: 'text-white'
			: tone === 'ink'
				? type === 'mysql'
					? 'qc-db-tone-ink'
					: 'qc-db-mono-ink'
				: '',
	);
</script>

<span class={`inline-flex ${wrapClass} ${className}`}>
	{#if type === 'mysql'}
		<MysqlIcon {size} />
	{:else if type === 'sqlite'}
		<SqliteIcon {size} {mono} />
	{:else}
		<PostgresqlIcon {size} {mono} />
	{/if}
</span>
