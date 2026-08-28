<script lang="ts">
	import { History, Home, Network, Star } from '@lucide/svelte';
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
	class="w-12 bg-white border-r border-gray-200 flex flex-col items-center py-3 space-y-4 shrink-0 z-20 shadow-[1px_0_5px_rgba(0,0,0,0.02)]"
>
	<button
		onclick={() => onSelectView('sql')}
		class={`transition-colors p-1.5 rounded-md flex items-center justify-center ${mainView === 'sql' ? 'text-gray-900 bg-gray-100 border border-gray-200/50' : 'text-gray-400 hover:text-gray-900 hover:bg-gray-100'}`}
	>
		<Home size={18} />
	</button>
	<button
		onclick={() => onSelectView('diagram')}
		title="Schema diagram"
		class={`transition-colors p-1.5 rounded-md flex items-center justify-center ${mainView === 'diagram' ? 'text-gray-900 bg-gray-100 border border-gray-200/50' : 'text-gray-400 hover:text-gray-900 hover:bg-gray-100'}`}
	><Network size={18} /></button
	>
	<button
		onclick={() => onSelectView('saved_queries')}
		class={`transition-colors p-1.5 rounded-md flex items-center justify-center ${mainView === 'saved_queries' ? 'text-gray-900 bg-gray-100 border border-gray-200/50' : 'text-gray-400 hover:text-gray-900 hover:bg-gray-100'}`}
	><Star size={18} /></button
	>
	<button
		onclick={() => onSelectView('history')}
		title="History"
		class={`transition-colors p-1.5 rounded-md flex items-center justify-center ${mainView === 'history' ? 'text-gray-900 bg-gray-100 border border-gray-200/50' : 'text-gray-400 hover:text-gray-900 hover:bg-gray-100'}`}
	><History size={18} /></button
	>
</aside>
