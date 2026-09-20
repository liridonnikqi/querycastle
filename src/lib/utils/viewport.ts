export type Point = { x: number; y: number };

/** Keep a menu of known size inside the viewport. */
export function clampMenuPosition(
	x: number,
	y: number,
	width: number,
	height: number,
	viewportWidth: number,
	viewportHeight: number,
	pad = 8,
): Point {
	const maxX = Math.max(pad, viewportWidth - width - pad);
	const maxY = Math.max(pad, viewportHeight - height - pad);
	return {
		x: Math.min(Math.max(pad, x), maxX),
		y: Math.min(Math.max(pad, y), maxY),
	};
}

export type AnchorRect = {
	left: number;
	top: number;
	right: number;
	bottom: number;
	width: number;
	height: number;
};

/** Place a tooltip below an anchor, flipping above if it would overflow. */
export function clampTooltipToAnchor(
	anchor: AnchorRect,
	width: number,
	height: number,
	viewportWidth: number,
	viewportHeight: number,
	pad = 8,
	gap = 6,
): Point {
	let left = anchor.left + (anchor.width - width) / 2;
	let top = anchor.bottom + gap;
	if (top + height > viewportHeight - pad) {
		top = anchor.top - height - gap;
	}
	return clampMenuPosition(
		left,
		top,
		width,
		height,
		viewportWidth,
		viewportHeight,
		pad,
	);
}

/** Svelte action: pin a fixed menu to (x, y) without leaving the window. */
export function fitToViewport(node: HTMLElement, coords: Point) {
	const apply = () => {
		const rect = node.getBoundingClientRect();
		const next = clampMenuPosition(
			coords.x,
			coords.y,
			rect.width,
			rect.height,
			window.innerWidth,
			window.innerHeight,
		);
		node.style.left = `${next.x}px`;
		node.style.top = `${next.y}px`;
	};
	apply();
	const frame = requestAnimationFrame(apply);
	return {
		update(next: Point) {
			coords = next;
			apply();
		},
		destroy() {
			cancelAnimationFrame(frame);
		},
	};
}
