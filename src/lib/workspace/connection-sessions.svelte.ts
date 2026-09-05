import type { ConnectionInput, ConnectionStatus } from '$lib/rpc';
import { normalizeConnectionInput } from '$lib/utils/connection';
import {
	removeSession,
	sessionIdOf,
	snapshotSession,
	upsertSession,
	type LiveWorkspace,
	type OpenSession,
} from '$lib/utils/open-session';

export function disconnectedStatus(): ConnectionStatus {
	return {
		connected: false,
		databaseType: 'postgres',
		name: 'Disconnected',
		host: '',
		port: 5432,
		database: '',
		user: '',
		serverVersion: null,
		sessionId: '',
	};
}

/** Open connection tabs. Tabs/explorer still live on Workspace and are snapshotted in. */
export class ConnectionSessions {
	connectionStatus = $state<ConnectionStatus>(disconnectedStatus());
	openSessions = $state<OpenSession[]>([]);
	activeSessionId = $state('');
	activeSessionInput = $state<ConnectionInput>(normalizeConnectionInput({}));

	stash(live: LiveWorkspace) {
		const id = sessionIdOf(this.connectionStatus) || this.activeSessionId;
		if (!id || !this.connectionStatus.connected) return;
		this.openSessions = upsertSession(this.openSessions, snapshotSession(id, live));
	}

	restoreIdentity(session: OpenSession) {
		this.connectionStatus = session.status;
		this.activeSessionInput = session.input;
		this.activeSessionId = session.id;
	}

	clear() {
		this.openSessions = [];
		this.activeSessionId = '';
		this.connectionStatus = disconnectedStatus();
	}

	drop(id: string) {
		this.openSessions = removeSession(this.openSessions, id);
	}

	find(id: string): OpenSession | undefined {
		return this.openSessions.find((item) => item.id === id);
	}

	remember(id: string, live: LiveWorkspace) {
		this.openSessions = [snapshotSession(id, live)];
		this.activeSessionId = id;
	}
}
