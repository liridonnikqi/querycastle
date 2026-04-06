<script lang="ts">
	import { X, Shield, DatabaseZap, FolderOpen } from "@lucide/svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import type { ConnectionInput } from "$lib/rpc";

	let {
		visible,
		editing,
		mode,
		connectionForm,
		connectionStringInput,
		testConnectionMessage,
		testConnectionOk,
		isTestingConnection,
		isConnecting,
		onClose,
		onModeChange,
		onConnectionFormChange,
		onConnectionStringChange,
		onTest,
		onSaveAndConnect,
	}: {
		visible: boolean;
		editing: boolean;
		mode: "fields" | "string";
		connectionForm: ConnectionInput;
		connectionStringInput: string;
		testConnectionMessage: string;
		testConnectionOk: boolean;
		isTestingConnection: boolean;
		isConnecting: boolean;
		onClose: () => void;
		onModeChange: (mode: "fields" | "string") => void;
		onConnectionFormChange: (next: ConnectionInput) => void;
		onConnectionStringChange: (value: string) => void;
		onTest: () => void;
		onSaveAndConnect: () => void;
	} = $props();

	function updateField<K extends keyof ConnectionInput>(key: K, value: ConnectionInput[K]) {
		onConnectionFormChange({ ...connectionForm, [key]: value });
	}

	function defaultNameForType(databaseType: ConnectionInput["databaseType"]) {
		if (databaseType === "mysql") return "local_mysql";
		if (databaseType === "sqlite") return "local_sqlite";
		return "local_pg";
	}

	async function chooseSqliteFile() {
		const selected = await open({
			multiple: false,
			directory: false,
			filters: [
				{ name: "SQLite Database", extensions: ["db", "sqlite", "sqlite3"] },
				{ name: "All Files", extensions: ["*"] },
			],
		});
		if (!selected || Array.isArray(selected)) return;

		const normalized = selected.replaceAll("\\", "/");
		const fileName = normalized.split("/").pop() ?? "";
		const nextName =
			!connectionForm.name.trim() || connectionForm.name === "local_sqlite"
				? fileName.replace(/\.(sqlite|sqlite3|db)$/i, "") || connectionForm.name
				: connectionForm.name;

		onConnectionFormChange({
			...connectionForm,
			database: selected,
			name: nextName,
		});
	}

	const engineLabel = $derived.by(() =>
		connectionForm.databaseType === "mysql"
			? "MySQL"
			: connectionForm.databaseType === "sqlite"
				? "SQLite"
				: "PostgreSQL",
	);
	const isSqlite = $derived(connectionForm.databaseType === "sqlite");
</script>

{#if visible}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
		<div class="w-full max-w-[680px] overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-[0_20px_50px_rgba(15,23,42,0.18)]">
			<div class="border-b border-slate-200 bg-white px-5 py-3 text-slate-900">
				<div class="flex items-start justify-between gap-3">
					<div>
						<h3 class="inline-flex items-center gap-2 text-[15px] font-semibold text-slate-900">
							<span class="inline-flex h-7 w-7 items-center justify-center rounded-md border border-slate-200 bg-slate-100 text-slate-700">
								<DatabaseZap size={16} />
							</span>
							{editing
								? `Edit ${engineLabel} Connection`
								: `New ${engineLabel} Connection`}
						</h3>
					</div>
					<button
						aria-label="Close modal"
						title="Close"
						onclick={onClose}
						class="flex h-8 w-8 items-center justify-center rounded-md text-slate-400 hover:bg-slate-100 hover:text-slate-700"
					>
						<X size={16} />
					</button>
				</div>
			</div>

			<div class="space-y-4 p-5 text-[13px]">
				<label class="flex flex-col gap-1 text-slate-600">
					<span class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500">Database Type</span>
					<select
						value={connectionForm.databaseType}
						onchange={(e) => {
							const nextType = (e.currentTarget as HTMLSelectElement).value as ConnectionInput["databaseType"];
							if (nextType === connectionForm.databaseType) return;
							const prevType = connectionForm.databaseType;
							const prevDefaultName = defaultNameForType(prevType);
							const nextDefaultName = defaultNameForType(nextType);
							const shouldUseDefaultName =
								!connectionForm.name.trim() || connectionForm.name === prevDefaultName;
							const nextPort = nextType === "mysql" ? 3306 : nextType === "sqlite" ? 0 : 5432;
							const nextUser = nextType === "mysql" ? "root" : nextType === "sqlite" ? "" : "postgres";
							const nextDatabase = nextType === "mysql" ? "mysql" : nextType === "sqlite" ? "main" : "postgres";
							onConnectionFormChange({
								...connectionForm,
								databaseType: nextType,
								name: shouldUseDefaultName ? nextDefaultName : connectionForm.name,
								port: nextPort,
								user: nextUser,
								password: nextType === "sqlite" ? "" : connectionForm.password,
								database: nextDatabase,
								host: nextType === "sqlite" ? "" : connectionForm.host || "localhost",
								ssl: nextType === "sqlite" ? false : connectionForm.ssl,
							});
						}}
						class="ui-input h-9 bg-white px-3"
					>
						<option value="postgres">PostgreSQL</option>
						<option value="mysql">MySQL</option>
						<option value="sqlite">SQLite</option>
					</select>
				</label>

				<div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
					<button
						onclick={() => onModeChange("fields")}
						class={`rounded-lg border px-3 py-2 text-left transition-colors ${
							(isSqlite || mode === "fields")
								? "border-slate-300 bg-slate-50 text-slate-900"
								: "border-slate-200 bg-white text-slate-600 hover:bg-slate-50"
						}`}
					>
						<p class="text-[12px] font-semibold">Use Fields</p>
						<p class="mt-0.5 text-[11px] text-slate-500">{isSqlite ? "Database file path" : "Host, port, database, user, password"}</p>
					</button>
					{#if !isSqlite}
						<button
							onclick={() => onModeChange("string")}
							class={`rounded-lg border px-3 py-2 text-left transition-colors ${
								mode === "string"
									? "border-slate-300 bg-slate-50 text-slate-900"
									: "border-slate-200 bg-white text-slate-600 hover:bg-slate-50"
							}`}
						>
							<p class="text-[12px] font-semibold">Use Connection String</p>
							<p class="mt-0.5 text-[11px] text-slate-500">{`Paste a full ${engineLabel} URL`}</p>
						</button>
					{/if}
				</div>

				<div class="grid min-h-[248px] content-start grid-cols-1 gap-3 sm:grid-cols-2">
					<label class="sm:col-span-2 flex flex-col gap-1 text-slate-600">
						<span class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500">Connection Name</span>
						<input
							value={connectionForm.name}
							oninput={(e) =>
								updateField("name", (e.currentTarget as HTMLInputElement).value)}
							class="ui-input h-9 bg-white px-3"
						/>
					</label>

					{#if mode === "string" && !isSqlite}
						<label class="sm:col-span-2 flex flex-col gap-1 text-slate-600">
							<span class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500">Connection String</span>
							<input
								value={connectionStringInput}
								oninput={(e) =>
									onConnectionStringChange((e.currentTarget as HTMLInputElement).value)}
								placeholder={connectionForm.databaseType === "mysql"
									? "mysql://root:password@host:3306/dbname"
									: connectionForm.databaseType === "sqlite"
										? "sqlite:///absolute/path/to/database.db"
										: "postgresql://postgres:password@host:5432/dbname"}
								class="ui-input h-9 bg-white px-3 font-mono-code text-[12px]"
							/>
						</label>
					{:else}
						{#if connectionForm.databaseType !== "sqlite"}
							<label class="flex flex-col gap-1 text-slate-600">
								<span class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500">Host</span>
								<input
									value={connectionForm.host}
									oninput={(e) =>
										updateField("host", (e.currentTarget as HTMLInputElement).value)}
									class="ui-input h-9 bg-white px-3"
								/>
							</label>
							<label class="flex flex-col gap-1 text-slate-600">
								<span class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500">Port</span>
								<input
									type="number"
									value={connectionForm.port}
									oninput={(e) =>
										updateField(
											"port",
											Number((e.currentTarget as HTMLInputElement).value) || (connectionForm.databaseType === "mysql" ? 3306 : 5432),
										)}
									class="ui-input h-9 bg-white px-3"
								/>
							</label>
						{/if}
						<label class="flex flex-col gap-1 text-slate-600">
							<span class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500">{isSqlite ? "Database Path" : "Database"}</span>
							<div class="flex items-center gap-2">
								<input
									value={connectionForm.database}
									oninput={(e) =>
										updateField("database", (e.currentTarget as HTMLInputElement).value)}
									placeholder={isSqlite ? "C:/data/mydb.sqlite" : undefined}
									class="ui-input h-9 bg-white px-3"
								/>
								{#if isSqlite}
									<button
										type="button"
										onclick={chooseSqliteFile}
										class="inline-flex h-9 shrink-0 items-center gap-1 rounded-md border border-slate-200 bg-white px-3 text-[12px] font-medium text-slate-700 hover:bg-slate-100"
									>
										<FolderOpen size={14} />
										Open File
									</button>
								{/if}
							</div>
						</label>
						{#if !isSqlite}
							<label class="flex flex-col gap-1 text-slate-600">
								<span class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500">User</span>
								<input
									value={connectionForm.user}
									oninput={(e) =>
										updateField("user", (e.currentTarget as HTMLInputElement).value)}
									class="ui-input h-9 bg-white px-3"
								/>
							</label>
							<label class="sm:col-span-2 flex flex-col gap-1 text-slate-600">
								<span class="text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500">Password</span>
								<input
									type="password"
									value={connectionForm.password}
									oninput={(e) =>
										updateField("password", (e.currentTarget as HTMLInputElement).value)}
									class="ui-input h-9 bg-white px-3"
								/>
							</label>
						{/if}
						{#if connectionForm.databaseType !== "sqlite"}
							<label class="sm:col-span-2 mt-1 inline-flex items-center gap-2 rounded-md border border-slate-200 bg-slate-50 px-3 py-2 text-slate-600">
								<input
									type="checkbox"
									checked={connectionForm.ssl}
									onchange={(e) =>
										updateField("ssl", (e.currentTarget as HTMLInputElement).checked)}
									class="accent-slate-700"
								/>
								<Shield size={13} class="text-slate-600" />
								Use SSL
							</label>
						{/if}
					{/if}
				</div>

				{#if testConnectionMessage}
					<div
						class={`rounded-md border px-3 py-2 text-xs ${
							testConnectionOk
								? "border-slate-200 bg-slate-50 text-slate-700"
								: "border-slate-300 bg-slate-100 text-slate-700"
						}`}
					>
						{testConnectionMessage}
					</div>
				{/if}
			</div>

			<div class="flex flex-wrap items-center justify-end gap-2 border-t border-slate-100 bg-slate-50 px-5 py-3">
				<button
					onclick={onClose}
					class="h-8 rounded-md border border-slate-200 bg-white px-3 text-[13px] text-slate-700 hover:bg-slate-100"
				>
					Cancel
				</button>
				<button
					onclick={onTest}
					disabled={isTestingConnection || isConnecting}
					class="h-8 rounded-md border border-slate-200 bg-white px-3 text-[13px] text-slate-700 hover:bg-slate-100 disabled:opacity-60"
				>
					{isTestingConnection ? "Testing..." : "Test"}
				</button>
				<button
					onclick={onSaveAndConnect}
					disabled={isTestingConnection || isConnecting}
					class="h-8 rounded-md border border-emerald-500 bg-emerald-500 px-3 text-[13px] font-medium text-white hover:border-emerald-600 hover:bg-emerald-600 disabled:opacity-60"
				>
					{isConnecting
						? "Connecting..."
						: editing
							? "Save Changes and Connect"
							: "Save and Connect"}
				</button>
			</div>
		</div>
	</div>
{/if}


