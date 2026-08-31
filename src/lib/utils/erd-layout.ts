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

function median(values: number[]): number {
	if (values.length === 0) return 0;
	const sorted = [...values].sort((a, b) => a - b);
	const mid = Math.floor(sorted.length / 2);
	return sorted.length % 2 === 0 ? (sorted[mid - 1] + sorted[mid]) / 2 : sorted[mid];
}

function countInversions(pairs: Array<[number, number]>): number {
	pairs.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
	let crossings = 0;
	for (let i = 0; i < pairs.length; i++) {
		for (let j = i + 1; j < pairs.length; j++) {
			if (pairs[i][1] > pairs[j][1]) crossings += 1;
		}
	}
	return crossings;
}

// Sugiyama-style layout with Graphviz-inspired refinements:
// min-rank compaction, median + transpose crossing reduction, then
// two-way neighbor alignment with overlap resolution.
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

	const degree = nodes.map((_, i) => parentSets[i].size + childSets[i].size);
	const nameKey = (i: number) => `${nodes[i].schema}.${nodes[i].table}`;

	const indegree = parentSets.map((set) => set.size);
	const queue: number[] = [];
	for (let i = 0; i < n; i++) {
		if (indegree[i] === 0) queue.push(i);
	}
	queue.sort((a, b) => degree[b] - degree[a] || nameKey(a).localeCompare(nameKey(b)));
	const inTopo = new Array<boolean>(n).fill(false);
	const topo: number[] = [];
	let head = 0;
	while (head < queue.length) {
		const i = queue[head++];
		topo.push(i);
		inTopo[i] = true;
		const next: number[] = [];
		for (const child of childSets[i]) {
			indegree[child]--;
			if (indegree[child] === 0) next.push(child);
		}
		next.sort((a, b) => degree[b] - degree[a] || nameKey(a).localeCompare(nameKey(b)));
		queue.push(...next);
	}
	for (let i = 0; i < n; i++) {
		if (!inTopo[i]) topo.push(i);
	}

	const layer = new Array<number>(n).fill(0);
	for (const i of topo) {
		let minParent = 0;
		for (const parent of parentSets[i]) {
			minParent = Math.max(minParent, layer[parent] + 1);
		}
		layer[i] = minParent;
	}
	// Pull nodes toward their children so long FK chains stay compact
	for (let t = topo.length - 1; t >= 0; t--) {
		const i = topo[t];
		if (childSets[i].size === 0) continue;
		let minChild = Infinity;
		for (const child of childSets[i]) minChild = Math.min(minChild, layer[child]);
		if (Number.isFinite(minChild) && minChild - 1 > layer[i] && parentSets[i].size === 0) {
			layer[i] = minChild - 1;
		}
	}

	const maxLayer = Math.max(0, ...layer);
	const columns: number[][] = Array.from({ length: maxLayer + 1 }, () => []);
	nodes.forEach((_, i) => columns[layer[i]].push(i));
	for (const column of columns) {
		column.sort((a, b) => degree[b] - degree[a] || nameKey(a).localeCompare(nameKey(b)));
	}

	const pos = new Array<number>(n).fill(0);
	const refreshPositions = (column: number[]) => {
		column.forEach((nodeIndex, position) => {
			pos[nodeIndex] = position;
		});
	};
	columns.forEach(refreshPositions);

	const medianSort = (column: number[], neighborsOf: (i: number) => Set<number>) => {
		const score = new Map<number, number>();
		for (const nodeIndex of column) {
			const neighborPos = [...neighborsOf(nodeIndex)]
				.filter((neighbor) => layer[neighbor] !== layer[nodeIndex])
				.map((neighbor) => pos[neighbor]);
			score.set(nodeIndex, neighborPos.length === 0 ? pos[nodeIndex] : median(neighborPos));
		}
		column.sort(
			(a, b) => (score.get(a) ?? 0) - (score.get(b) ?? 0) || pos[a] - pos[b] || nameKey(a).localeCompare(nameKey(b)),
		);
		refreshPositions(column);
	};

	const adjacentPairs = (left: number[], right: number[]): Array<[number, number]> => {
		const rightSet = new Set(right);
		const pairs: Array<[number, number]> = [];
		for (const i of left) {
			for (const j of childSets[i]) {
				if (rightSet.has(j)) pairs.push([pos[i], pos[j]]);
			}
			for (const j of parentSets[i]) {
				if (rightSet.has(j)) pairs.push([pos[i], pos[j]]);
			}
		}
		return pairs;
	};

	const twoColumnCrossings = (a: number[], b: number[]) => countInversions(adjacentPairs(a, b));

	const localCrossings = (c: number) => {
		let sum = 0;
		if (c > 0) sum += twoColumnCrossings(columns[c - 1], columns[c]);
		if (c < columns.length - 1) sum += twoColumnCrossings(columns[c], columns[c + 1]);
		return sum;
	};

	for (let sweep = 0; sweep < 8; sweep++) {
		if (sweep % 2 === 0) {
			for (let c = 1; c < columns.length; c++) medianSort(columns[c], (i) => parentSets[i]);
		} else {
			for (let c = columns.length - 2; c >= 0; c--) medianSort(columns[c], (i) => childSets[i]);
		}
	}

	let improved = true;
	let guard = 0;
	while (improved && guard < 16) {
		improved = false;
		guard += 1;
		for (let c = 0; c < columns.length; c++) {
			const column = columns[c];
			for (let k = 0; k < column.length - 1; k++) {
				const before = localCrossings(c);
				const tmp = column[k];
				column[k] = column[k + 1];
				column[k + 1] = tmp;
				refreshPositions(column);
				const after = localCrossings(c);
				if (after < before) {
					improved = true;
				} else {
					column[k + 1] = column[k];
					column[k] = tmp;
					refreshPositions(column);
				}
			}
		}
	}

	const columnHeights = columns.map((column) => {
		if (column.length === 0) return 0;
		const heightSum = column.reduce((sum, i) => sum + nodes[i].height, 0);
		return heightSum + Math.max(0, column.length - 1) * LAYER_GAP_Y;
	});
	const tallest = Math.max(0, ...columnHeights);
	columns.forEach((column, c) => {
		let y = (tallest - columnHeights[c]) / 2;
		for (const nodeIndex of column) {
			const node = nodes[nodeIndex];
			node.x = c * (NODE_WIDTH + LAYER_GAP_X);
			node.y = y;
			y += node.height + LAYER_GAP_Y;
		}
	});

	const neighborCenters = (i: number) => {
		const centers: number[] = [];
		for (const neighbor of parentSets[i]) {
			centers.push(nodes[neighbor].y + nodes[neighbor].height / 2);
		}
		for (const neighbor of childSets[i]) {
			centers.push(nodes[neighbor].y + nodes[neighbor].height / 2);
		}
		return centers;
	};

	for (let pass = 0; pass < 6; pass++) {
		const order = pass % 2 === 0 ? columns : [...columns].reverse();
		for (const column of order) {
			for (let k = 0; k < column.length; k++) {
				const node = nodes[column[k]];
				const centers = neighborCenters(column[k]);
				if (centers.length === 0) continue;
				const targetY = median(centers) - node.height / 2;
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

	for (const column of columns) {
		if (column.length === 0) continue;
		column.sort((a, b) => nodes[a].y - nodes[b].y);
		let y = nodes[column[0]].y;
		for (let k = 0; k < column.length; k++) {
			const node = nodes[column[k]];
			if (k === 0) {
				y = node.y;
			} else {
				y = Math.max(y, node.y);
			}
			node.y = y;
			y += node.height + LAYER_GAP_Y;
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
