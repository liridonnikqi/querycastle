<script lang="ts">
	import { X, Shield, DatabaseZap } from "@lucide/svelte";
	import type { ConnectionInput } from "../lib/rpc";

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
</script>

{#if visible}
	<div class="fixed inset-0 bg-black/35 backdrop-blur-[1px] flex items-center justify-center z-50 p-4">
		<div class="w-full max-w-[620px] rounded-xl bg-white border border-gray-200 shadow-[0_24px_60px_rgba(16,37,70,0.26)] overflow-hidden">
			<div class="h-12 px-4 border-b border-gray-200 flex items-center justify-between bg-white text-gray-900">
				<h3 class="text-sm font-semibold inline-flex items-center gap-2"><DatabaseZap size={16} class="text-emerald-600" />{editing ? "Edit PostgreSQL Connection" : "New PostgreSQL Connection"}</h3>
				<button aria-label="Close modal" title="Close" onclick={onClose} class="text-gray-400 hover:text-gray-700 w-7 h-7 rounded-md flex items-center justify-center hover:bg-gray-100"><X size={16} /></button>
			</div>

			<div class="p-4 grid grid-cols-2 gap-3 text-[13px]">
				<div class="col-span-2 inline-flex rounded-md border border-gray-200 bg-gray-50 p-1">
					<button onclick={() => onModeChange("fields")} class={`px-3 py-1.5 rounded text-xs ${mode === "fields" ? "bg-white border border-gray-200 text-gray-900" : "text-gray-500"}`}>Fields</button>
					<button onclick={() => onModeChange("string")} class={`px-3 py-1.5 rounded text-xs ${mode === "string" ? "bg-white border border-gray-200 text-gray-900" : "text-gray-500"}`}>Connection String</button>
				</div>

				<label class="flex flex-col gap-1 text-gray-600">
					Connection Name
					<input value={connectionForm.name} oninput={(e) => updateField("name", (e.currentTarget as HTMLInputElement).value)} class="ui-input h-9 px-3 bg-white" />
				</label>

				{#if mode === "string"}
					<label class="col-span-2 flex flex-col gap-1 text-gray-600">
						Connection String
						<input
							value={connectionStringInput}
							oninput={(e) => onConnectionStringChange((e.currentTarget as HTMLInputElement).value)}
							placeholder="postgresql://postgres:password@host:5432/dbname"
							class="ui-input h-9 px-3 bg-white"
						/>
					</label>
				{:else}
					<label class="flex flex-col gap-1 text-gray-600">Host<input value={connectionForm.host} oninput={(e) => updateField("host", (e.currentTarget as HTMLInputElement).value)} class="ui-input h-9 px-3 bg-white" /></label>
					<label class="flex flex-col gap-1 text-gray-600">Port<input type="number" value={connectionForm.port} oninput={(e) => updateField("port", Number((e.currentTarget as HTMLInputElement).value) || 5432)} class="ui-input h-9 px-3 bg-white" /></label>
					<label class="flex flex-col gap-1 text-gray-600">Database<input value={connectionForm.database} oninput={(e) => updateField("database", (e.currentTarget as HTMLInputElement).value)} class="ui-input h-9 px-3 bg-white" /></label>
					<label class="flex flex-col gap-1 text-gray-600">User<input value={connectionForm.user} oninput={(e) => updateField("user", (e.currentTarget as HTMLInputElement).value)} class="ui-input h-9 px-3 bg-white" /></label>
					<label class="flex flex-col gap-1 text-gray-600">Password<input type="password" value={connectionForm.password} oninput={(e) => updateField("password", (e.currentTarget as HTMLInputElement).value)} class="ui-input h-9 px-3 bg-white" /></label>
					<label class="col-span-2 flex items-center gap-2 text-gray-600 mt-1">
						<input type="checkbox" checked={connectionForm.ssl} onchange={(e) => updateField("ssl", (e.currentTarget as HTMLInputElement).checked)} class="accent-emerald-500" />
						<Shield size={13} /> Use SSL
					</label>
				{/if}

				{#if testConnectionMessage}
					<div class={`col-span-2 px-3 py-2 rounded-md border text-xs ${testConnectionOk ? "bg-emerald-50 border-emerald-200 text-emerald-700" : "bg-red-50 border-red-200 text-red-700"}`}>{testConnectionMessage}</div>
				{/if}
			</div>

			<div class="h-14 px-4 border-t border-gray-100 flex justify-end items-center gap-2 bg-gray-50">
				<button onclick={onClose} class="h-9 px-4 rounded-md text-sm border border-gray-200 bg-white text-gray-700 hover:bg-gray-100">Cancel</button>
				<button onclick={onTest} disabled={isTestingConnection || isConnecting} class="h-9 px-4 rounded-md text-sm border border-gray-200 bg-white text-gray-700 hover:bg-gray-100 disabled:opacity-60">{isTestingConnection ? "Testing..." : "Test"}</button>
				<button onclick={onSaveAndConnect} disabled={isTestingConnection || isConnecting} class="h-9 px-4 rounded-md text-sm border border-emerald-500 bg-emerald-500 text-white hover:bg-emerald-600 hover:border-emerald-600 disabled:opacity-60">{isConnecting ? "Connecting..." : editing ? "Save Changes and Connect" : "Save and Connect"}</button>
			</div>
		</div>
	</div>
{/if}
