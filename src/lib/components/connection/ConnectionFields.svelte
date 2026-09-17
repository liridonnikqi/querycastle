<script lang="ts">
	import { FolderOpen } from '$lib/icons';
	import { open } from '@tauri-apps/plugin-dialog';
	import type { ConnectionInput } from '$lib/rpc';
	import {
		connectionStringPlaceholder,
		generateConnectionString,
		withParsedConnectionString,
		withSqliteFile,
	} from '$lib/utils/connection';
	import { dialectCapabilities } from '$lib/utils/dialect';

	let {
		form,
		connectionString,
		variant = 'dialog',
		showString = true,
		passwordStored = false,
		nameInput = $bindable(null),
		onFormChange,
		onStringChange,
	}: {
		form: ConnectionInput;
		connectionString: string;
		variant?: 'hub' | 'dialog';
		showString?: boolean;
		passwordStored?: boolean;
		nameInput?: HTMLInputElement | null;
		onFormChange: (next: ConnectionInput) => void;
		onStringChange: (value: string) => void;
	} = $props();

	const isSqlite = $derived(form.databaseType === 'sqlite');
	const hub = $derived(variant === 'hub');
	const passwordPlaceholder = $derived(
		passwordStored ? 'Stored in Keychain (Leave blank to keep)' : undefined,
	);
	const inputClass = $derived(
		hub
			? 'field-input w-full h-9 px-3 text-[13px] placeholder:text-qc-muted'
			: 'ui-input h-8 w-full px-3 placeholder:text-qc-muted',
	);
	const labelClass = $derived(
		hub
			? 'text-[11px] font-medium text-qc-subtle mb-1.5'
			: 'text-[10px] font-semibold uppercase tracking-wider text-qc-muted',
	);

	function commit(next: ConnectionInput) {
		onFormChange(next);
		onStringChange(generateConnectionString(next));
	}

	function updateField<K extends keyof ConnectionInput>(
		key: K,
		value: ConnectionInput[K],
	) {
		const next: ConnectionInput = { ...form, [key]: value };
		if (key === 'ssl' && value === false) next.sslInsecure = false;
		commit(next);
	}

	async function chooseSqliteFile() {
		const selected = await open({
			multiple: false,
			directory: false,
			filters: [
				{ name: 'SQLite Database', extensions: ['db', 'sqlite', 'sqlite3'] },
				{ name: 'All Files', extensions: ['*'] },
			],
		});
		if (!selected || Array.isArray(selected)) return;
		commit(withSqliteFile(form, selected));
	}

	function handleStringInput(value: string) {
		onStringChange(value);
		const parsed = withParsedConnectionString(form, value);
		if (parsed) onFormChange(parsed);
	}
</script>

<div
	class={hub
		? 'space-y-3.5'
		: 'grid content-start grid-cols-1 gap-2.5 sm:grid-cols-2'}
>
	<div
		class={hub ? (isSqlite ? '' : 'grid grid-cols-2 gap-3') : 'sm:col-span-2'}
	>
		<label class={hub ? 'block' : 'flex flex-col gap-1 text-qc-subtle'}>
			<span class={labelClass}>{hub ? 'Name' : 'Connection Name'}</span>
			<input
				bind:this={nameInput}
				value={form.name}
				oninput={(event) => updateField('name', event.currentTarget.value)}
				class={inputClass}
				placeholder={hub ? 'My database' : undefined}
			/>
		</label>
		{#if hub && !isSqlite}
			<div>
				<div class={labelClass}>Host</div>
				<input
					value={form.host}
					oninput={(event) => updateField('host', event.currentTarget.value)}
					class={inputClass}
					placeholder="localhost"
				/>
			</div>
		{/if}
	</div>

	{#if isSqlite}
		<label
			class={hub ? 'block' : 'sm:col-span-2 flex flex-col gap-1 text-qc-subtle'}
		>
			<span class={labelClass}>{hub ? 'Database file' : 'Database Path'}</span>
			<div class="flex items-center gap-2">
				<input
					value={form.database}
					oninput={(event) =>
						updateField('database', event.currentTarget.value)}
					placeholder="C:/data/mydb.sqlite"
					class={`${inputClass} ${hub ? 'font-mono' : ''}`}
				/>
				<button
					type="button"
					onclick={chooseSqliteFile}
					class={hub
						? 'h-9 px-3 rounded-md border border-qc-border bg-qc-elevated text-qc-subtle text-[12px] inline-flex items-center gap-1.5 hover:bg-qc-hover shrink-0'
						: 'inline-flex h-8 shrink-0 items-center gap-1 rounded-md border border-qc-border bg-qc-panel px-3 text-[12px] font-medium text-qc-subtle hover:bg-qc-hover hover:text-qc-fg'}
				>
					<FolderOpen size={14} />
					{hub ? 'Open' : 'Open File'}
				</button>
			</div>
		</label>
	{:else}
		{#if !hub}
			<label class="sm:col-span-2 flex flex-col gap-1 text-qc-subtle">
				<span class={labelClass}>Host & Port</span>
				<div class="flex gap-2">
					<input
						value={form.host}
						oninput={(event) => updateField('host', event.currentTarget.value)}
						class={inputClass}
						placeholder="localhost"
					/>
					<input
						type="number"
						value={form.port}
						oninput={(event) =>
							updateField(
								'port',
								Number(event.currentTarget.value) ||
									dialectCapabilities(form.databaseType).defaultPort,
							)}
						class="ui-input h-8 w-24 shrink-0 px-3"
						title="Port"
					/>
				</div>
			</label>
			<label class="sm:col-span-2 flex flex-col gap-1 text-qc-subtle">
				<span class={labelClass}
					>Database{form.databaseType === 'postgres' ? ' — Optional' : ''}</span
				>
				<input
					value={form.database}
					oninput={(event) =>
						updateField('database', event.currentTarget.value)}
					placeholder={form.databaseType === 'postgres'
						? 'postgres (default)'
						: form.databaseType === 'mysql'
							? 'mysql (default)'
							: undefined}
					class={inputClass}
				/>
			</label>
			<label class="flex flex-col gap-1 text-qc-subtle">
				<span class={labelClass}>User</span>
				<input
					value={form.user}
					oninput={(event) => updateField('user', event.currentTarget.value)}
					class={inputClass}
				/>
			</label>
			<label class="flex flex-col gap-1 text-qc-subtle">
				<span class={labelClass}>Password</span>
				<input
					type="password"
					value={form.password}
					oninput={(event) =>
						updateField('password', event.currentTarget.value)}
					class={inputClass}
					placeholder={passwordPlaceholder}
					autocomplete="off"
				/>
				{#if passwordStored && !form.password}
					<span class="text-[11px] text-qc-muted">Using the saved password</span
					>
				{/if}
			</label>
		{:else}
			<div class="grid grid-cols-2 gap-3">
				<div>
					<div class={labelClass}>Port</div>
					<input
						value={String(form.port)}
						oninput={(event) =>
							updateField(
								'port',
								Number(event.currentTarget.value) ||
									dialectCapabilities(form.databaseType).defaultPort,
							)}
						class="{inputClass} font-mono"
					/>
				</div>
				<div>
					<div class={labelClass}>Database</div>
					<input
						value={form.database}
						oninput={(event) =>
							updateField('database', event.currentTarget.value)}
						class={inputClass}
					/>
				</div>
			</div>
			<div class="grid grid-cols-2 gap-3">
				<div>
					<div class={labelClass}>User</div>
					<input
						value={form.user}
						oninput={(event) => updateField('user', event.currentTarget.value)}
						class={inputClass}
					/>
				</div>
				<div>
					<div class={labelClass}>Password</div>
					<input
						type="password"
						value={form.password}
						oninput={(event) =>
							updateField('password', event.currentTarget.value)}
						class={inputClass}
						placeholder={passwordPlaceholder}
						autocomplete="off"
					/>
					{#if passwordStored && !form.password}
						<div class="mt-1 text-[11px] text-qc-muted">
							Using the saved password
						</div>
					{/if}
				</div>
			</div>
		{/if}
		<div class={hub ? 'space-y-2' : 'sm:col-span-2 flex flex-col gap-2'}>
			<label class="flex items-center gap-2 text-[12px] text-qc-subtle">
				<input
					type="checkbox"
					class="qc-check"
					checked={form.ssl}
					onchange={(event) => updateField('ssl', event.currentTarget.checked)}
				/>
				Use SSL
			</label>
			<label class="flex items-center gap-2 text-[12px] text-qc-subtle">
				<input
					type="checkbox"
					class="qc-check"
					checked={form.sslInsecure ?? false}
					disabled={!form.ssl}
					onchange={(event) =>
						updateField('sslInsecure', event.currentTarget.checked)}
				/>
				Allow insecure TLS (self-signed)
			</label>
		</div>
	{/if}

	{#if showString}
		<div class={hub ? '' : 'sm:col-span-2 mt-2 pt-1'}>
			{#if !hub}
				<div class="relative py-2">
					<div class="absolute inset-0 flex items-center" aria-hidden="true">
						<div class="w-full border-t border-qc-border"></div>
					</div>
					<div class="relative flex justify-center">
						<span
							class="bg-qc-elevated px-2 text-[10px] font-semibold uppercase tracking-wider text-qc-muted"
							>Or paste a connection string to auto-fill</span
						>
					</div>
				</div>
			{/if}
			<input
				value={connectionString}
				oninput={(event) => handleStringInput(event.currentTarget.value)}
				placeholder={connectionStringPlaceholder(form.databaseType)}
				class={hub ? `${inputClass} font-mono mt-2` : `${inputClass} mt-1`}
			/>
		</div>
	{/if}
</div>
