import type { ConnectionInput } from '$lib/rpc';
import type { QueryHistoryItem, SavedQueryItem } from '$lib/types';
import type { TabKind, WorkspaceTab } from '$lib/utils/workspace';
import { createEmptyResult } from '$lib/utils/workspace';

export function loadSavedConnectionsFromStorage(params: {
	key: string;
	normalize: (input: Partial<ConnectionInput>) => ConnectionInput;
}): ConnectionInput[] {
	try {
		const raw = localStorage.getItem(params.key);
		if (!raw) return [];
		const parsed = JSON.parse(raw) as Array<Partial<ConnectionInput>>;
		if (!Array.isArray(parsed)) return [];
		return parsed.map((entry) => params.normalize(entry));
	} catch {
		return [];
	}
}

export function loadQueryTabsFromStorage(key: string): WorkspaceTab[] {
	try {
		const raw = localStorage.getItem(key);
		if (!raw) return [];
		const parsed = JSON.parse(raw) as Array<Partial<WorkspaceTab>>;
		if (!Array.isArray(parsed)) return [];
		return parsed
			.filter(
				(item) => typeof item.id === 'string' && typeof item.title === 'string',
			)
			.map((item) => ({
				id: item.id!,
				title: item.title!,
				kind: (item.kind === 'data' ? 'data' : 'query') as TabKind,
				sql: typeof item.sql === 'string' ? item.sql : '',
				lastRunSql: '',
				result: createEmptyResult(),
				sqlError: '',
				resultContext: item.resultContext ?? null,
			}));
	} catch {
		return [];
	}
}

export function loadQueryFavoritesFromStorage(key: string): SavedQueryItem[] {
	try {
		const raw = localStorage.getItem(key);
		if (!raw) return [];
		const parsed = JSON.parse(raw) as Array<Partial<SavedQueryItem>>;
		if (!Array.isArray(parsed)) return [];
		return parsed
			.filter(
				(item) =>
					typeof item.id === 'string' &&
					typeof item.sql === 'string' &&
					typeof item.connectionKey === 'string',
			)
			.map((item) => ({
				id: item.id!,
				title:
					typeof item.title === 'string' && item.title.trim().length > 0
						? item.title
						: 'Saved Query',
				sql: item.sql!,
				createdAt: typeof item.createdAt === 'number' ? item.createdAt : Date.now(),
				connectionKey: item.connectionKey!,
			}));
	} catch {
		return [];
	}
}

export function loadQueryHistoryFromStorage(key: string): QueryHistoryItem[] {
	try {
		const raw = localStorage.getItem(key);
		if (!raw) return [];
		const parsed = JSON.parse(raw) as Array<Partial<QueryHistoryItem>>;
		if (!Array.isArray(parsed)) return [];
		return parsed
			.filter(
				(item) =>
					typeof item.time === 'string' &&
					typeof item.sql === 'string' &&
					typeof item.durationMs === 'number' &&
					typeof item.success === 'boolean',
			)
			.map((item) => ({
				time: item.time!,
				sql: item.sql!,
				durationMs: item.durationMs!,
				success: item.success!,
				error: typeof item.error === 'string' ? item.error : undefined,
				connectionKey:
					typeof item.connectionKey === 'string' ? item.connectionKey : undefined,
			}));
	} catch {
		return [];
	}
}

export function persistJsonValue(key: string, value: unknown): void {
	localStorage.setItem(key, JSON.stringify(value));
}

export function toPersistedTabs(tabs: WorkspaceTab[]) {
	return tabs.map((tab) => ({
		id: tab.id,
		title: tab.title,
		kind: tab.kind,
		sql: tab.sql,
		resultContext: tab.resultContext,
	}));
}
