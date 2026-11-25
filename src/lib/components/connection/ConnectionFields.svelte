<script lang="ts">
	import { FolderOpen } from '$lib/icons';
	import { open } from '@tauri-apps/plugin-dialog';
	import type { ConnectionInput } from '$lib/rpc';
	import DatabaseIcon from '$lib/components/ui/DatabaseIcon.svelte';
	import {
		connectionStringPlaceholder,
		generateConnectionString,
		withParsedConnectionString,
		withSqliteFile,
	} from '$lib/utils/connection';
	import { dialectCapabilities, displayDatabaseName, engineDisplayName } from '$lib/utils/dialect';

	export type ConnectionTab = 'general' | 'ssh' | 'url';

	let {
		tab = 'general',
		form,
		connectionString,
		passwordStored = false,
		nameInput = $bindable(null),
		onFormChange,
		onStringChange,
	}: {
		tab?: ConnectionTab;
		form: ConnectionInput;
		connectionString: string;
		passwordStored?: boolean;
		nameInput?: HTMLInputElement | null;
		onFormChange: (next: ConnectionInput) => void;
		onStringChange: (value: string) => void;
	} = $props();

	const isSqlite = $derived(form.databaseType === 'sqlite');
	const passwordPlaceholder = $derived(
		passwordStored ? 'Saved in keychain' : undefined,
	);
	const inputClass =
		'field-input w-full h-9 px-3 text-[13px] placeholder:text-qc-muted';
	const labelClass = 'text-[11.5px] font-medium text-qc-subtle';

	let sshAuth = $state<'password' | 'key'>('password');
	let lastAuthInitKey = $state('');

	$effect(() => {
		const key = `${form.name}::${form.sshEnabled}`;
		if (key !== lastAuthInitKey) {
			lastAuthInitKey = key;
			sshAuth = form.sshPrivateKeyPath?.trim() ? 'key' : 'password';
		}
	});

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

	function handlePortInput(
		key: 'port' | 'sshPort',
		event: Event & { currentTarget: HTMLInputElement },
	) {
		const digits = event.currentTarget.value.replace(/\D+/g, '');
		event.currentTarget.value = digits;
		const parsed = Number(digits);
		if (parsed > 0 && parsed <= 65535) {
			updateField(key, parsed);
		}
	}

	function handlePortBlur(
		key: 'port' | 'sshPort',
		fallback: number,
		event: FocusEvent & { currentTarget: HTMLInputElement },
	) {
		const parsed = Number(event.currentTarget.value);
		if (!parsed || parsed <= 0 || parsed > 65535) {
			updateField(key, fallback);
			event.currentTarget.value = String(fallback);
		}
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

	async function chooseSshKey() {
		const selected = await open({
			multiple: false,
			directory: false,
			filters: [
				{ name: 'Private key', extensions: ['pem', 'key', 'id_rsa', 'id_ed25519'] },
				{ name: 'All Files', extensions: ['*'] },
			],
		});
		if (!selected || Array.isArray(selected)) return;
		updateField('sshPrivateKeyPath', selected);
	}

	function handleStringInput(value: string) {
		onStringChange(value);
		const parsed = withParsedConnectionString(form, value);
		if (parsed) onFormChange(parsed);
	}

	async function pasteClipboard() {
		try {
			const clip = await navigator.clipboard.readText();
			if (clip) handleStringInput(clip.trim());
		} catch {
			// Ignore clipboard permission denials in restricted webviews.
		}
	}

	const summaryTarget = $derived(
		isSqlite
			? displayDatabaseName(form.database, 'sqlite') || 'No file selected'
			: `${form.host || 'localhost'}:${form.port || dialectCapabilities(form.databaseType).defaultPort}`,
	);
</script>

{#if tab === 'general'}
	{#if isSqlite}
		<div class="space-y-3">
			<label class="flex flex-col gap-1.5 min-w-0">
				<span class={labelClass}>Connection name</span>
				<input
					bind:this={nameInput}
					value={form.name}
					oninput={(event) => updateField('name', event.currentTarget.value)}
					class={inputClass}
					placeholder="My SQLite database"
				/>
			</label>

			<label class="flex flex-col gap-1.5 min-w-0">
				<span class={labelClass}>Database file</span>
				<div class="flex items-center gap-2">
					<input
						value={form.database}
						oninput={(event) => updateField('database', event.currentTarget.value)}
						placeholder="C:/data/mydb.sqlite"
						class={`${inputClass} font-mono`}
					/>
					<button
						type="button"
						onclick={chooseSqliteFile}
						class="btn-secondary h-9 px-3 text-[12.5px] font-medium inline-flex items-center gap-1.5 shrink-0"
					>
						<FolderOpen size={14} />
						Browse…
					</button>
				</div>
			</label>

			<div class="pt-3 border-t border-qc-border-subtle flex items-center justify-between gap-3">
				<label
					data-tip="Prevent write queries (safe for production)"
					class="inline-flex items-center gap-2 text-[12px] text-qc-subtle select-none cursor-pointer"
				>
					<input
						type="checkbox"
						class="qc-form-check"
						checked={form.readOnly ?? false}
						onchange={(event) => updateField('readOnly', event.currentTarget.checked)}
					/>
					<span>Read-only</span>
				</label>
				<span class="text-[11px] text-qc-muted">Creates file if missing</span>
			</div>
		</div>
	{:else}
		<div class="space-y-3">
			<label class="flex flex-col gap-1.5 min-w-0">
				<span class={labelClass}>Connection name</span>
				<input
					bind:this={nameInput}
					value={form.name}
					oninput={(event) => updateField('name', event.currentTarget.value)}
					class={inputClass}
					placeholder="Production DB"
				/>
			</label>

			<div class="grid grid-cols-[1fr_7.5rem] gap-2.5">
				<label class="flex flex-col gap-1.5 min-w-0">
					<span class={labelClass}>Host</span>
					<input
						value={form.host}
						oninput={(event) => updateField('host', event.currentTarget.value)}
						class={inputClass}
						placeholder="localhost"
					/>
				</label>
				<label class="flex flex-col gap-1.5 min-w-0">
					<span class={labelClass}>Port</span>
					<input
						type="text"
						inputmode="numeric"
						value={String(form.port)}
						oninput={(event) => handlePortInput('port', event)}
						onblur={(event) =>
							handlePortBlur(
								'port',
								dialectCapabilities(form.databaseType).defaultPort,
								event,
							)}
						class={`${inputClass} font-mono`}
					/>
				</label>
			</div>

			<div class="grid grid-cols-2 gap-2.5">
				<label class="flex flex-col gap-1.5 min-w-0">
					<span class={labelClass}>User</span>
					<input
						value={form.user}
						oninput={(event) => updateField('user', event.currentTarget.value)}
						class={inputClass}
					/>
				</label>
				<label class="flex flex-col gap-1.5 min-w-0">
					<span class={labelClass}>Password</span>
					<input
						type="password"
						value={form.password}
						oninput={(event) => updateField('password', event.currentTarget.value)}
						class={inputClass}
						placeholder={passwordPlaceholder}
						autocomplete="off"
					/>
				</label>
			</div>

			<label class="flex flex-col gap-1.5 min-w-0">
				<span class="flex items-baseline justify-between gap-2">
					<span class={labelClass}>Database</span>
					{#if form.databaseType === 'postgres'}
						<span class="text-[11px] text-qc-muted">Optional</span>
					{/if}
				</span>
				<input
					value={form.database}
					oninput={(event) => updateField('database', event.currentTarget.value)}
					placeholder={form.databaseType === 'postgres'
						? 'postgres'
						: form.databaseType === 'mysql'
							? 'mysql'
							: 'master'}
					class={inputClass}
				/>
			</label>

			<div class="pt-3 border-t border-qc-border-subtle flex flex-wrap items-center justify-between gap-3">
				<div class="flex items-center gap-4">
					<label
						class="inline-flex items-center gap-2 text-[12px] text-qc-subtle select-none cursor-pointer"
					>
						<input
							type="checkbox"
							class="qc-form-check"
							checked={form.ssl}
							onchange={(event) => updateField('ssl', event.currentTarget.checked)}
						/>
						<span>Use SSL</span>
					</label>
					<label
						data-tip="Allow insecure or self-signed TLS certificates"
						class={`inline-flex items-center gap-2 text-[12px] text-qc-subtle select-none ${
							form.ssl ? 'cursor-pointer' : 'opacity-45 cursor-not-allowed'
						}`}
					>
						<input
							type="checkbox"
							class="qc-form-check"
							checked={form.sslInsecure ?? false}
							disabled={!form.ssl}
							onchange={(event) =>
								updateField('sslInsecure', event.currentTarget.checked)}
						/>
						<span>Self-signed TLS</span>
					</label>
				</div>

				<label
					data-tip="Prevent write queries (safe for production)"
					class="inline-flex items-center gap-2 text-[12px] text-qc-subtle select-none cursor-pointer"
				>
					<input
						type="checkbox"
						class="qc-form-check"
						checked={form.readOnly ?? false}
						onchange={(event) => updateField('readOnly', event.currentTarget.checked)}
					/>
					<span>Read-only</span>
				</label>
			</div>
		</div>
	{/if}
{:else if tab === 'ssh' && !isSqlite}
	<div class="space-y-3">
		<label
			class="pb-3 border-b border-qc-border-subtle flex items-center justify-between gap-3 cursor-pointer select-none"
		>
			<div class="min-w-0">
				<div class="text-[12.5px] font-medium text-qc-fg">Enable SSH tunnel</div>
				<div class="text-[11px] text-qc-muted mt-0.5">
					Route database traffic through an SSH bastion host
				</div>
			</div>
			<input
				type="checkbox"
				class="qc-form-check"
				checked={form.sshEnabled ?? false}
				onchange={(event) => updateField('sshEnabled', event.currentTarget.checked)}
			/>
		</label>

		<div
			class={`space-y-3 pt-1 transition-opacity ${
				form.sshEnabled ? '' : 'opacity-45 pointer-events-none'
			}`}
		>
			<div class="grid grid-cols-[1fr_6.5rem] gap-2.5">
				<label class="flex flex-col gap-1.5 min-w-0">
					<span class={labelClass}>SSH host</span>
					<input
						value={form.sshHost ?? ''}
						oninput={(event) => updateField('sshHost', event.currentTarget.value)}
						class={inputClass}
						placeholder="bastion.example.com"
					/>
				</label>
				<label class="flex flex-col gap-1.5 min-w-0">
					<span class={labelClass}>SSH port</span>
					<input
						type="text"
						inputmode="numeric"
						value={String(form.sshPort ?? 22)}
						oninput={(event) => handlePortInput('sshPort', event)}
						onblur={(event) => handlePortBlur('sshPort', 22, event)}
						class={`${inputClass} font-mono`}
					/>
				</label>
			</div>

			<div class="grid grid-cols-2 gap-2.5 items-end">
				<label class="flex flex-col gap-1.5 min-w-0">
					<span class={labelClass}>SSH user</span>
					<input
						value={form.sshUser ?? ''}
						oninput={(event) => updateField('sshUser', event.currentTarget.value)}
						class={inputClass}
						placeholder="ubuntu"
					/>
				</label>
				<div class="flex flex-col gap-1.5 min-w-0">
					<span class={labelClass}>Authentication</span>
					<div class="qc-segmented w-full grid grid-cols-2 h-9">
						<button
							type="button"
							class={`qc-seg-btn justify-center h-[30px] ${sshAuth === 'password' ? 'is-on' : ''}`}
							onclick={() => (sshAuth = 'password')}
						>
							Password
						</button>
						<button
							type="button"
							class={`qc-seg-btn justify-center h-[30px] ${sshAuth === 'key' ? 'is-on' : ''}`}
							onclick={() => (sshAuth = 'key')}
						>
							Private Key
						</button>
					</div>
				</div>
			</div>

			{#if sshAuth === 'password'}
				<label class="flex flex-col gap-1.5 min-w-0">
					<span class={labelClass}>SSH password</span>
					<input
						type="password"
						value={form.sshPassword ?? ''}
						oninput={(event) => updateField('sshPassword', event.currentTarget.value)}
						class={inputClass}
						placeholder={passwordPlaceholder}
						autocomplete="off"
					/>
				</label>
			{:else}
				<div class="grid grid-cols-[1.4fr_1fr] gap-2.5">
					<label class="flex flex-col gap-1.5 min-w-0">
						<span class={labelClass}>Private key</span>
						<div class="flex items-center gap-1.5">
							<input
								value={form.sshPrivateKeyPath ?? ''}
								oninput={(event) =>
									updateField('sshPrivateKeyPath', event.currentTarget.value)}
								class={`${inputClass} font-mono`}
								placeholder="~/.ssh/id_ed25519"
							/>
							<button
								type="button"
								onclick={chooseSshKey}
								aria-label="Browse private key"
								data-tip="Browse private key"
								class="btn-secondary w-9 h-9 inline-flex items-center justify-center shrink-0"
							>
								<FolderOpen size={14} />
							</button>
						</div>
					</label>
					<label class="flex flex-col gap-1.5 min-w-0">
						<span class={labelClass}>Passphrase</span>
						<input
							type="password"
							value={form.sshKeyPassphrase ?? ''}
							oninput={(event) =>
								updateField('sshKeyPassphrase', event.currentTarget.value)}
							class={inputClass}
							placeholder={passwordPlaceholder}
							autocomplete="off"
						/>
					</label>
				</div>
			{/if}
		</div>
	</div>
{:else}
	<div class="space-y-3">
		<div>
			<div class="flex items-center justify-between mb-1.5">
				<label class={labelClass} for="modal-connection-url">
					Connection URL
				</label>
				<button
					type="button"
					onclick={pasteClipboard}
					class="btn-secondary h-7 px-2.5 text-[11.5px] font-medium"
				>
					Paste clipboard
				</button>
			</div>
			<textarea
				id="modal-connection-url"
				rows={3}
				value={connectionString}
				oninput={(event) => handleStringInput(event.currentTarget.value)}
				placeholder={connectionStringPlaceholder(form.databaseType)}
				class="field-input w-full p-2.5 text-[12.5px] font-mono leading-relaxed placeholder:text-qc-muted resize-none"
			></textarea>
			<p class="mt-1.5 text-[11px] text-qc-muted">
				Supports postgres://, mysql://, sqlserver://, sqlite://, and ADO.NET strings. Changes sync live with General fields.
			</p>
		</div>

		<div
			class="rounded-lg border border-qc-border-subtle bg-qc-panel px-3 py-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-[11.5px]"
		>
			<span class="inline-flex items-center gap-1.5 font-medium text-qc-fg">
				<DatabaseIcon
					type={form.databaseType}
					size={13}
					tone={isSqlite ? 'ink' : 'brand'}
				/>
				{engineDisplayName(form.databaseType)}
			</span>
			<span class="text-qc-muted font-mono truncate max-w-[200px]">{summaryTarget}</span>
			{#if !isSqlite && form.database}
				<span class="text-qc-subtle font-mono">db: {form.database}</span>
			{/if}
			{#if !isSqlite && form.user}
				<span class="text-qc-muted font-mono">user: {form.user}</span>
			{/if}
		</div>
	</div>
{/if}
