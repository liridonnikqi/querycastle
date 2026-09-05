<script lang="ts">
    import { X, FolderOpen } from "@lucide/svelte";
    import { open } from "@tauri-apps/plugin-dialog";
    import type { ConnectionInput, DatabaseType } from "$lib/rpc";
    import DatabaseIcon from "$lib/components/ui/DatabaseIcon.svelte";
    import { generateConnectionString } from "$lib/utils/connection";
    import { engineDisplayName } from "$lib/utils/dialect";

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

    function updateField<K extends keyof ConnectionInput>(key: K, value: ConnectionInput[K]) {
        const nextForm = { ...connectionForm, [key]: value };
        onConnectionFormChange(nextForm);
        onConnectionStringChange(generateConnectionString(nextForm));
    }

    function updateFields(patch: Partial<ConnectionInput>) {
        const nextForm = { ...connectionForm, ...patch };
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
            sslInsecure: nextType === "sqlite" ? false : connectionForm.sslInsecure,
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

    const engineLabel = $derived(engineDisplayName(connectionForm.databaseType));
    const isSqlite = $derived(connectionForm.databaseType === "sqlite");

    const connectionStringPlaceholder = $derived.by(() => {
        if (connectionForm.databaseType === "sqlite") return "sqlite://C:/path/to/db.sqlite";
        if (connectionForm.databaseType === "mysql") return "mysql://root:password@localhost:3306/mydb";
        return "postgres://postgres:password@localhost:5432/postgres";
    });

    const engineOptions: Array<{ value: DatabaseType; label: string; detail: string }> = [
        { value: "postgres", label: "PostgreSQL", detail: "Relational" },
        { value: "mysql", label: "MySQL", detail: "Relational" },
        { value: "sqlite", label: "SQLite", detail: "Local file" },
    ];

    let step = $state(1);
    let wasVisible = $state(false);
    let nameInput: HTMLInputElement | null = $state(null);

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
                    sslInsecure: false,
                };
                onConnectionFormChange(freshForm);
                onConnectionStringChange(generateConnectionString(freshForm));
            }
        }
        wasVisible = visible;
    });

    // Focus the name field whenever the details step is shown.
    $effect(() => {
        if (visible && step === 2) {
            const node = nameInput;
            if (node) queueMicrotask(() => node.focus());
        }
    });

    function handleBackdropClick(event: MouseEvent) {
        // Close only when the backdrop itself is clicked, not the panel.
        if (event.target === event.currentTarget) onClose();
    }

    function handleWindowKeydown(event: KeyboardEvent) {
        if (!visible) return;
        if (event.key === "Escape") onClose();
    }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if visible}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions: backdrop click mirrors Cancel; Escape is handled globally above -->
    <div
        role="presentation"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-[2px] p-4 cursor-default"
        onclick={handleBackdropClick}
    >
        <div
            class="w-full max-w-[560px] overflow-hidden rounded-lg border border-qc-border bg-qc-elevated shadow-[0_24px_60px_rgba(0,0,0,0.35)]"
            role="dialog"
            aria-modal="true"
            aria-label={editing ? `Edit ${engineLabel} connection` : `New ${engineLabel} connection`}
        >
            <div class="border-b border-qc-border bg-qc-elevated px-4 py-3 text-qc-fg">
                <div class="flex items-center justify-between gap-3">
                    <div class="flex min-w-0 items-center gap-2.5">
                        <span class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-qc-hover text-qc-subtle">
                            <DatabaseIcon type={connectionForm.databaseType} size={18} />
                        </span>
                        <div class="min-w-0">
                            <h3 class="truncate text-[13px] font-semibold text-qc-fg">
                                {editing
                                    ? `Edit ${engineLabel} Connection`
                                    : `New ${engineLabel} Connection`}
                            </h3>
                            <div class="mt-1 flex items-center gap-1 text-[11px]">
                                <span class={`rounded-full px-2 py-0.5 ${step === 1 ? "bg-qc-hover font-medium text-qc-fg" : "text-qc-muted"}`}>1. Database</span>
                                <span class={`rounded-full px-2 py-0.5 ${step === 2 ? "bg-qc-hover font-medium text-qc-fg" : "text-qc-muted"}`}>2. Details</span>
                            </div>
                        </div>
                    </div>
                    <button
                        aria-label="Close modal"
                        title="Close"
                        onclick={onClose}
                        class="flex h-7 w-7 shrink-0 items-center justify-center rounded-sm text-qc-muted hover:bg-qc-hover hover:text-qc-fg"
                    >
                        <X size={16} />
                    </button>
                </div>
            </div>

            <div class="space-y-3 p-4 text-[13px] bg-qc-elevated text-qc-fg">
            {#if step === 1}
                <div>
                    <div class="mb-2 flex items-center justify-between">
                        <span class="text-[10px] font-semibold uppercase tracking-wider text-qc-muted">Database Type</span>
                        <span class="text-[11px] text-qc-muted">Choose an engine</span>
                    </div>
                    <div class="grid grid-cols-2 gap-2 sm:grid-cols-3">
                        {#each engineOptions as database (database.value)}
                            <button
                                type="button"
                                title={database.label}
                                onclick={() => changeDatabaseType(database.value)}
                                class={`relative flex min-w-0 flex-col items-center justify-center gap-1 rounded-md border px-2 py-2 text-center transition-colors ${
                                    connectionForm.databaseType === database.value
                                        ? "border-qc-fg bg-qc-hover text-qc-fg"
                                        : "border-qc-border bg-qc-panel text-qc-subtle hover:border-qc-muted hover:bg-qc-hover"
                                }`}
                            >
                                <span class="inline-flex h-8 w-8 items-center justify-center rounded-md bg-qc-elevated">
                                    <DatabaseIcon type={database.value} size={20} />
                                </span>
                                <span class="min-w-0">
                                    <span class="block truncate text-[12px] font-semibold">{database.label}</span>
                                    <span class="block text-[10px] text-qc-muted">{database.detail}</span>
                                </span>
                            </button>
                        {/each}
                    </div>
                </div>
            {:else}
                <div class="flex items-center gap-3 rounded-md border border-qc-border bg-qc-panel px-3 py-2">
                    <DatabaseIcon type={connectionForm.databaseType} size={22} />
                    <div class="min-w-0">
                        <div class="text-[12px] font-semibold text-qc-fg">{engineLabel}</div>
                        <div class="text-[11px] text-qc-muted">{isSqlite ? "Connect to a local database file" : "Connect to a database server"}</div>
                    </div>
                    <button type="button" onclick={() => (step = 1)} class="ml-auto text-[11px] font-medium text-qc-muted hover:text-qc-fg">Change</button>
                </div>

                <div class="grid content-start grid-cols-1 gap-2.5 sm:grid-cols-2">
                    <label class="sm:col-span-2 flex flex-col gap-1 text-qc-subtle">
                        <span class="text-[10px] font-semibold uppercase tracking-wider text-qc-muted">Connection Name</span>
                        <input
                            bind:this={nameInput}
                            value={connectionForm.name}
                            oninput={(e) =>
                                updateField("name", (e.currentTarget as HTMLInputElement).value)}
                            class="ui-input h-8 px-3"
                        />
                    </label>

                    {#if connectionForm.databaseType !== "sqlite"}
                        <label class="sm:col-span-2 flex flex-col gap-1 text-qc-subtle">
                            <span class="text-[10px] font-semibold uppercase tracking-wider text-qc-muted">Host & Port</span>
                            <div class="flex gap-2">
                                <input
                                    value={connectionForm.host}
                                    oninput={(e) =>
                                        updateField("host", (e.currentTarget as HTMLInputElement).value)}
                                    class="ui-input h-8 w-full px-3"
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
                                    class="ui-input h-8 w-24 shrink-0 px-3"
                                    title="Port"
                                />
                            </div>
                        </label>
                    {/if}
                    <label class="sm:col-span-2 flex flex-col gap-1 text-qc-subtle">
                        <span class="text-[10px] font-semibold uppercase tracking-wider text-qc-muted">{isSqlite ? "Database Path" : "Database"}{!isSqlite && connectionForm.databaseType === "postgres" ? ' — Optional' : ''}</span>
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
                                class="ui-input h-8 w-full px-3 placeholder:text-qc-muted"
                            />
                            {#if isSqlite}
                                <button
                                    type="button"
                                    onclick={chooseSqliteFile}
                                    class="inline-flex h-8 shrink-0 items-center gap-1 rounded-md border border-qc-border bg-qc-panel px-3 text-[12px] font-medium text-qc-subtle hover:bg-qc-hover hover:text-qc-fg"
                                >
                                    <FolderOpen size={14} />
                                    Open File
                                </button>
                            {/if}
                        </div>
                    </label>
                    {#if !isSqlite}
                        <label class="flex flex-col gap-1 text-qc-subtle">
                            <span class="text-[10px] font-semibold uppercase tracking-wider text-qc-muted">User</span>
                            <input
                                value={connectionForm.user}
                                oninput={(e) =>
                                    updateField("user", (e.currentTarget as HTMLInputElement).value)}
                                class="ui-input h-8 px-3"
                            />
                        </label>
                        <label class="flex flex-col gap-1 text-qc-subtle">
                            <span class="text-[10px] font-semibold uppercase tracking-wider text-qc-muted">Password</span>
                            <input
                                type="password"
                                value={connectionForm.password}
                                oninput={(e) =>
                                    updateField("password", (e.currentTarget as HTMLInputElement).value)}
                                class="ui-input h-8 px-3"
                            />
                        </label>
                        <div class="sm:col-span-2 flex flex-col gap-2">
                            <label class="flex items-center gap-2 text-[12px] text-qc-subtle">
                                <input
                                    type="checkbox"
                                    class="qc-check"
                                    checked={connectionForm.ssl}
                                    onchange={(e) => {
                                        const checked = e.currentTarget.checked;
                                        updateFields({
                                            ssl: checked,
                                            sslInsecure: checked ? connectionForm.sslInsecure : false,
                                        });
                                    }}
                                />
                                Use SSL
                            </label>
                            <label class="flex items-center gap-2 text-[12px] text-qc-subtle">
                                <input
                                    type="checkbox"
                                    class="qc-check"
                                    checked={connectionForm.sslInsecure ?? false}
                                    disabled={!connectionForm.ssl}
                                    onchange={(e) =>
                                        updateField("sslInsecure", e.currentTarget.checked)}
                                />
                                Allow insecure TLS (self-signed)
                            </label>
                        </div>
                    {/if}
                </div>

                <div class="sm:col-span-2 mt-2 pt-1">
                    <div class="relative py-2">
                        <div class="absolute inset-0 flex items-center" aria-hidden="true">
                            <div class="w-full border-t border-qc-border"></div>
                        </div>
                        <div class="relative flex justify-center">
                            <span class="bg-qc-elevated px-2 text-[10px] font-semibold uppercase tracking-wider text-qc-muted">Or paste a connection string to auto-fill</span>
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
                        class="ui-input h-8 w-full px-3 mt-1 placeholder:text-qc-muted"
                    />
                </div>
            {/if}

                {#if testConnectionMessage}
                    <div
                        class={`rounded-md border px-3 py-2 text-xs ${
                            testConnectionOk
                                ? "border-qc-border bg-qc-panel text-qc-subtle"
                                : "border-qc-danger/30 bg-qc-danger/10 text-qc-danger"
                        }`}
                    >
                        {testConnectionMessage}
                    </div>
                {/if}
            </div>

            <div class="flex flex-wrap items-center justify-between gap-2 border-t border-qc-border bg-qc-panel px-4 py-3">
                {#if step === 2}
                    <button type="button" onclick={() => (step = 1)} class="btn-secondary h-8 px-3 text-[13px] font-medium">Back</button>
                {:else}
                    <span></span>
                {/if}
                <div class="flex flex-wrap items-center justify-end gap-2">
                <button
                    onclick={onClose}
                    class="btn-secondary h-8 px-3 text-[13px] font-medium"
                >
                    Cancel
                </button>
                {#if step === 1}
                <button
                    type="button"
                    onclick={() => (step = 2)}
                    class="h-8 btn-primary px-3 text-[13px] font-medium"
                >
                    Next
                </button>
                {:else}
                <button
                    onclick={onTest}
                    disabled={isTestingConnection || isConnecting}
                    class="btn-secondary h-8 px-3 text-[13px] font-medium disabled:opacity-60"
                >
                    {isTestingConnection ? "Testing..." : "Test"}
                </button>
                <button
                    onclick={onSaveAndConnect}
                    disabled={isTestingConnection || isConnecting}
                    class="h-8 btn-primary px-3 text-[13px] font-medium disabled:opacity-60"
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