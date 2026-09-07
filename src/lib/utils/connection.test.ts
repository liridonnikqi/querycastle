import { describe, expect, it, beforeEach } from 'vitest';
import {
	connectionMetaLine,
	connectionStringPlaceholder,
	defaultsForType,
	generateConnectionString,
	injectConnectionPassword,
	loadRecentConnectionNames,
	migrateSavedConnectionSecrets,
	normalizeConnectionInput,
	passwordFromConnection,
	rememberRecentConnection,
	stripConnectionSecrets,
	withDatabaseType,
	withParsedConnectionString,
	withSqliteFile,
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

describe('connection form helpers', () => {
	it('switches engine defaults but keeps a custom name and password', () => {
		const current = {
			...defaultsForType('postgres'),
			name: 'prod',
			password: 's3cret',
			ssl: true,
			sslInsecure: true,
		};
		const mysql = withDatabaseType(current, 'mysql');
		expect(mysql.databaseType).toBe('mysql');
		expect(mysql.name).toBe('prod');
		expect(mysql.password).toBe('s3cret');
		expect(mysql.port).toBe(3306);
		expect(mysql.ssl).toBe(true);
		expect(mysql.sslInsecure).toBe(true);

		const sqlite = withDatabaseType(current, 'sqlite');
		expect(sqlite.databaseType).toBe('sqlite');
		expect(sqlite.password).toBe('');
		expect(sqlite.ssl).toBe(false);
		expect(sqlite.sslInsecure).toBe(false);
	});

	it('names a sqlite connection from the file when the name is still the default', () => {
		const next = withSqliteFile(defaultsForType('sqlite'), 'C:/data/analytics.db');
		expect(next.database).toBe('C:/data/analytics.db');
		expect(next.name).toBe('analytics');
	});

	it('defaults SQL Server to local encrypt + trust cert', () => {
		const mssql = defaultsForType('mssql');
		expect(mssql).toMatchObject({
			databaseType: 'mssql',
			name: 'local_mssql',
			port: 1433,
			user: 'sa',
			database: 'master',
			ssl: true,
			sslInsecure: true,
		});
		expect(generateConnectionString(mssql)).toContain('sqlserver://');
		expect(connectionStringPlaceholder('mssql')).toContain('sqlserver://');
	});

	it('parses sqlserver URLs and ADO.NET strings', () => {
		const fromUrl = withParsedConnectionString(
			defaultsForType('mssql'),
			'sqlserver://sa:pw@db.example:14333/app',
		);
		expect(fromUrl).toMatchObject({
			databaseType: 'mssql',
			host: 'db.example',
			port: 14333,
			user: 'sa',
			password: 'pw',
			database: 'app',
		});
		const fromAdo = withParsedConnectionString(
			defaultsForType('postgres'),
			'Server=tcp:localhost,1433;Database=shop;User Id=sa;Password=secret;Encrypt=yes;TrustServerCertificate=yes',
		);
		expect(fromAdo).toMatchObject({
			databaseType: 'mssql',
			host: 'localhost',
			port: 1433,
			database: 'shop',
			user: 'sa',
			password: 'secret',
			ssl: true,
			sslInsecure: true,
		});
	});

	it('fills fields from a pasted connection string', () => {
		const parsed = withParsedConnectionString(
			defaultsForType('postgres'),
			'mysql://root:pw@db.example:3307/app',
		);
		expect(parsed?.databaseType).toBe('mysql');
		expect(parsed?.host).toBe('db.example');
		expect(parsed?.port).toBe(3307);
		expect(parsed?.user).toBe('root');
		expect(parsed?.password).toBe('pw');
		expect(parsed?.database).toBe('app');
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

describe('connection secrets', () => {
	it('defaults sslInsecure to false', () => {
		expect(normalizeConnectionInput({ databaseType: 'postgres' }).sslInsecure).toBe(false);
		expect(normalizeConnectionInput({ databaseType: 'sqlite', sslInsecure: true }).sslInsecure).toBe(
			false,
		);
	});

	it('strips password fields and URL passwords', () => {
		const stripped = stripConnectionSecrets(
			normalizeConnectionInput({
				name: 'prod',
				password: 's3cret',
				useConnectionString: true,
				connectionString: 'postgres://postgres:s3cret@localhost:5432/app',
			}),
		);
		expect(stripped.password).toBe('');
		expect(stripped.connectionString).not.toContain('s3cret');
		expect(passwordFromConnection(stripped)).toBe('');
	});

	it('reinjects a keyring password into the form and URL', () => {
		const injected = injectConnectionPassword(
			normalizeConnectionInput({
				name: 'prod',
				useConnectionString: true,
				connectionString: 'postgres://postgres@localhost:5432/app',
			}),
			's3cret',
		);
		expect(injected.password).toBe('s3cret');
		expect(injected.connectionString).toContain('s3cret');
	});

	it('migrates plaintext passwords into the keychain and rewrites storage', async () => {
		const stored: Array<[string, string]> = [];
		const result = await migrateSavedConnectionSecrets({
			connections: [
				normalizeConnectionInput({
					name: 'prod',
					password: 's3cret',
					connectionString: 'postgres://postgres:s3cret@localhost:5432/app',
				}),
			],
			secretSet: async (name, password) => {
				stored.push([name, password]);
			},
		});
		expect(stored).toEqual([['prod', 's3cret']]);
		expect(result.changed).toBe(true);
		expect(result.connections[0]?.password).toBe('');
		expect(result.connections[0]?.connectionString).not.toContain('s3cret');
	});

	it('keeps plaintext if the keychain write fails', async () => {
		const original = normalizeConnectionInput({ name: 'prod', password: 's3cret' });
		const result = await migrateSavedConnectionSecrets({
			connections: [original],
			secretSet: async () => {
				throw new Error('no keychain');
			},
		});
		expect(result.changed).toBe(false);
		expect(result.connections[0]?.password).toBe('s3cret');
	});
});
