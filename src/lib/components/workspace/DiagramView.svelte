<script lang="ts">
	import { onMount } from 'svelte';
	import {
		LayoutGrid,
		Maximize2,
		Minus,
		Network,
		Plus,
		RefreshCw,
		Search,
		Table2,
		X,
	} from '@lucide/svelte';
	import type { ConnectionStatus, DatabaseExplorer } from '$lib/rpc';
	import type { TableAction } from '$lib/utils/workspace';
	import {
		HEADER_HEIGHT,
		ROW_HEIGHT,
		computeEdgeGeometry,
		layoutDiagram,
		truncateNodeName,
		truncateTypeName,
		type ErdEdge,
		type ErdLayout,
		type ErdNode,
	} from '$lib/utils/erd-layout';

	let {
		connectionStatus,
		explorer,
		loadingExplorer,
		onRefreshTables,
		onTableAction,
	}: {
		connectionStatus: ConnectionStatus;
		explorer: DatabaseExplorer | null;
		loadingExplorer: boolean;
		onRefreshTables: () => void | Promise<void>;
		onTableAction: (
			action: TableAction,
			schema: string,
			table: string,
		) => void | Promise<void>;
	} = $props();

	const MIN_SCALE = 0.1;
	const MAX_SCALE = 2.5;

	let layout = $state<ErdLayout>({ nodes: [], edges: [], width: 0, height: 0 });
	let viewport = $state({ x: 40, y: 40, scale: 1 });
	let selectedNodeId = $state<string | null>(null);
	let hoveredEdgeId = $state<string | null>(null);
	let search = $state('');
	let isRefreshing = $state(false);
	let tooltip = $state<{ x: number; y: number; text: string } | null>(null);
	let containerEl: HTMLDivElement | null = $state(null);
	let svgEl: SVGSVGElement | null = $state(null);
	let hasFitted = false;

	let panState: {
		pointerId: number;
		startX: number;
		startY: number;
		originX: number;
		originY: number;
	} | null = null;
	let dragState: {
		pointerId: number;
		nodeId: string;
		startPointerX: number;
		startPointerY: number;
		originX: number;
		originY: number;
		moved: boolean;
	} | null = null;

	let nodeById = $derived(new Map(layout.nodes.map((node) => [node.id, node])));
	let showSchemaBadges = $derived(
		new Set(layout.nodes.map((node) => node.schema)).size > 1,
	);
	let searchQuery = $derived(search.trim().toLowerCase());
	let matchingNodeIds = $derived.by(() => {
		if (!searchQuery) return null;
		const set = new Set<string>();
		for (const node of layout.nodes) {
			if (
				node.table.toLowerCase().includes(searchQuery) ||
				node.schema.toLowerCase().includes(searchQuery)
			) {
				set.add(node.id);
			}
		}
		return set;
	});
	let neighborIds = $derived.by(() => {
		if (!selectedNodeId) return null;
		const set = new Set<string>([selectedNodeId]);
		for (const edge of layout.edges) {
			if (edge.sourceId === selectedNodeId) set.add(edge.targetId);
			if (edge.targetId === selectedNodeId) set.add(edge.sourceId);
		}
		return set;
	});

	$effect(() => {
		// Pass the fresh layout to fitToScreen explicitly: reading the `layout`
		// state inside this effect would make the effect re-run on its own write.
		const next = layoutDiagram(explorer);
		layout = next;
		selectedNodeId = null;
		hoveredEdgeId = null;
		tooltip = null;
		hasFitted = false;
		fitToScreen(next);
	});

	function fitToScreen(target: ErdLayout = layout) {
		if (!containerEl || target.nodes.length === 0) {
			viewport = { x: 40, y: 40, scale: 1 };
			return;
		}
		const cw = containerEl.clientWidth;
		const ch = containerEl.clientHeight;
		if (cw === 0 || ch === 0) return;
		const padding = 48;
		const fitScale = Math.min(
			(cw - padding * 2) / target.width,
			(ch - padding * 2) / target.height,
		);
		// Keep the initial view readable: never zoom below 30% just to fit
		// everything, and never enlarge past 100%.
		const clamped = Math.max(0.3, Math.min(fitScale, 1));
		viewport = {
			x: (cw - target.width * clamped) / 2,
			y: (ch - target.height * clamped) / 2,
			scale: clamped,
		};
		hasFitted = true;
	}

	function applyZoom(factor: number, cx: number, cy: number) {
		const next = Math.max(
			MIN_SCALE,
			Math.min(viewport.scale * factor, MAX_SCALE),
		);
		if (next === viewport.scale) return;
		const ratio = next / viewport.scale;
		viewport = {
			x: cx - (cx - viewport.x) * ratio,
			y: cy - (cy - viewport.y) * ratio,
			scale: next,
		};
	}

	function zoomBy(factor: number) {
		if (!containerEl) return;
		applyZoom(factor, containerEl.clientWidth / 2, containerEl.clientHeight / 2);
	}

	function handleRelayout() {
		layout = layoutDiagram(explorer);
		selectedNodeId = null;
		hoveredEdgeId = null;
		hasFitted = false;
		fitToScreen();
	}

	async function handleRefresh() {
		isRefreshing = true;
		try {
			await onRefreshTables();
		} finally {
			isRefreshing = false;
		}
	}

	function onCanvasPointerDown(event: PointerEvent) {
		if (event.button !== 0) return;
		tooltip = null;
		svgEl?.setPointerCapture(event.pointerId);
		panState = {
			pointerId: event.pointerId,
			startX: event.clientX,
			startY: event.clientY,
			originX: viewport.x,
			originY: viewport.y,
		};
	}

	function onCanvasPointerMove(event: PointerEvent) {
		if (panState && event.pointerId === panState.pointerId) {
			viewport = {
				...viewport,
				x: panState.originX + (event.clientX - panState.startX),
				y: panState.originY + (event.clientY - panState.startY),
			};
			return;
		}
		if (dragState && event.pointerId === dragState.pointerId) {
			const dx = (event.clientX - dragState.startPointerX) / viewport.scale;
			const dy = (event.clientY - dragState.startPointerY) / viewport.scale;
			if (
				Math.abs(event.clientX - dragState.startPointerX) > 3 ||
				Math.abs(event.clientY - dragState.startPointerY) > 3
			) {
				dragState.moved = true;
			}
			const node = nodeById.get(dragState.nodeId);
			if (node) {
				node.x = Math.round(dragState.originX + dx);
				node.y = Math.round(dragState.originY + dy);
			}
		}
	}

	function onCanvasPointerUp(event: PointerEvent) {
		if (panState && event.pointerId === panState.pointerId) {
			const moved =
				Math.abs(event.clientX - panState.startX) > 3 ||
				Math.abs(event.clientY - panState.startY) > 3;
			panState = null;
			if (!moved) selectedNodeId = null;
		}
		if (dragState && event.pointerId === dragState.pointerId) {
			if (!dragState.moved) {
				selectedNodeId =
					selectedNodeId === dragState.nodeId ? null : dragState.nodeId;
			}
			dragState = null;
		}
	}

	function onNodePointerDown(event: PointerEvent, node: ErdNode) {
		if (event.button !== 0) return;
		event.stopPropagation();
		svgEl?.setPointerCapture(event.pointerId);
		dragState = {
			pointerId: event.pointerId,
			nodeId: node.id,
			startPointerX: event.clientX,
			startPointerY: event.clientY,
			originX: node.x,
			originY: node.y,
			moved: false,
		};
	}

	function onNodeDoubleClick(event: MouseEvent, node: ErdNode) {
		event.stopPropagation();
		void onTableAction('view_data', node.schema, node.table);
	}

	function edgeGeometry(edge: ErdEdge) {
		const source = nodeById.get(edge.sourceId);
		const target = nodeById.get(edge.targetId);
		if (!source || !target) return null;
		return computeEdgeGeometry(edge, source, target);
	}

	function nodeOpacity(node: ErdNode): number {
		let opacity = 1;
		if (neighborIds && !neighborIds.has(node.id)) opacity = 0.25;
		if (matchingNodeIds && !matchingNodeIds.has(node.id)) {
			opacity = Math.min(opacity, 0.2);
		}
		return opacity;
	}

	function edgeOpacity(edge: ErdEdge): number {
		let opacity = 1;
		if (selectedNodeId) {
			const touches =
				edge.sourceId === selectedNodeId || edge.targetId === selectedNodeId;
			opacity = touches ? 1 : 0.12;
		}
		if (matchingNodeIds) {
			const touches =
				matchingNodeIds.has(edge.sourceId) || matchingNodeIds.has(edge.targetId);
			opacity = Math.min(opacity, touches ? 1 : 0.12);
		}
		return opacity;
	}

	function isEdgeActive(edge: ErdEdge): boolean {
		if (hoveredEdgeId === edge.id) return true;
		if (!selectedNodeId) return false;
		return edge.sourceId === selectedNodeId || edge.targetId === selectedNodeId;
	}

	function onEdgePointerMove(event: PointerEvent, edge: ErdEdge) {
		if (panState || dragState) return;
		if (!containerEl) return;
		const rect = containerEl.getBoundingClientRect();
		const source = nodeById.get(edge.sourceId);
		const target = nodeById.get(edge.targetId);
		hoveredEdgeId = edge.id;
		tooltip = {
			x: event.clientX - rect.left,
			y: event.clientY - rect.top,
			text: `${source?.table ?? edge.sourceId}.${edge.sourceColumn} → ${target?.table ?? edge.targetId}.${edge.targetColumn}`,
		};
	}

	function onEdgePointerLeave() {
		hoveredEdgeId = null;
		tooltip = null;
	}

	function columnBadges(column: { isPrimary: boolean; isForeign: boolean }) {
		const badges: Array<'PK' | 'FK'> = [];
		if (column.isPrimary) badges.push('PK');
		if (column.isForeign) badges.push('FK');
		return badges;
	}

	onMount(() => {
		const el = containerEl;
		if (!el) return;
		const onWheel = (event: WheelEvent) => {
			event.preventDefault();
			const rect = el.getBoundingClientRect();
			const factor = Math.exp(-event.deltaY * 0.0015);
			applyZoom(factor, event.clientX - rect.left, event.clientY - rect.top);
		};
		const onKeyDown = (event: KeyboardEvent) => {
			if (event.key === 'Escape') {
				selectedNodeId = null;
				hoveredEdgeId = null;
				tooltip = null;
			}
		};
		// Retry the initial fit if the container had no size on first layout
		const resizeObserver = new ResizeObserver(() => {
			if (!hasFitted && el.clientWidth > 0 && el.clientHeight > 0) {
				fitToScreen();
			}
		});
		el.addEventListener('wheel', onWheel, { passive: false });
		window.addEventListener('keydown', onKeyDown);
		resizeObserver.observe(el);
		return () => {
			el.removeEventListener('wheel', onWheel);
			window.removeEventListener('keydown', onKeyDown);
			resizeObserver.disconnect();
		};
	});
</script>

<section class="flex-1 min-w-0 flex flex-col bg-gray-50">
	<div
		class="h-12 border-b border-gray-200 bg-white flex items-center justify-between px-3 gap-3 shrink-0"
	>
		<div class="flex items-center gap-2 min-w-0">
			<Network size={15} class="text-gray-500 shrink-0" />
			<span class="text-sm font-semibold text-gray-800 whitespace-nowrap"
				>Schema diagram</span
			>
			{#if connectionStatus.connected}
				<span
					class="text-[11px] px-1.5 py-0.5 rounded-full bg-gray-100 border border-gray-200 text-gray-600 truncate"
					>{connectionStatus.database}</span
				>
				<span class="text-[11px] text-gray-400 whitespace-nowrap">
					{layout.nodes.length} tables · {layout.edges.length} relations
				</span>
			{/if}
		</div>
		{#if connectionStatus.connected && layout.nodes.length > 0}
			<div class="flex items-center gap-1.5">
				<div class="relative flex items-center w-56">
					<Search size={14} class="w-4 h-4 absolute left-2.5 text-gray-400" />
					<input
						type="text"
						placeholder="Search..."
						bind:value={search}
						class="w-full h-8 bg-white border border-gray-200 text-gray-900 text-sm rounded-md block pl-8 pr-8 py-1.5 placeholder-gray-400 focus:outline-none hover:border-gray-300 focus:border-gray-300 focus:ring-1 focus:ring-gray-200"
					/>
					{#if search}
						<button
							onclick={() => (search = '')}
							class="absolute right-2 w-4 h-4 rounded-full bg-gray-100 hover:bg-gray-200 text-gray-500 flex items-center justify-center"
							aria-label="Clear search"
						>
							<X size={10} />
						</button>
					{/if}
				</div>
				<button
					onclick={handleRelayout}
					title="Re-layout"
					class="w-8 h-8 rounded-lg border border-gray-200 bg-white flex items-center justify-center text-gray-500 hover:text-gray-900 hover:bg-gray-50"
				>
					<LayoutGrid size={14} />
				</button>
				<button
					onclick={handleRefresh}
					title="Refresh schema"
					disabled={isRefreshing || loadingExplorer}
					class="w-8 h-8 rounded-lg border border-gray-200 bg-white flex items-center justify-center text-gray-500 hover:text-gray-900 hover:bg-gray-50 disabled:opacity-50"
				>
					<RefreshCw size={14} class={isRefreshing || loadingExplorer ? 'animate-spin' : ''} />
				</button>
			</div>
		{/if}
	</div>

	<div class="flex-1 relative overflow-hidden" bind:this={containerEl}>
		{#if !connectionStatus.connected}
			<div class="h-full flex flex-col items-center justify-center text-center p-6">
				<Network size={22} class="text-gray-300 mb-2" />
				<div class="text-sm text-gray-500">No active connection</div>
				<div class="text-xs text-gray-400 mt-1">
					Connect to a database to see its schema diagram
				</div>
			</div>
		{:else if loadingExplorer && layout.nodes.length === 0}
			<div class="h-full flex flex-col items-center justify-center text-center p-6">
				<div
					class="w-6 h-6 border-2 border-gray-300 border-t-gray-600 rounded-full animate-spin mb-3"
				></div>
				<div class="text-xs text-gray-500">Loading schema...</div>
			</div>
		{:else if layout.nodes.length === 0}
			<div class="h-full flex flex-col items-center justify-center text-center p-6">
				<Table2 size={22} class="text-gray-300 mb-2" />
				<div class="text-sm text-gray-500">No tables found</div>
				<div class="text-xs text-gray-400 mt-1">
					This database has no tables to diagram yet
				</div>
			</div>
		{:else}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<svg
				bind:this={svgEl}
				class="w-full h-full block touch-none select-none cursor-grab active:cursor-grabbing"
				onpointerdown={onCanvasPointerDown}
				onpointermove={onCanvasPointerMove}
				onpointerup={onCanvasPointerUp}
				onpointercancel={onCanvasPointerUp}
			>
				<defs>
					<pattern
						id="erd-dot-grid"
						width="24"
						height="24"
						patternUnits="userSpaceOnUse"
					>
						<circle cx="1.2" cy="1.2" r="1.2" fill="#e5e7eb" />
					</pattern>
				</defs>
				<g
					transform={`translate(${viewport.x}, ${viewport.y}) scale(${viewport.scale})`}
				>
					<rect
						x={-20000}
						y={-20000}
						width={40000 + layout.width}
						height={40000 + layout.height}
						fill="url(#erd-dot-grid)"
					/>
					{#each layout.edges as edge (edge.id)}
						{@const geometry = edgeGeometry(edge)}
						{#if geometry}
							{@const active = isEdgeActive(edge)}
							<g opacity={edgeOpacity(edge)}>
								<path
									d={geometry.path}
									fill="none"
									stroke={active ? '#3b82f6' : '#9ca3af'}
									stroke-width={active ? 2.2 : 1.4}
								/>
								{#if !edge.selfLoop}
									<!-- crow's foot on the FK (many) side -->
									<path
										d={`M ${geometry.sourceAnchor.x + geometry.sourceDir * 13} ${geometry.sourceAnchor.y} L ${geometry.sourceAnchor.x} ${geometry.sourceAnchor.y - 6} M ${geometry.sourceAnchor.x + geometry.sourceDir * 13} ${geometry.sourceAnchor.y} L ${geometry.sourceAnchor.x} ${geometry.sourceAnchor.y} M ${geometry.sourceAnchor.x + geometry.sourceDir * 13} ${geometry.sourceAnchor.y} L ${geometry.sourceAnchor.x} ${geometry.sourceAnchor.y + 6}`}
										stroke={active ? '#3b82f6' : '#9ca3af'}
										stroke-width={active ? 1.8 : 1.3}
										fill="none"
									/>
									<!-- "one" tick + arrow on the referenced side -->
									<path
										d={`M ${geometry.targetAnchor.x - geometry.targetDir * 9} ${geometry.targetAnchor.y - 4.5} L ${geometry.targetAnchor.x} ${geometry.targetAnchor.y} L ${geometry.targetAnchor.x - geometry.targetDir * 9} ${geometry.targetAnchor.y + 4.5}`}
										stroke={active ? '#3b82f6' : '#9ca3af'}
										stroke-width={active ? 1.8 : 1.3}
										fill="none"
									/>
								{/if}
								<path
									d={geometry.path}
									fill="none"
									stroke="transparent"
									stroke-width={12}
									class="cursor-pointer"
									onpointermove={(e) => onEdgePointerMove(e, edge)}
									onpointerleave={onEdgePointerLeave}
								/>
							</g>
						{/if}
					{/each}
					{#each layout.nodes as node (node.id)}
						{@const selected = selectedNodeId === node.id}
						<g
							transform={`translate(${node.x}, ${node.y})`}
							opacity={nodeOpacity(node)}
							class="cursor-grab"
							onpointerdown={(e) => onNodePointerDown(e, node)}
							ondblclick={(e) => onNodeDoubleClick(e, node)}
						>
							<rect
								width={node.width}
								height={node.height}
								rx={8}
								fill="white"
								stroke={selected ? '#3b82f6' : '#d1d5db'}
								stroke-width={selected ? 1.8 : 1}
							/>
							<path
								d={`M 0 8 a 8 8 0 0 1 8 -8 H ${node.width - 8} a 8 8 0 0 1 8 8 V ${HEADER_HEIGHT} H 0 Z`}
								fill={selected ? '#eff6ff' : '#f9fafb'}
							/>
							<line
								x1={0}
								y1={HEADER_HEIGHT}
								x2={node.width}
								y2={HEADER_HEIGHT}
								stroke="#e5e7eb"
							/>
							<text
								x={10}
								y={19}
								font-size="12"
								font-weight="600"
								fill="#111827"
							>{truncateNodeName(node.table)}</text>
							{#if showSchemaBadges}
								<text
									x={node.width - 10}
									y={19}
									text-anchor="end"
									font-size="9"
									fill="#6b7280"
								>{node.schema}</text>
							{/if}
							{#each node.columns as column, i}
								{@const rowY = HEADER_HEIGHT + i * ROW_HEIGHT}
								{@const badges = columnBadges(column)}
								{@const nameX = 10 + badges.length * 23}
								{#each badges as badge, b}
									<rect
										x={10 + b * 23}
										y={rowY + 5}
										width={20}
										height={12}
										rx={3}
										fill={badge === 'PK' ? '#fef3c7' : '#dbeafe'}
										stroke={badge === 'PK' ? '#fcd34d' : '#93c5fd'}
										stroke-width={0.75}
									/>
									<text
										x={20 + b * 23}
										y={rowY + 14}
										text-anchor="middle"
										font-size="8"
										font-weight="700"
										fill={badge === 'PK' ? '#92400e' : '#1e40af'}
									>{badge}</text>
								{/each}
								<text
									x={nameX}
									y={rowY + 15}
									font-size="11"
									fill={column.isPrimary ? '#111827' : '#374151'}
									font-weight={column.isPrimary ? 600 : 400}
								>{truncateNodeName(column.name)}{column.notNull && !column.isPrimary ? ' *' : ''}</text>
								<text
									x={node.width - 10}
									y={rowY + 15}
									text-anchor="end"
									font-size="10"
									fill="#9ca3af"
									font-family="ui-monospace, monospace"
								>{truncateTypeName(column.dataType)}</text>
							{/each}
						</g>
					{/each}
				</g>
			</svg>
			<div class="absolute left-3 bottom-3 z-10 flex flex-col items-center gap-1">
				<button
					onclick={() => zoomBy(1.25)}
					title="Zoom in"
					class="w-7 h-7 flex items-center justify-center bg-white border border-gray-200 rounded-md text-gray-500 hover:text-gray-900 hover:bg-gray-50"
				>
					<Plus size={13} />
				</button>
				<button
					onclick={() => zoomBy(1 / 1.25)}
					title="Zoom out"
					class="w-7 h-7 flex items-center justify-center bg-white border border-gray-200 rounded-md text-gray-500 hover:text-gray-900 hover:bg-gray-50"
				>
					<Minus size={13} />
				</button>
				<button
					onclick={() => fitToScreen()}
					title="Fit to screen"
					class="w-7 h-7 flex items-center justify-center bg-white border border-gray-200 rounded-md text-gray-500 hover:text-gray-900 hover:bg-gray-50"
				>
					<Maximize2 size={13} />
				</button>
			</div>
			{#if layout.edges.length === 0}
				<div
					class="absolute bottom-3 left-1/2 -translate-x-1/2 px-3 py-1.5 rounded-full bg-white border border-gray-200 shadow-sm text-[11px] text-gray-500 pointer-events-none"
				>
					No foreign-key relationships found
				</div>
			{/if}
			{#if tooltip}
				<div
					class="absolute pointer-events-none z-10 px-2 py-1 rounded-md bg-gray-900 text-white text-[11px] font-mono shadow-lg whitespace-nowrap"
					style={`left: ${tooltip.x + 12}px; top: ${tooltip.y + 12}px;`}
				>
					{tooltip.text}
				</div>
			{/if}
		{/if}
	</div>
</section>
