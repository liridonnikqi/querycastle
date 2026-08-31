import type { ConnectionInput, ConnectionStatus, DatabaseExplorer } from '$lib/rpc';
import type { WorkspaceTab } from '$lib/utils/workspace';

export type OpenSession = {
	id: string;
	status: ConnectionStatus;
	input: ConnectionInput;
	explorer: DatabaseExplorer | null;
	databases: string[];
	tabs: WorkspaceTab[];
	activeTabId: string;
	explorerSearch: string;
	isExplorerLoading: boolean;
	globalError: string;
	queryDurationMs: number;
};

export type LiveWorkspace = {
	connectionStatus: ConnectionStatus;
	input: ConnectionInput;
	explorer: DatabaseExplorer | null;
	databases: string[];
	tabs: WorkspaceTab[];
	activeTabId: string;
	explorerSearch: string;
	isExplorerLoading: boolean;
	globalError: string;
	queryDurationMs: number;
};

export function sessionIdOf(status: ConnectionStatus): string {
	return status.sessionId?.trim() ?? '';
}

export function snapshotSession(id: string, live: LiveWorkspace): OpenSession {
	return {
		id,
		status: live.connectionStatus,
		input: live.input,
		explorer: live.explorer,
		databases: live.databases,
		tabs: live.tabs,
		activeTabId: live.activeTabId,
		explorerSearch: live.explorerSearch,
		isExplorerLoading: live.isExplorerLoading,
		globalError: live.globalError,
		queryDurationMs: live.queryDurationMs,
	};
}

export function upsertSession(sessions: OpenSession[], next: OpenSession): OpenSession[] {
	const index = sessions.findIndex((item) => item.id === next.id);
	if (index === -1) return [...sessions, next];
	const copy = [...sessions];
	copy[index] = next;
	return copy;
}

export function removeSession(sessions: OpenSession[], id: string): OpenSession[] {
	return sessions.filter((item) => item.id !== id);
}

export function savedConnectionKey(connection: {
	name: string;
	databaseType: string;
	host: string;
	port: number;
	database: string;
	user: string;
}): string {
	return `${connection.databaseType}|${connection.name}|${connection.host}|${connection.port}|${connection.database}|${connection.user}`;
}

export function isSavedConnectionOpen(
	connection: {
		name: string;
		databaseType: string;
		host: string;
		port: number;
		database: string;
		user: string;
	},
	sessions: Array<{
		input: { name: string; databaseType: string; host: string; port: number; database: string; user: string };
		status?: { name?: string | null; databaseType: string; host: string; port: number; database: string; user: string };
	}>,
): boolean {
	const key = savedConnectionKey(connection);
	return sessions.some((session) => {
		if (savedConnectionKey(session.input) === key) return true;
		const status = session.status;
		if (!status) return false;
		return (
			(status.name?.trim() || '') === connection.name &&
			status.databaseType === connection.databaseType &&
			status.host === connection.host &&
			status.port === connection.port &&
			status.database === connection.database &&
			status.user === connection.user
		);
	});
}
