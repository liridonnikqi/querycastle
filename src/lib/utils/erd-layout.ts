import type { DatabaseExplorer } from '$lib/rpc';

export const NODE_WIDTH = 240;
export const HEADER_HEIGHT = 30;
export const ROW_HEIGHT = 22;
export const NODE_PADDING_BOTTOM = 8;

const MAX_NAME_CHARS = 26;
const MAX_TYPE_CHARS = 16;
// Horizontal pitch between hierarchy columns / vertical gap inside a column
const LAYER_GAP_X = 170;
const LAYER_GAP_Y = 56;
const COMPONENT_GAP_X = 120;
const COMPONENT_GAP_Y = 90;
const LAYOUT_MARGIN = 60;

export type ErdColumn = {
	name: string;
	dataType: string;
	notNull: boolean;
	isPrimary: boolean;
	isForeign: boolean;
};

export type ErdNode = {
	id: string;
	schema: string;
	table: string;
	x: number;
	y: number;
	width: number;
	height: number;
	columns: ErdColumn[];
	columnRowIndex: Record<string, number>;
};

export type ErdEdge = {
	id: string;
	sourceId: string;
	targetId: string;
	sourceColumn: string;
	targetColumn: string;
	selfLoop: boolean;
	pairIndex: number;
	pairCount: number;
};

export type ErdLayout = {
	nodes: ErdNode[];
	edges: ErdEdge[];
	width: number;
	height: number;
};

export function nodeHeight(columnCount: number): number {
	return HEADER_HEIGHT + columnCount * ROW_HEIGHT + NODE_PADDING_BOTTOM;
}

export function truncateLabel(value: string, maxChars: number): string {
	if (value.length <= maxChars) return value;
	return `${value.slice(0, maxChars - 1)}…`;
}

export function truncateNodeName(value: string): string {
	return truncateLabel(value, MAX_NAME_CHARS);
}

export function truncateTypeName(value: string): string {
	return truncateLabel(value, MAX_TYPE_CHARS);
}

function buildNodes(explorer: DatabaseExplorer): ErdNode[] {
	const nodes: ErdNode[] = [];
	for (const schema of explorer.schemas) {
		for (const table of schema.tables) {
			if (table.kind !== 'table') continue;
			const fkColumns = new Set(table.foreignKeys.map((fk) => fk.column));
			const columns: ErdColumn[] = table.columns.map((column) => ({
				name: column.name,
				dataType: column.dataType,
				notNull: column.notNull,
				isPrimary: column.isPrimary,
				isForeign: fkColumns.has(column.name),
			}));
			const columnRowIndex: Record<string, number> = {};
			columns.forEach((column, index) => {
				columnRowIndex[column.name] = index;
			});
			nodes.push({
				id: `${schema.name}.${table.name}`,
				schema: schema.name,
				table: table.name,
				x: 0,
				y: 0,
				width: NODE_WIDTH,
				height: nodeHeight(columns.length),
				columns,
				columnRowIndex,
			});
		}
	}
	return nodes;
}

function buildEdges(explorer: DatabaseExplorer, nodeIds: Set<string>): ErdEdge[] {
	const edges: ErdEdge[] = [];
	const pairCounts = new Map<string, number>();
	for (const schema of explorer.schemas) {
		for (const table of schema.tables) {
			if (table.kind !== 'table') continue;
			const sourceId = `${schema.name}.${table.name}`;
			for (const fk of table.foreignKeys) {
				const targetId = `${fk.referencedSchema}.${fk.referencedTable}`;
				if (!nodeIds.has(sourceId) || !nodeIds.has(targetId)) continue;
				const pairKey =
					sourceId <= targetId
						? `${sourceId}|${targetId}`
						: `${targetId}|${sourceId}`;
				pairCounts.set(pairKey, (pairCounts.get(pairKey) ?? 0) + 1);
				edges.push({
					id: `${sourceId}.${fk.column}->${targetId}.${fk.referencedColumn}`,
					sourceId,
					targetId,
					sourceColumn: fk.column,
					targetColumn: fk.referencedColumn,
					selfLoop: sourceId === targetId,
					pairIndex: 0,
					pairCount: 0,
				});
			}
		}
	}
	const pairSeen = new Map<string, number>();
	for (const edge of edges) {
		const pairKey =
			edge.sourceId <= edge.targetId
				? `${edge.sourceId}|${edge.targetId}`
				: `${edge.targetId}|${edge.sourceId}`;
		edge.pairCount = pairCounts.get(pairKey) ?? 1;
		edge.pairIndex = pairSeen.get(pairKey) ?? 0;
		pairSeen.set(pairKey, edge.pairIndex + 1);
	}
	return edges;
}

// Layered (hierarchical) layout: referenced tables sit in the leftmost
// columns, each referencing table one column to the right of its deepest
// parent. Column order is refined with barycenter sweeps to reduce edge
// crossings, then nodes are pulled toward their neighbors' vertical center.
function layoutLayered(nodes: ErdNode[], edges: ErdEdge[]) {
	const n = nodes.length;
	if (n === 0) return;
	if (n === 1) {
		nodes[0].x = 0;
		nodes[0].y = 0;
		return;
	}
	const indexById = new Map<string, number>();
	nodes.forEach((node, index) => indexById.set(node.id, index));

	// FK edge: source (child) references target (parent)
	const parentSets: Set<number>[] = nodes.map(() => new Set());
	const childSets: Set<number>[] = nodes.map(() => new Set());
	for (const edge of edges) {
		if (edge.selfLoop) continue;
		const si = indexById.get(edge.sourceId);
		const ti = indexById.get(edge.targetId);
		if (si === undefined || ti === undefined || si === ti) continue;
		parentSets[si].add(ti);
		childSets[ti].add(si);
	}

	// Topological order (Kahn). Nodes stuck in FK cycles never reach
	// indegree 0; append them in index order so layers stay bounded.
	const indegree = parentSets.map((set) => set.size);
	const queue: number[] = [];
	for (let i = 0; i < n; i++) {
		if (indegree[i] === 0) queue.push(i);
	}
	const inTopo = new Array<boolean>(n).fill(false);
	const topo: number[] = [];
	let head = 0;
	while (head < queue.length) {
		const i = queue[head++];
		topo.push(i);
		inTopo[i] = true;
		for (const child of childSets[i]) {
			indegree[child]--;
			if (indegree[child] === 0) queue.push(child);
		}
	}
	for (let i = 0; i < n; i++) {
		if (!inTopo[i]) topo.push(i);
	}

	// Layer = longest path from any root (table that references nothing)
	const layer = new Array<number>(n).fill(0);
	for (const i of topo) {
		for (const child of childSets[i]) {
			if (layer[child] < layer[i] + 1) layer[child] = layer[i] + 1;
		}
	}
	const maxLayer = Math.max(...layer);
	const columns: number[][] = Array.from({ length: maxLayer + 1 }, () => []);
	nodes.forEach((_, i) => columns[layer[i]].push(i));

	// Position of each node within its column (kept in sync after sorting)
	const pos = new Array<number>(n).fill(0);
	const refreshPositions = (column: number[]) => {
		column.forEach((nodeIndex, position) => {
			pos[nodeIndex] = position;
		});
	};
	columns.forEach(refreshPositions);

	const barycenterSort = (column: number[], neighborsOf: (i: number) => Set<number>) => {
		const barycenter = new Map<number, number>();
		for (const nodeIndex of column) {
			const neighbors = [...neighborsOf(nodeIndex)];
			if (neighbors.length === 0) {
				barycenter.set(nodeIndex, pos[nodeIndex]);
			} else {
				let sum = 0;
				for (const neighbor of neighbors) sum += pos[neighbor];
				barycenter.set(nodeIndex, sum / neighbors.length);
			}
		}
		column.sort(
			(a, b) =>
				(barycenter.get(a) ?? 0) - (barycenter.get(b) ?? 0) || pos[a] - pos[b],
		);
		refreshPositions(column);
	};

	// A few left-right / right-left sweeps to reduce edge crossings
	for (let sweep = 0; sweep < 3; sweep++) {
		for (let c = 1; c < columns.length; c++) {
			barycenterSort(columns[c], (i) => parentSets[i]);
		}
		for (let c = columns.length - 2; c >= 0; c--) {
			barycenterSort(columns[c], (i) => childSets[i]);
		}
	}

	// Coordinates: fixed column pitch, columns centered vertically
	const columnHeights = columns.map((column) => {
		const heightSum = column.reduce((sum, i) => sum + nodes[i].height, 0);
		return heightSum + Math.max(0, column.length - 1) * LAYER_GAP_Y;
	});
	const tallest = Math.max(...columnHeights);
	columns.forEach((column, c) => {
		let y = (tallest - columnHeights[c]) / 2;
		for (const nodeIndex of column) {
			const node = nodes[nodeIndex];
			node.x = c * (NODE_WIDTH + LAYER_GAP_X);
			node.y = y;
			y += node.height + LAYER_GAP_Y;
		}
	});

	// Pull each node toward the vertical center of its neighbors, clamped
	// between its column neighbors so order and spacing are preserved
	for (let pass = 0; pass < 3; pass++) {
		for (let c = 0; c < columns.length; c++) {
			const column = columns[c];
			for (let k = 0; k < column.length; k++) {
				const node = nodes[column[k]];
				const neighbors = [...parentSets[column[k]], ...childSets[column[k]]];
				if (neighbors.length === 0) continue;
				let sum = 0;
				for (const neighbor of neighbors) {
					const other = nodes[neighbor];
					sum += other.y + other.height / 2;
				}
				const targetY = sum / neighbors.length - node.height / 2;
				const minY =
					k > 0
						? nodes[column[k - 1]].y + nodes[column[k - 1]].height + LAYER_GAP_Y
						: -Infinity;
				const maxY =
					k < column.length - 1
						? nodes[column[k + 1]].y - node.height - LAYER_GAP_Y
						: Infinity;
				node.y = Math.max(minY, Math.min(targetY, maxY));
			}
		}
	}
}

type Component = { nodes: ErdNode[]; edges: ErdEdge[] };

function findComponents(nodes: ErdNode[], edges: ErdEdge[]): Component[] {
	const indexById = new Map<string, number>();
	nodes.forEach((node, index) => indexById.set(node.id, index));
	const parent = nodes.map((_, index) => index);
	const find = (start: number): number => {
		let i = start;
		while (parent[i] !== i) {
			parent[i] = parent[parent[i]];
			i = parent[i];
		}
		return i;
	};
	for (const edge of edges) {
		if (edge.selfLoop) continue;
		const si = indexById.get(edge.sourceId);
		const ti = indexById.get(edge.targetId);
		if (si === undefined || ti === undefined) continue;
		const rs = find(si);
		const rt = find(ti);
		if (rs !== rt) parent[rs] = rt;
	}
	const groups = new Map<number, number[]>();
	nodes.forEach((_, index) => {
		const root = find(index);
		const list = groups.get(root) ?? [];
		list.push(index);
		groups.set(root, list);
	});
	const components: Component[] = [];
	for (const memberIndices of groups.values()) {
		const members = new Set(memberIndices);
		const compNodes = memberIndices.map((index) => nodes[index]);
		const compEdges = edges.filter((edge) => {
			const si = indexById.get(edge.sourceId);
			const ti = indexById.get(edge.targetId);
			return si !== undefined && ti !== undefined && members.has(si) && members.has(ti);
		});
		components.push({ nodes: compNodes, edges: compEdges });
	}
	return components;
}

function normalizeToLocal(nodes: ErdNode[]): { width: number; height: number } {
	const minX = Math.min(...nodes.map((node) => node.x));
	const minY = Math.min(...nodes.map((node) => node.y));
	for (const node of nodes) {
		node.x = Math.round(node.x - minX);
		node.y = Math.round(node.y - minY);
	}
	return {
		width: Math.max(...nodes.map((node) => node.x + node.width)),
		height: Math.max(...nodes.map((node) => node.y + node.height)),
	};
}

// Arrange independent components on shelves so unrelated tables sit in tidy
// rows instead of being scattered across one giant canvas.
function packComponents(
	components: Array<{ nodes: ErdNode[]; width: number; height: number }>,
): { width: number; height: number } {
	if (components.length === 0) return { width: 0, height: 0 };
	if (components.length === 1) {
		return { width: components[0].width, height: components[0].height };
	}
	const sorted = [...components].sort(
		(a, b) => b.height - a.height || b.width - a.width,
	);
	const totalArea = sorted.reduce(
		(sum, c) => sum + (c.width + COMPONENT_GAP_X) * (c.height + COMPONENT_GAP_Y),
		0,
	);
	const widest = Math.max(...sorted.map((c) => c.width + COMPONENT_GAP_X));
	const targetRowWidth = Math.max(widest, Math.sqrt(totalArea) * 1.4);
	let cursorX = 0;
	let cursorY = 0;
	let rowHeight = 0;
	let maxWidth = 0;
	for (const comp of sorted) {
		if (cursorX > 0 && cursorX + comp.width > targetRowWidth) {
			cursorX = 0;
			cursorY += rowHeight + COMPONENT_GAP_Y;
			rowHeight = 0;
		}
		for (const node of comp.nodes) {
			node.x += cursorX;
			node.y += cursorY;
		}
		cursorX += comp.width + COMPONENT_GAP_X;
		rowHeight = Math.max(rowHeight, comp.height);
		maxWidth = Math.max(maxWidth, cursorX - COMPONENT_GAP_X);
	}
	return { width: maxWidth, height: cursorY + rowHeight };
}

export function layoutDiagram(explorer: DatabaseExplorer | null): ErdLayout {
	if (!explorer) return { nodes: [], edges: [], width: 0, height: 0 };
	const nodes = buildNodes(explorer);
	const nodeIds = new Set(nodes.map((node) => node.id));
	const edges = buildEdges(explorer, nodeIds);
	if (nodes.length === 0) return { nodes, edges, width: 0, height: 0 };
	const components = findComponents(nodes, edges);
	for (const component of components) {
		layoutLayered(component.nodes, component.edges);
	}
	const placed = components.map((component) => ({
		nodes: component.nodes,
		...normalizeToLocal(component.nodes),
	}));
	const packed = packComponents(placed);
	for (const node of nodes) {
		node.x += LAYOUT_MARGIN;
		node.y += LAYOUT_MARGIN;
	}
	return {
		nodes,
		edges,
		width: packed.width + LAYOUT_MARGIN * 2,
		height: packed.height + LAYOUT_MARGIN * 2,
	};
}

export type EdgeGeometry = {
	path: string;
	sourceAnchor: { x: number; y: number };
	targetAnchor: { x: number; y: number };
	// Unit direction the edge leaves the source / enters the target (horizontal)
	sourceDir: 1 | -1;
	targetDir: 1 | -1;
};

export function columnAnchorY(node: ErdNode, columnName: string): number {
	const rowIndex = node.columnRowIndex[columnName];
	const safeRow = rowIndex === undefined ? 0 : rowIndex;
	return node.y + HEADER_HEIGHT + safeRow * ROW_HEIGHT + ROW_HEIGHT / 2;
}

export function computeEdgeGeometry(
	edge: ErdEdge,
	source: ErdNode,
	target: ErdNode,
): EdgeGeometry {
	if (edge.selfLoop) {
		const sy = columnAnchorY(source, edge.sourceColumn);
		const ty = columnAnchorY(target, edge.targetColumn);
		const x = source.x + source.width;
		const reach = 46 + edge.pairIndex * 18;
		const topY = Math.min(sy, ty);
		const bottomY = Math.max(sy, ty);
		const path = `M ${x} ${sy} C ${x + reach} ${topY - 24}, ${x + reach} ${bottomY + 24}, ${x} ${ty}`;
		return {
			path,
			sourceAnchor: { x, y: sy },
			targetAnchor: { x, y: ty },
			sourceDir: 1,
			targetDir: 1,
		};
	}

	const sourceCenterX = source.x + source.width / 2;
	const targetCenterX = target.x + target.width / 2;
	const spread = Math.max(0, Math.abs(targetCenterX - sourceCenterX) - source.width / 2 - target.width / 2);
	// Prefer leaving from the side facing the target; fall back to right/left sides
	const sourceDir: 1 | -1 = targetCenterX >= sourceCenterX ? 1 : -1;
	const targetDir: 1 | -1 = sourceDir === 1 ? -1 : 1;
	const sx = sourceDir === 1 ? source.x + source.width : source.x;
	const tx = targetDir === -1 ? target.x : target.x + target.width;
	const sy = columnAnchorY(source, edge.sourceColumn);
	const ty = columnAnchorY(target, edge.targetColumn);
	const bend = Math.max(48, spread / 2) + edge.pairIndex * 14;
	const path = `M ${sx} ${sy} C ${sx + sourceDir * bend} ${sy}, ${tx - targetDir * bend} ${ty}, ${tx} ${ty}`;
	return {
		path,
		sourceAnchor: { x: sx, y: sy },
		targetAnchor: { x: tx, y: ty },
		sourceDir,
		targetDir,
	};
}
