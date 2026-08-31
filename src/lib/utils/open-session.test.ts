import { describe, expect, it } from 'vitest';
import type { ConnectionInput, ConnectionStatus } from '$lib/rpc';
import {
	isSavedConnectionOpen,
	removeSession,
	sessionIdOf,
	snapshotSession,
	upsertSession,
	type LiveWorkspace,
	type OpenSession,
} from '$lib/utils/open-session';
import { createDefaultTab } from '$lib/utils/workspace';

const input: ConnectionInput = {
	databaseType: 'postgres',
	name: 'local_pg',
	host: 'localhost',
	port: 5432,
	user: 'postgres',
	password: '',
	database: 'postgres',
	ssl: false,
};

const status: ConnectionStatus = {
	connected: true,
	databaseType: 'postgres',
	name: 'local_pg',
	host: 'localhost',
	port: 5432,
	database: 'postgres',
	user: 'postgres',
	serverVersion: '16',
	sessionId: 's1',
};

function live(overrides: Partial<LiveWorkspace> = {}): LiveWorkspace {
	return {
		connectionStatus: status,
		input,
		explorer: null,
		databases: ['postgres'],
		tabs: [createDefaultTab()],
		activeTabId: 'tab-1',
		explorerSearch: '',
		isExplorerLoading: false,
		globalError: '',
		queryDurationMs: 12,
		...overrides,
	};
}

describe('open sessions', () => {
	it('reads session ids from connection status', () => {
		expect(sessionIdOf(status)).toBe('s1');
		expect(sessionIdOf({ ...status, sessionId: '  ' })).toBe('');
	});

	it('snapshots live workspace into a session', () => {
		const tab = createDefaultTab();
		const snap = snapshotSession('s1', live({ tabs: [tab], activeTabId: tab.id }));
		expect(snap.id).toBe('s1');
		expect(snap.databases).toEqual(['postgres']);
		expect(snap.tabs).toHaveLength(1);
	});

	it('upserts and removes sessions like query tabs', () => {
		const first = snapshotSession('s1', live());
		const second: OpenSession = { ...first, id: 's2', status: { ...status, sessionId: 's2', name: 'other' } };
		const both = upsertSession(upsertSession([], first), second);
		expect(both.map((item) => item.id)).toEqual(['s1', 's2']);
		const replaced = upsertSession(both, { ...first, globalError: 'boom' });
		expect(replaced).toHaveLength(2);
		expect(replaced[0]?.globalError).toBe('boom');
		expect(removeSession(replaced, 's1').map((item) => item.id)).toEqual(['s2']);
	});

	it('marks a saved connection as open when a matching session exists', () => {
		const session = snapshotSession('s1', live());
		expect(isSavedConnectionOpen(input, [session])).toBe(true);
		expect(isSavedConnectionOpen({ ...input, name: 'other' }, [session])).toBe(false);
	});
});
