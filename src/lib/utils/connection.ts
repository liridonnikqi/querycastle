import type { ConnectionInput, DatabaseType } from '$lib/rpc';

export function generateConnectionString(form: ConnectionInput): string {
	try {
		if (form.databaseType === 'sqlite') {
			return form.database ? `sqlite://${form.database}` : '';
		}
		const protocol = form.databaseType === 'mysql' ? 'mysql' : 'postgres';
		const userPart = form.user ? encodeURIComponent(form.user) : '';
		const passPart = form.password ? `:${encodeURIComponent(form.password)}` : '';
		const authPart = userPart ? `${userPart}${passPart}@` : '';
		const hostPart = form.host || 'localhost';
		const portPart = form.port ? `:${form.port}` : '';
		const dbPart = form.database ? `/${form.database}` : '';
		return `${protocol}://${authPart}${hostPart}${portPart}${dbPart}`;
	} catch {
		return '';
	}
}

export function connectionStringPlaceholder(databaseType: DatabaseType): string {
	if (databaseType === 'sqlite') return 'sqlite://C:/path/to/db.sqlite';
	if (databaseType === 'mysql') return 'mysql://root:password@localhost:3306/mydb';
	return 'postgres://postgres:password@localhost:5432/postgres';
}

export function parseConnectionString(
	value: string,
	current: ConnectionInput,
): Partial<ConnectionInput> | null {
	const val = value.trim();
	if (!val) return null;
	try {
		if (val.startsWith('sqlite://')) {
			return { databaseType: 'sqlite', database: val.slice(9) };
		}
		const url = new URL(val);
		let databaseType = current.databaseType;
		if (url.protocol.includes('postgres')) databaseType = 'postgres';
		else if (url.protocol.includes('mysql')) databaseType = 'mysql';
		return {
			databaseType,
			host: url.hostname || current.host,
			port: url.port
				? Number(url.port)
				: databaseType === 'mysql'
					? 3306
					: 5432,
			user: url.username ? decodeURIComponent(url.username) : current.user,
			password: url.password ? decodeURIComponent(url.password) : current.password,
			database:
				url.pathname && url.pathname.length > 1
					? url.pathname.slice(1)
					: current.database,
		};
	} catch {
		return null;
	}
}

export function defaultsForType(databaseType: DatabaseType): ConnectionInput {
	return normalizeConnectionInput({ databaseType });
}

export function connectionSubtitle(connection: ConnectionInput): string {
	if (connection.databaseType === 'sqlite') {
		return connection.database || 'local file';
	}
	const host = connection.host || 'localhost';
	const port = connection.port ? `:${connection.port}` : '';
	const db = connection.database ? ` / ${connection.database}` : '';
	return `${host}${port}${db}`;
}

export function connectionEngineLabel(databaseType: DatabaseType): string {
	if (databaseType === 'mysql') return 'mysql';
	if (databaseType === 'sqlite') return 'sqlite';
	return 'postgres';
}

export function connectionMetaLine(connection: ConnectionInput): string {
	const engine = connectionEngineLabel(connection.databaseType);
	if (connection.databaseType === 'sqlite') {
		const file = connection.database.split(/[/\\]/).pop() || 'local file';
		return `${engine} · ${file}`;
	}
	return `${engine} · ${connection.host || 'localhost'}`;
}

export const RECENT_CONNECTIONS_KEY = 'querycastle.recentConnections.v1';

export function loadRecentConnectionNames(): string[] {
	if (typeof localStorage === 'undefined') return [];
	try {
		const raw = localStorage.getItem(RECENT_CONNECTIONS_KEY);
		if (!raw) return [];
		const parsed = JSON.parse(raw) as unknown;
		if (!Array.isArray(parsed)) return [];
		return parsed.filter((item): item is string => typeof item === 'string' && item.length > 0);
	} catch {
		return [];
	}
}

export function rememberRecentConnection(name: string): string[] {
	const trimmed = name.trim();
	if (!trimmed) return loadRecentConnectionNames();
	const next = [trimmed, ...loadRecentConnectionNames().filter((item) => item !== trimmed)].slice(0, 8);
	try {
		localStorage.setItem(RECENT_CONNECTIONS_KEY, JSON.stringify(next));
	} catch {
		// ignore quota / private-mode failures
	}
	return next;
}

export function normalizeConnectionInput(
	input: Partial<ConnectionInput>,
): ConnectionInput {
	const databaseType: DatabaseType =
		input.databaseType === 'mysql' || input.databaseType === 'sqlite'
			? input.databaseType
			: 'postgres';
	const defaultPort =
		databaseType === 'mysql' ? 3306 : databaseType === 'sqlite' ? 0 : 5432;
	const defaultName =
		databaseType === 'mysql'
			? 'local_mysql'
			: databaseType === 'sqlite'
				? 'local_sqlite'
				: 'local_pg';
	const defaultUser =
		databaseType === 'mysql'
			? 'root'
			: databaseType === 'sqlite'
				? ''
				: 'postgres';
	const defaultDatabase =
		databaseType === 'mysql'
			? 'mysql'
			: databaseType === 'sqlite'
				? 'main'
				: 'postgres';

	return {
		databaseType,
		name: input.name ?? defaultName,
		host: input.host ?? (databaseType === 'sqlite' ? '' : 'localhost'),
		port: input.port ?? defaultPort,
		user: input.user ?? defaultUser,
		password: input.password ?? '',
		database: input.database ?? defaultDatabase,
		ssl: databaseType === 'sqlite' ? false : (input.ssl ?? false),
		sslInsecure: databaseType === 'sqlite' ? false : (input.sslInsecure ?? false),
		useConnectionString: input.useConnectionString ?? false,
		connectionString: input.connectionString ?? '',
	};
}

function setConnectionStringPassword(raw: string, password: string): string {
	try {
		const url = new URL(raw);
		url.password = password;
		return url.toString();
	} catch {
		return raw;
	}
}

function stripConnectionStringPassword(raw: string): string {
	try {
		const url = new URL(raw);
		if (!url.password) return raw;
		url.password = '';
		return url.toString();
	} catch {
		return raw;
	}
}

export function passwordFromConnection(connection: ConnectionInput): string {
	if (connection.password) return connection.password;
	const raw = connection.connectionString?.trim() ?? '';
	if (!raw) return '';
	try {
		return new URL(raw).password || '';
	} catch {
		return '';
	}
}

export function stripConnectionSecrets(connection: ConnectionInput): ConnectionInput {
	return {
		...connection,
		password: '',
		connectionString: connection.connectionString
			? stripConnectionStringPassword(connection.connectionString)
			: connection.connectionString,
	};
}

export function injectConnectionPassword(
	connection: ConnectionInput,
	password: string,
): ConnectionInput {
	const next: ConnectionInput = { ...connection, password };
	if (next.useConnectionString && next.connectionString?.trim()) {
		next.connectionString = setConnectionStringPassword(next.connectionString, password);
	}
	return next;
}

export async function migrateSavedConnectionSecrets(params: {
	connections: ConnectionInput[];
	secretSet: (name: string, password: string) => Promise<void>;
}): Promise<{ connections: ConnectionInput[]; changed: boolean }> {
	let changed = false;
	const next: ConnectionInput[] = [];
	for (const connection of params.connections) {
		const password = passwordFromConnection(connection);
		const stripped = stripConnectionSecrets(connection);
		const needsStrip =
			stripped.password !== connection.password ||
			stripped.connectionString !== connection.connectionString;
		if (password && connection.name.trim()) {
			try {
				await params.secretSet(connection.name, password);
				next.push(stripped);
				changed = true;
			} catch {
				next.push(connection);
			}
		} else if (needsStrip) {
			next.push(stripped);
			changed = true;
		} else {
			next.push(stripped);
		}
	}
	return { connections: next, changed };
}
