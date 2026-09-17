import type { ConnectionInput, DatabaseType } from '$lib/rpc';
import { dialectCapabilities } from '$lib/utils/dialect';

export function generateConnectionString(form: ConnectionInput): string {
	try {
		if (form.databaseType === 'sqlite') {
			return form.database ? `sqlite://${form.database}` : '';
		}
		const protocol =
			form.databaseType === 'mysql'
				? 'mysql'
				: form.databaseType === 'mssql'
					? 'sqlserver'
					: 'postgres';
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
	if (databaseType === 'mssql') return 'sqlserver://sa:password@localhost:1433/master';
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
		if (/^(jdbc:)?sqlserver:/i.test(val) || val.toLowerCase().startsWith('mssql:')) {
			return parseMssqlConnectionString(val, current);
		}
		if (looksLikeAdoNet(val)) {
			return parseAdoNetConnectionString(val, current);
		}
		const url = new URL(val);
		let databaseType = current.databaseType;
		if (url.protocol.includes('postgres')) databaseType = 'postgres';
		else if (url.protocol.includes('mysql')) databaseType = 'mysql';
		else if (url.protocol.includes('mssql') || url.protocol.includes('sqlserver')) {
			databaseType = 'mssql';
		}
		return {
			databaseType,
			host: url.hostname || current.host,
			port: url.port
				? Number(url.port)
				: dialectCapabilities(databaseType).defaultPort,
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

export const DATABASE_ENGINES: Array<{
	value: DatabaseType;
	label: string;
	detail: string;
}> = [
	{ value: 'postgres', label: 'PostgreSQL', detail: 'Relational' },
	{ value: 'mysql', label: 'MySQL', detail: 'Relational' },
	{ value: 'mssql', label: 'SQL Server', detail: 'Relational' },
	{ value: 'sqlite', label: 'SQLite', detail: 'Local file' },
];

export const ENGINE_KEY: Record<DatabaseType, string> = {
	sqlite: '#0f80cc',
	postgres: '#336791',
	mssql: '#cc2927',
	mysql: '#00758f',
};

function looksLikeAdoNet(value: string): boolean {
	return /(?:^|;)\s*(server|data source|initial catalog|database|user id|uid|password|pwd|encrypt|trustservercertificate)\s*=/i.test(
		value,
	);
}

function adoNetMap(value: string): Map<string, string> {
	const map = new Map<string, string>();
	for (const part of value.split(';')) {
		const trimmed = part.trim();
		if (!trimmed) continue;
		const eq = trimmed.indexOf('=');
		if (eq <= 0) continue;
		map.set(trimmed.slice(0, eq).trim().toLowerCase(), trimmed.slice(eq + 1).trim());
	}
	return map;
}

function parseAdoNetConnectionString(
	value: string,
	current: ConnectionInput,
): Partial<ConnectionInput> {
	const map = adoNetMap(value);
	const server = map.get('server') || map.get('data source') || '';
	let host = current.host;
	let port = dialectCapabilities('mssql').defaultPort;
	if (server) {
		const cleaned = server.replace(/^tcp:/i, '');
		const comma = cleaned.lastIndexOf(',');
		if (comma >= 0) {
			host = cleaned.slice(0, comma).trim() || host;
			const parsedPort = Number(cleaned.slice(comma + 1).trim());
			if (Number.isFinite(parsedPort) && parsedPort > 0) port = parsedPort;
		} else {
			host = cleaned.trim() || host;
		}
	}
	const encrypt = map.get('encrypt');
	const trust = map.get('trustservercertificate');
	return {
		databaseType: 'mssql',
		host,
		port,
		user: map.get('user id') || map.get('uid') || current.user,
		password: map.get('password') || map.get('pwd') || current.password,
		database: map.get('database') || map.get('initial catalog') || current.database,
		ssl: encrypt ? !/^(no|false|off|disable|optional)$/i.test(encrypt) : true,
		sslInsecure: trust ? /^(yes|true|1)$/i.test(trust) : current.sslInsecure,
		useConnectionString: true,
		connectionString: value,
	};
}

function parseMssqlConnectionString(
	value: string,
	current: ConnectionInput,
): Partial<ConnectionInput> | null {
	const jdbc = value.match(/^jdbc:sqlserver:\/\/([^;]+)(?:;(.*))?$/i);
	if (jdbc) {
		const hostPort = jdbc[1] ?? '';
		const rest = jdbc[2] ? `Server=${hostPort};${jdbc[2]}` : `Server=${hostPort}`;
		return parseAdoNetConnectionString(rest, current);
	}
	try {
		const normalized = value.replace(/^mssql:/i, 'sqlserver:');
		const url = new URL(normalized);
		if (!url.protocol.includes('sqlserver') && !url.protocol.includes('mssql')) return null;
		return {
			databaseType: 'mssql',
			host: url.hostname || current.host,
			port: url.port ? Number(url.port) : dialectCapabilities('mssql').defaultPort,
			user: url.username ? decodeURIComponent(url.username) : current.user,
			password: url.password ? decodeURIComponent(url.password) : current.password,
			database:
				url.pathname && url.pathname.length > 1
					? url.pathname.slice(1)
					: current.database,
		};
	} catch {
		return parseAdoNetConnectionString(value, current);
	}
}

export function withDatabaseType(
	current: ConnectionInput,
	nextType: DatabaseType,
): ConnectionInput {
	if (current.databaseType === nextType) return current;
	const defaults = defaultsForType(nextType);
	const previousDefaultName = defaultsForType(current.databaseType).name;
	const keepName = Boolean(current.name.trim()) && current.name !== previousDefaultName;
	return {
		...defaults,
		name: keepName ? current.name : defaults.name,
		password: nextType === 'sqlite' ? '' : current.password,
		ssl: nextType === 'sqlite' ? false : nextType === 'mssql' ? true : current.ssl,
		sslInsecure:
			nextType === 'sqlite'
				? false
				: nextType === 'mssql'
					? true
					: current.ssl && Boolean(current.sslInsecure),
	};
}

export function withSqliteFile(current: ConnectionInput, path: string): ConnectionInput {
	const fileName = path.replaceAll('\\', '/').split('/').pop() ?? '';
	const defaultName = defaultsForType('sqlite').name;
	const nextName =
		!current.name.trim() || current.name === defaultName
			? fileName.replace(/\.(sqlite|sqlite3|db)$/i, '') || current.name
			: current.name;
	return { ...current, databaseType: 'sqlite', database: path, name: nextName };
}

export function withParsedConnectionString(
	current: ConnectionInput,
	value: string,
): ConnectionInput | null {
	const parsed = parseConnectionString(value, current);
	if (!parsed) return null;
	return normalizeConnectionInput({ ...current, ...parsed });
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
	if (databaseType === 'mssql') return 'mssql';
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
		input.databaseType === 'mysql' ||
		input.databaseType === 'sqlite' ||
		input.databaseType === 'mssql'
			? input.databaseType
			: 'postgres';
	const defaults = dialectCapabilities(databaseType);
	const sslDefault = databaseType === 'mssql' ? true : false;
	const sslInsecureDefault = databaseType === 'mssql' ? true : false;

	return {
		databaseType,
		name: input.name ?? defaults.defaultName,
		host: input.host ?? (databaseType === 'sqlite' ? '' : 'localhost'),
		port: input.port ?? defaults.defaultPort,
		user: input.user ?? defaults.defaultUser,
		password: input.password ?? '',
		database: input.database ?? defaults.defaultDatabase,
		ssl: databaseType === 'sqlite' ? false : (input.ssl ?? sslDefault),
		sslInsecure:
			databaseType === 'sqlite' ? false : (input.sslInsecure ?? sslInsecureDefault),
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
