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
	}: {
		type: 'postgres' | 'mysql' | 'sqlite' | 'duckdb' | 'mongodb' | 'mssql' | 'redis';
		size?: number;
		class?: string;
	} = $props();

</script>

{#if type === 'postgres'}
	<PostgresqlIcon {size} class={className} />
{:else if type === 'mysql'}
	<MysqlIcon {size} class={className} />
{:else if type === 'sqlite'}
	<SqliteIcon {size} class={className} />
{:else if type === 'duckdb'}
	<DuckdbIcon {size} class={className} />
{:else if type === 'redis'}
	<RedisIcon {size} class={className} />
{:else if type === 'mongodb'}
	<MongodbIcon {size} class={className} />
{:else if type === 'mssql'}
	<MssqlIcon {size} class={className} />
{:else}
	<DuckdbIcon {size} class={className} />
{/if}

