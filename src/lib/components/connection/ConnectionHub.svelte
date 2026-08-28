<script lang="ts">
    import { 
        Database, 
        Plus, 
        MoreVertical 
    } from '@lucide/svelte';
    import type { ConnectionInput } from '$lib/rpc';
    import DatabaseIcon from '$lib/components/ui/DatabaseIcon.svelte';

    let {
        savedConnections,
        onConnect,
        onCreate,
        onEdit,
        onDelete,
        connectingName,
        searchQuery,
    }: {
        savedConnections: ConnectionInput[];
        onConnect: (connection: ConnectionInput) => void;
        onCreate: () => void;
        onEdit: (connection: ConnectionInput) => void;
        onDelete: (name: string) => void;
        connectingName: string | null;
        searchQuery: string;
    } = $props();

    // Track active menu state & filter choice
    let activeMenuName = $state<string | null>(null);
    let selectedFilter = $state<string>('all');

    // Filter connections based on search query and engine filter
    let filteredConnections = $derived(
        savedConnections.filter((connection) => {
            const query = searchQuery ? searchQuery.trim().toLowerCase() : '';
            const matchesQuery = !query || (
                connection.name.toLowerCase().includes(query) ||
                connection.host.toLowerCase().includes(query) ||
                connection.database.toLowerCase().includes(query)
            );
            const matchesFilter = selectedFilter === 'all' || connection.databaseType === selectedFilter;
            return matchesQuery && matchesFilter;
        })
    );

    let counts = $derived({
        all: savedConnections.length,
        postgres: savedConnections.filter(c => c.databaseType === 'postgres').length,
        mysql: savedConnections.filter(c => c.databaseType === 'mysql').length,
        sqlite: savedConnections.filter(c => c.databaseType === 'sqlite').length,
    });
</script>

<div class="w-full min-h-full flex-1 bg-white p-6 md:p-10 flex flex-col justify-start">
    <div class="w-full max-w-6xl mx-auto space-y-8">
        
        <!-- HEADER -->
        <header class="flex flex-col sm:flex-row sm:items-end justify-between gap-6 pb-6 border-b border-gray-100">
            <div>
                <h1 class="text-gray-900 text-3xl font-medium tracking-tight">
                    Databases
                </h1>
                <p class="text-gray-500 text-sm mt-1.5">
                    Select a connection target to launch your workspace.
                </p>
            </div>

            <button
                onclick={onCreate}
                class="h-8 px-4 rounded-md text-xs font-medium bg-emerald-600 text-white hover:bg-emerald-700 transition-colors inline-flex items-center justify-center gap-1.5"
            >
                <Plus size={14} strokeWidth={2.5} />
                <span>New Connection</span>
            </button>
        </header>

        <!-- MAIN CONNECTIONS AREA -->
        <div class="space-y-6">
            
            <!-- FILTER TOOLBAR -->
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
                <div class="flex items-center gap-2">
                    <button
                        onclick={() => selectedFilter = 'all'}
                        class="px-3 py-1.5 text-xs font-medium rounded border transition-colors {selectedFilter === 'all' ? 'border-[#1c1c1e] bg-[#1c1c1e] text-white' : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300'}"
                    >
                        All <span class="opacity-60 ml-1 font-mono">{counts.all}</span>
                    </button>
                    <button
                        onclick={() => selectedFilter = 'postgres'}
                        class="px-3 py-1.5 text-xs font-medium rounded border transition-colors {selectedFilter === 'postgres' ? 'border-[#1c1c1e] bg-[#1c1c1e] text-white' : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300'}"
                    >
                        PostgreSQL <span class="opacity-60 ml-1 font-mono">{counts.postgres}</span>
                    </button>
                    <button
                        onclick={() => selectedFilter = 'mysql'}
                        class="px-3 py-1.5 text-xs font-medium rounded border transition-colors {selectedFilter === 'mysql' ? 'border-[#1c1c1e] bg-[#1c1c1e] text-white' : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300'}"
                    >
                        MySQL <span class="opacity-60 ml-1 font-mono">{counts.mysql}</span>
                    </button>
                    <button
                        onclick={() => selectedFilter = 'sqlite'}
                        class="px-3 py-1.5 text-xs font-medium rounded border transition-colors {selectedFilter === 'sqlite' ? 'border-[#1c1c1e] bg-[#1c1c1e] text-white' : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300'}"
                    >
                        SQLite <span class="opacity-60 ml-1 font-mono">{counts.sqlite}</span>
                    </button>
                </div>

                {#if searchQuery}
                    <div class="text-xs text-gray-500 font-medium">
                        Search: <span class="text-emerald-700 font-semibold">"{searchQuery}"</span>
                    </div>
                {/if}
            </div>

            <!-- CONNECTIONS GRID -->
            {#if savedConnections.length === 0}
                <div class="border border-dashed border-gray-300 rounded-lg bg-gray-50/50 p-12 text-center mt-4">
                    <div class="w-12 h-12 mx-auto rounded border border-gray-200 bg-white flex items-center justify-center text-gray-400 mb-4">
                        <Database size={22} strokeWidth={1.5} />
                    </div>
                    <h2 class="text-gray-900 text-base font-medium tracking-tight">No connections found</h2>
                    <p class="text-gray-500 text-sm mt-1 max-w-sm mx-auto">
                        Add a database target to start exploring your data.
                    </p>
                    <button
                        onclick={onCreate}
                        class="mt-6 h-8 px-4 rounded-md text-xs font-medium bg-emerald-600 text-white hover:bg-emerald-700 transition-colors inline-flex items-center justify-center gap-1.5"
                    >
                        <Plus size={14} strokeWidth={2.5} />
                        <span>Create Connection</span>
                    </button>
                </div>
            {:else if filteredConnections.length === 0}
                <div class="border border-dashed border-gray-200 rounded-lg p-10 text-center text-gray-500 text-sm font-medium mt-4">
                    No connections match your active filter or search.
                </div>
            {:else}
                <!-- FLAT COMPACT BUTTON CARDS -->
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3">
                    {#each filteredConnections as connection (connection.name)}
                        {@const isConnecting = connectingName === connection.name}
                        
                        <div
                            role="button"
                            tabindex="0"
                            onclick={() => {
                                if (isConnecting) return;
                                onConnect(connection);
                            }}
                            onkeydown={(event) => {
                                if (isConnecting) return;
                                if (event.key === 'Enter' || event.key === ' ') {
                                    event.preventDefault();
                                    onConnect(connection);
                                }
                            }}
                            class="relative bg-white border border-gray-200 hover:border-gray-300 hover:bg-gray-50 rounded-md p-3 transition-colors cursor-pointer flex items-center justify-between gap-3 select-none"
                        >
                            <!-- Left: Minimalist Icon + Text -->
                            <div class="flex items-center gap-3 min-w-0">
                                <div class="w-8 h-8 rounded bg-gray-50 border border-gray-100 flex items-center justify-center shrink-0">
                                    <DatabaseIcon type={connection.databaseType} size={16} />
                                </div>

                                <div class="min-w-0 flex flex-col">
                                    <div class="flex items-center gap-2">
                                        <span class="text-gray-900 font-medium text-sm tracking-tight truncate">
                                            {connection.name}
                                        </span>
                                    </div>
                                    <div class="flex items-center gap-1.5 text-[11px] text-gray-500 font-mono mt-0.5 truncate">
                                        <span class="truncate">{connection.host || 'localhost'}</span>
                                        <span class="text-gray-300">/</span>
                                        <span class="truncate">{connection.database}</span>
                                    </div>
                                </div>
                            </div>

                            <!-- Right: Actions -->
                            <div class="flex items-center gap-1.5 shrink-0">
                                {#if isConnecting}
                                    <div class="w-6 h-6 flex items-center justify-center text-emerald-600">
                                        <div class="w-3.5 h-3.5 border-2 border-emerald-600 border-t-transparent rounded-full animate-spin"></div>
                                    </div>
                                {/if}

                                <div class="relative">
                                    <button
                                        onclick={(event) => {
                                            event.stopPropagation();
                                            activeMenuName = activeMenuName === connection.name ? null : connection.name;
                                        }}
                                        class="p-1 rounded text-gray-400 hover:text-gray-800 hover:bg-gray-200 transition-colors"
                                        title="Options"
                                        aria-label="Options"
                                    >
                                        <MoreVertical size={16} />
                                    </button>

                                    <!-- Flat Dropdown Menu -->
                                    {#if activeMenuName === connection.name}
                                        <div
                                            class="absolute right-0 top-full mt-1 w-28 bg-white border border-gray-300 rounded-md py-1 z-50"
                                            onclick={(e) => e.stopPropagation()}
                                            onkeydown={(e) => {
                                                if (e.key === "Escape") activeMenuName = null;
                                            }}
                                            role="menu"
                                            tabindex="-1"
                                        >
                                            <button
                                                onclick={() => {
                                                    activeMenuName = null;
                                                    onEdit(connection);
                                                }}
                                                class="w-full text-left px-3 py-1.5 text-xs font-medium text-gray-700 hover:bg-gray-100 transition-colors"
                                                role="menuitem"
                                            >
                                                Edit
                                            </button>
                                            <button
                                                onclick={() => {
                                                    activeMenuName = null;
                                                    onDelete(connection.name);
                                                }}
                                                class="w-full text-left px-3 py-1.5 text-xs font-medium text-red-600 hover:bg-red-50 transition-colors"
                                                role="menuitem"
                                            >
                                                Delete
                                            </button>
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    </div>
</div>