import type { ConnectionStatus } from '$lib/rpc';
import { rpc } from '$lib/rpc-client';

export async function initializeWorkspace(params: {
	connectExternalSqliteFile: (path: string) => Promise<void>;
	loadExternalSqlFile: (path: string, content: string) => void;
	setConnectionStatus: (status: ConnectionStatus) => void;
	loadDatabases: () => Promise<void>;
	loadExplorer: () => Promise<void>;
	restoreDataTabResults: () => Promise<void>;
}): Promise<void> {
	const launchSqliteFile = await rpc.getLaunchSqliteFile();
	if (launchSqliteFile) {
		await params.connectExternalSqliteFile(launchSqliteFile.path);
	}

	const launchSqlFile = await rpc.getLaunchSqlFile();
	if (launchSqlFile) {
		params.loadExternalSqlFile(launchSqlFile.path, launchSqlFile.content);
	}

	const status = await rpc.connectionStatus();
	params.setConnectionStatus(status);
	await params.loadDatabases();
	await params.loadExplorer();
	await params.restoreDataTabResults();
}
