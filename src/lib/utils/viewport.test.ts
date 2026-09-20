import { describe, expect, it } from 'vitest';
import { clampMenuPosition, clampTooltipToAnchor } from '$lib/utils/viewport';

describe('clampMenuPosition', () => {
	it('keeps a menu that already fits', () => {
		expect(clampMenuPosition(40, 50, 180, 220, 800, 600)).toEqual({
			x: 40,
			y: 50,
		});
	});

	it('flips away from the right and bottom edges', () => {
		expect(clampMenuPosition(760, 520, 180, 220, 800, 600)).toEqual({
			x: 612,
			y: 372,
		});
	});

	it('pins oversized menus to the padding', () => {
		expect(clampMenuPosition(0, 0, 900, 700, 800, 600, 8)).toEqual({
			x: 8,
			y: 8,
		});
	});
});

describe('clampTooltipToAnchor', () => {
	it('centers below the anchor', () => {
		expect(
			clampTooltipToAnchor(
				{ left: 100, top: 40, right: 140, bottom: 68, width: 40, height: 28 },
				120,
				32,
				800,
				600,
			),
		).toEqual({ x: 60, y: 74 });
	});

	it('flips above when the anchor is near the bottom', () => {
		expect(
			clampTooltipToAnchor(
				{ left: 100, top: 560, right: 140, bottom: 588, width: 40, height: 28 },
				120,
				32,
				800,
				600,
			),
		).toEqual({ x: 60, y: 522 });
	});
});
