<script lang="ts">
	import MysqlIcon from './MysqlIcon.svelte';
	import PostgresqlIcon from './PostgresqlIcon.svelte';
	import SqliteIcon from './SqliteIcon.svelte';
	import DuckdbIcon from './DuckdbIcon.svelte';
	import MongodbIcon from './MongodbIcon.svelte';
	import MssqlIcon from './MssqlIcon.svelte';
	import RedisIcon from './RedisIcon.svelte';

	let {
		type,
		size = 24,
		class: className = '',
		tone = 'brand',
	}: {
		type: 'postgres' | 'mysql' | 'sqlite' | 'duckdb' | 'mongodb' | 'mssql' | 'redis';
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
	{#if type === 'postgres'}
		<PostgresqlIcon {size} {mono} />
	{:else if type === 'mysql'}
		<MysqlIcon {size} />
	{:else if type === 'sqlite'}
		<SqliteIcon {size} {mono} />
	{:else if type === 'duckdb'}
		<DuckdbIcon {size} />
	{:else if type === 'redis'}
		<RedisIcon {size} />
	{:else if type === 'mongodb'}
		<MongodbIcon {size} />
	{:else if type === 'mssql'}
		<MssqlIcon {size} />
	{:else}
		<DuckdbIcon {size} />
	{/if}
</span>
