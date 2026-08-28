<script lang="ts">
    import { X,  FolderOpen, Database } from "@lucide/svelte";
    import { open } from "@tauri-apps/plugin-dialog";
    import type { ConnectionInput } from "$lib/rpc";
    import DatabaseIcon from "$lib/components/ui/DatabaseIcon.svelte";

    let {
        visible,
        editing,
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

    function generateConnectionString(form: ConnectionInput) {
        try {
            if (form.databaseType === "sqlite") {
                return form.database ? `sqlite://${form.database}` : "";
            }
            const protocol = form.databaseType === "mysql" ? "mysql" : "postgres";
            const userPart = form.user ? encodeURIComponent(form.user) : "";
            const passPart = form.password ? `:${encodeURIComponent(form.password)}` : "";
            const authPart = userPart ? `${userPart}${passPart}@` : "";
            const hostPart = form.host || "localhost";
            const portPart = form.port ? `:${form.port}` : "";
            const dbPart = form.database ? `/${form.database}` : "";
            return `${protocol}://${authPart}${hostPart}${portPart}${dbPart}`;
        } catch {
            return "";
        }
    }

    function updateField<K extends keyof ConnectionInput>(key: K, value: ConnectionInput[K]) {
        const nextForm = { ...connectionForm, [key]: value };
        onConnectionFormChange(nextForm);
        onConnectionStringChange(generateConnectionString(nextForm));
    }

    function defaultNameForType(databaseType: ConnectionInput["databaseType"]) {
        if (databaseType === "mysql") return "local_mysql";
        if (databaseType === "sqlite") return "local_sqlite";
        return "local_pg";
    }

    function changeDatabaseType(nextType: ConnectionInput["databaseType"]) {
        if (nextType === connectionForm.databaseType) return;
        const prevType = connectionForm.databaseType;
        const prevDefaultName = defaultNameForType(prevType);
        const nextDefaultName = defaultNameForType(nextType);
        const shouldUseDefaultName = !connectionForm.name.trim() || connectionForm.name === prevDefaultName;
        const nextPort = nextType === "mysql" ? 3306 : nextType === "sqlite" ? 0 : 5432;
        const nextUser = nextType === "mysql" ? "root" : nextType === "sqlite" ? "" : "postgres";
        const nextDatabase = nextType === "mysql" ? "mysql" : nextType === "sqlite" ? "main" : "postgres";

        const nextForm = {
            ...connectionForm,
            databaseType: nextType,
            name: shouldUseDefaultName ? nextDefaultName : connectionForm.name,
            port: nextPort,
            user: nextUser,
            password: nextType === "sqlite" ? "" : connectionForm.password,
            database: nextDatabase,
            host: nextType === "sqlite" ? "" : connectionForm.host || "localhost",
            ssl: nextType === "sqlite" ? false : connectionForm.ssl,
        };

        onConnectionFormChange(nextForm);
        onConnectionStringChange(generateConnectionString(nextForm));
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

        const nextForm = {
            ...connectionForm,
            database: selected,
            name: nextName,
        };

        onConnectionFormChange(nextForm);
        onConnectionStringChange(generateConnectionString(nextForm));
    }

    function tryParseAndFill(val: string) {
        if (!val.trim()) return;
        try {
            if (val.startsWith("sqlite://")) {
                onConnectionFormChange({
                    ...connectionForm,
                    databaseType: "sqlite",
                    database: val.substring(9)
                });
                return;
            }

            const url = new URL(val);
            let dt = connectionForm.databaseType;
            if (url.protocol.includes("postgres")) dt = "postgres";
            else if (url.protocol.includes("mysql")) dt = "mysql";

            onConnectionFormChange({
                ...connectionForm,
                databaseType: dt,
                host: url.hostname || connectionForm.host,
                port: url.port ? parseInt(url.port) : (dt === "mysql" ? 3306 : 5432),
                user: url.username ? decodeURIComponent(url.username) : connectionForm.user,
                password: url.password ? decodeURIComponent(url.password) : connectionForm.password,
                database: url.pathname && url.pathname.length > 1 ? url.pathname.substring(1) : connectionForm.database,
            });
        } catch (err) {
            // Ignore parse errors while typing
        }
    }

    const engineLabels: Record<string, string> = {
        postgres: "PostgreSQL",
        mysql: "MySQL",
        sqlite: "SQLite",
        redis: "Redis",
        mongodb: "MongoDB",
        duckdb: "DuckDB",
        mssql: "SQL Server",
    };
    const engineLabel = $derived(engineLabels[connectionForm.databaseType] ?? "PostgreSQL");
    const isSqlite = $derived(connectionForm.databaseType === "sqlite");

    const connectionStringPlaceholder = $derived.by(() => {
        if (connectionForm.databaseType === "sqlite") return "sqlite://C:/path/to/db.sqlite";
        if (connectionForm.databaseType === "mysql") return "mysql://root:password@localhost:3306/mydb";
        return "postgres://postgres:password@localhost:5432/postgres";
    });

    let step = $state(1);
    let wasVisible = $state(false);

    $effect(() => {
        if (visible && !wasVisible) {
            step = editing ? 2 : 1;

            if (!editing) {
                const freshForm: ConnectionInput = {
                    databaseType: "postgres",
                    name: "local_pg",
                    host: "localhost",
                    port: 5432,
                    user: "postgres",
                    password: "",
                    database: "postgres",
                    ssl: false,
                };
                onConnectionFormChange(freshForm);
                onConnectionStringChange(generateConnectionString(freshForm));
            }
        }
        wasVisible = visible;
    });
</script>

{#if visible}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
        <div class="w-full max-w-[620px] overflow-hidden rounded-md border border-slate-200 bg-white shadow-[0_20px_50px_rgba(15,23,42,0.18)]">
            <div class="border-b border-slate-200 bg-white px-4 py-3 text-slate-900">
                <div class="flex items-start justify-between gap-3">
                    <div>
                        <h3 class="inline-flex items-center gap-2.5 text-[14px] font-semibold text-slate-900">
                            <span class="inline-flex h-7 w-7 items-center justify-center rounded-sm bg-slate-100 text-slate-700">
                                <DatabaseIcon type={connectionForm.databaseType} size={18} />
                            </span>
                            {editing
                                ? `Edit ${engineLabel} Connection`
                                : `New ${engineLabel} Connection`}
                        </h3>
                        <div class="mt-2.5 flex items-center gap-2 text-[11px] text-slate-400">
                            <span class={step === 1 ? "font-semibold text-slate-700" : ""}>1. Database</span>
                            <span class="text-slate-300">/</span>
                            <span class={step === 2 ? "font-semibold text-slate-700" : ""}>2. Details</span>
                        </div>
                    </div>
                    <button
                        aria-label="Close modal"
                        title="Close"
                        onclick={onClose}
                        class="flex h-7 w-7 items-center justify-center rounded-sm text-slate-400 hover:bg-slate-100 hover:text-slate-700"
                    >
                        <X size={16} />
                    </button>
                </div>
            </div>

            <div class="space-y-3 p-4 text-[13px]">
            {#if step === 1}
                <div>
                    <div class="mb-2 flex items-center justify-between">
                        <span class="text-[10px] font-semibold uppercase tracking-wider text-slate-500">Database Type</span>
                        <span class="text-[11px] text-slate-400">Choose an engine</span>
                    </div>
                    <div class="grid grid-cols-2 gap-2 sm:grid-cols-3">
                        {#each [
                            { value: "postgres", label: "PostgreSQL", detail: "Relational", disabled: false },
                            { value: "mysql", label: "MySQL", detail: "Relational", disabled: false },
                            { value: "sqlite", label: "SQLite", detail: "Local file", disabled: false },
                            { value: "redis", label: "Redis", detail: "In-memory", disabled: true },
                            { value: "mongodb", label: "MongoDB", detail: "Document", disabled: true },
                            { value: "duckdb", label: "DuckDB", detail: "Analytical", disabled: true },
                            { value: "mssql", label: "SQL Server", detail: "Relational", disabled: true },
                        ] as database}
                            <button
                                type="button"
                                disabled={database.disabled}
                                onclick={() => changeDatabaseType(database.value as any)}
                                class={`flex min-w-0 flex-col items-center justify-center gap-1.5 rounded-sm border px-2.5 py-2.5 text-center transition-colors ${
                                    database.disabled
                                        ? "border-slate-100 bg-slate-50/50 text-slate-400 opacity-60 cursor-not-allowed"
                                        : connectionForm.databaseType === database.value
                                          ? "border-emerald-500 bg-emerald-50/70 text-slate-900"
                                          : "border-slate-200 bg-white text-slate-600 hover:border-slate-300 hover:bg-slate-50"
                                }`}
                            >
                                <span class={`inline-flex h-9 w-9 items-center justify-center rounded-sm ${database.disabled ? "bg-slate-200/60" : "bg-slate-100"}`}>
                                    {#if database.value === "postgres" || database.value === "mysql" || database.value === "sqlite"}
                                        <DatabaseIcon type={database.value as any} size={22} />
                                    {:else if database.value === "redis" || database.value === "mongodb" || database.value === "duckdb" || database.value === "mssql"}
                                        <DatabaseIcon type={database.value as any} size={22} />
                                    {:else}
                                        <Database size={21} strokeWidth={1.7} class="text-slate-400" />
                                    {/if}
                                </span>
                                <span class="min-w-0">
                                    <span class="block truncate text-[12px] font-semibold">{database.label}</span>
                                    <span class="block text-[10px] text-slate-400">{database.detail}</span>
                                </span>
                            </button>
                        {/each}
                    </div>
                </div>
            {:else}
                <div class="flex items-center gap-3 rounded-sm border border-slate-200 bg-slate-50 px-3 py-2">
                    <DatabaseIcon type={connectionForm.databaseType} size={22} />
                    <div class="min-w-0">
                        <div class="text-[12px] font-semibold text-slate-900">{engineLabel}</div>
                        <div class="text-[11px] text-slate-500">{isSqlite ? "Connect to a local database file" : "Connect to a database server"}</div>
                    </div>
                    <button type="button" onclick={() => (step = 1)} class="ml-auto text-[11px] font-medium text-slate-500 hover:text-slate-900">Change</button>
                </div>

                <div class="grid content-start grid-cols-1 gap-2.5 sm:grid-cols-2">
                    <label class="sm:col-span-2 flex flex-col gap-1 text-slate-600">
                        <span class="text-[10px] font-semibold uppercase tracking-wider text-slate-500">Connection Name</span>
                        <input
                            value={connectionForm.name}
                            oninput={(e) =>
                                updateField("name", (e.currentTarget as HTMLInputElement).value)}
                            class="ui-input h-8 bg-white px-3"
                        />
                    </label>

                    {#if connectionForm.databaseType !== "sqlite"}
                        <label class="sm:col-span-2 flex flex-col gap-1 text-slate-600">
                            <span class="text-[10px] font-semibold uppercase tracking-wider text-slate-500">Host & Port</span>
                            <div class="flex gap-2">
                                <input
                                    value={connectionForm.host}
                                    oninput={(e) =>
                                        updateField("host", (e.currentTarget as HTMLInputElement).value)}
                                    class="ui-input h-8 w-full bg-white px-3"
                                    placeholder="localhost"
                                />
                                <input
                                    type="number"
                                    value={connectionForm.port}
                                    oninput={(e) =>
                                        updateField(
                                            "port",
                                            Number((e.currentTarget as HTMLInputElement).value) || (connectionForm.databaseType === "mysql" ? 3306 : 5432),
                                        )}
                                    class="ui-input h-8 w-24 shrink-0 bg-white px-3"
                                    title="Port"
                                />
                            </div>
                        </label>
                    {/if}
                    <label class="sm:col-span-2 flex flex-col gap-1 text-slate-600">
                        <span class="text-[10px] font-semibold uppercase tracking-wider text-slate-500">{isSqlite ? "Database Path" : "Database"}{!isSqlite && connectionForm.databaseType === "postgres" ? ' — Optional' : ''}</span>
                        <div class="flex items-center gap-2">
                            <input
                                value={connectionForm.database}
                                oninput={(e) =>
                                    updateField("database", (e.currentTarget as HTMLInputElement).value)}
                                placeholder={isSqlite
                                    ? "C:/data/mydb.sqlite"
                                    : connectionForm.databaseType === "postgres"
                                        ? "postgres (default)"
                                        : connectionForm.databaseType === "mysql"
                                            ? "mysql (default)"
                                            : undefined}
                                class="ui-input h-8 w-full bg-white px-3 placeholder:text-slate-400"
                            />
                            {#if isSqlite}
                                <button
                                    type="button"
                                    onclick={chooseSqliteFile}
                                    class="inline-flex h-8 shrink-0 items-center gap-1 rounded-sm border border-slate-200 bg-white px-3 text-[12px] font-medium text-slate-700 hover:bg-slate-100"
                                >
                                    <FolderOpen size={14} />
                                    Open File
                                </button>
                            {/if}
                        </div>
                    </label>
                    {#if !isSqlite}
                        <label class="flex flex-col gap-1 text-slate-600">
                            <span class="text-[10px] font-semibold uppercase tracking-wider text-slate-500">User</span>
                            <input
                                value={connectionForm.user}
                                oninput={(e) =>
                                    updateField("user", (e.currentTarget as HTMLInputElement).value)}
                                class="ui-input h-8 bg-white px-3"
                            />
                        </label>
                        <label class="flex flex-col gap-1 text-slate-600">
                            <span class="text-[10px] font-semibold uppercase tracking-wider text-slate-500">Password</span>
                            <input
                                type="password"
                                value={connectionForm.password}
                                oninput={(e) =>
                                    updateField("password", (e.currentTarget as HTMLInputElement).value)}
                                class="ui-input h-8 bg-white px-3"
                            />
                        </label>
                    {/if}
                </div>

                <div class="sm:col-span-2 mt-2 pt-1">
                    <div class="relative py-2">
                        <div class="absolute inset-0 flex items-center" aria-hidden="true">
                            <div class="w-full border-t border-slate-200"></div>
                        </div>
                        <div class="relative flex justify-center">
                            <span class="bg-white px-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400">Or paste a connection string to auto-fill</span>
                        </div>
                    </div>
                    <input
                        value={connectionStringInput}
                        oninput={(e) => {
                            const val = (e.currentTarget as HTMLInputElement).value;
                            onConnectionStringChange(val);
                            onModeChange(val.trim() ? "string" : "fields");
                            tryParseAndFill(val);
                        }}
                        placeholder={connectionStringPlaceholder}
                        class="ui-input h-8 w-full bg-white px-3 mt-1 placeholder:text-slate-400"
                    />
                </div>
            {/if}

                {#if testConnectionMessage}
                    <div
                        class={`rounded-sm border px-3 py-2 text-xs ${
                            testConnectionOk
                                ? "border-slate-200 bg-slate-50 text-slate-700"
                                : "border-slate-300 bg-slate-100 text-slate-700"
                        }`}
                    >
                        {testConnectionMessage}
                    </div>
                {/if}
            </div>

            <div class="flex flex-wrap items-center justify-between gap-2 border-t border-slate-100 bg-slate-50 px-4 py-3">
                {#if step === 2}
                    <button type="button" onclick={() => (step = 1)} class="h-8 rounded-sm border border-[#1c1c1e] bg-[#1c1c1e] px-3 text-[13px] font-medium text-white hover:border-[#111113] hover:bg-[#111113]">Back</button>
                {:else}
                    <span></span>
                {/if}
                <div class="flex flex-wrap items-center justify-end gap-2">
                <button
                    onclick={onClose}
                    class="h-8 rounded-sm border border-slate-200 bg-white px-3 text-[13px] text-slate-700 hover:bg-slate-100"
                >
                    Cancel
                </button>
                {#if step === 1}
                <button
                    type="button"
                    onclick={() => (step = 2)}
                    class="h-8 rounded-sm border border-[#1c1c1e] bg-[#1c1c1e] px-3 text-[13px] font-medium text-white hover:border-[#111113] hover:bg-[#111113]"
                >
                    Next
                </button>
                {:else}
                <button
                    onclick={onTest}
                    disabled={isTestingConnection || isConnecting}
                    class="h-8 rounded-sm border border-slate-200 bg-white px-3 text-[13px] text-slate-700 hover:bg-slate-100 disabled:opacity-60"
                >
                    {isTestingConnection ? "Testing..." : "Test"}
                </button>
                <button
                    onclick={onSaveAndConnect}
                    disabled={isTestingConnection || isConnecting}
                    class="h-8 rounded-sm border border-emerald-500 bg-emerald-500 px-3 text-[13px] font-medium text-white hover:border-emerald-600 hover:bg-emerald-600 disabled:opacity-60"
                >
                    {isConnecting
                        ? "Connecting..."
                        : editing
                            ? "Save Changes and Connect"
                            : "Save and Connect"}
                </button>
                    {/if}
                    </div>
            </div>
        </div>
    </div>
{/if}