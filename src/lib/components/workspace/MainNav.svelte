<script lang="ts">
	import { FolderKanban, Home, Star } from '@lucide/svelte';
	import type { MainView } from '$lib/utils/workspace';

	let {
		mainView,
		onSelectView,
	}: {
		mainView: MainView;
		onSelectView: (view: MainView) => void;
	} = $props();
</script>

<aside
	class="w-14 bg-white border-r border-gray-200 flex flex-col items-center py-4 space-y-6 shrink-0 z-20 shadow-[1px_0_5px_rgba(0,0,0,0.02)]"
>
	<button
		onclick={() => onSelectView('sql')}
		class={`transition-colors p-1.5 rounded-md flex items-center justify-center ${mainView === 'sql' ? 'text-gray-900 bg-gray-100 shadow-sm border border-gray-200/50' : 'text-gray-400 hover:text-gray-900 hover:bg-gray-100'}`}
	>
		<Home size={20} />
	</button>
	<button
		onclick={() => onSelectView('saved_queries')}
		class={`transition-colors p-1.5 rounded-md flex items-center justify-center ${mainView === 'saved_queries' ? 'text-gray-900 bg-gray-100 shadow-sm border border-gray-200/50' : 'text-gray-400 hover:text-gray-900 hover:bg-gray-100'}`}
	><Star size={20} /></button
	>
	<button
		onclick={() => onSelectView('last_queries')}
		class={`transition-colors p-1.5 rounded-md flex items-center justify-center ${mainView === 'last_queries' ? 'text-gray-900 bg-gray-100 shadow-sm border border-gray-200/50' : 'text-gray-400 hover:text-gray-900 hover:bg-gray-100'}`}
	><FolderKanban size={20} /></button
	>
</aside>
