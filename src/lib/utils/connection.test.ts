import { describe, expect, it, beforeEach } from 'vitest';
import {
	connectionMetaLine,
	defaultsForType,
	loadRecentConnectionNames,
	rememberRecentConnection,
} from '$lib/utils/connection';

describe('connectionMetaLine', () => {
	it('shows engine and host for postgres', () => {
		expect(connectionMetaLine(defaultsForType('postgres'))).toBe('postgres · localhost');
	});

	it('shows sqlite file name', () => {
		const connection = { ...defaultsForType('sqlite'), database: 'C:/data/demo.db' };
		expect(connectionMetaLine(connection)).toBe('sqlite · demo.db');
	});
});

describe('recent connections', () => {
	const memory = new Map<string, string>();

	beforeEach(() => {
		memory.clear();
		Object.defineProperty(globalThis, 'localStorage', {
			configurable: true,
			value: {
				getItem: (key: string) => memory.get(key) ?? null,
				setItem: (key: string, value: string) => {
					memory.set(key, value);
				},
				removeItem: (key: string) => {
					memory.delete(key);
				},
			},
		});
	});

	it('prepends unique names and caps the list', () => {
		rememberRecentConnection('alpha');
		rememberRecentConnection('beta');
		rememberRecentConnection('alpha');
		expect(loadRecentConnectionNames()).toEqual(['alpha', 'beta']);
	});
});
