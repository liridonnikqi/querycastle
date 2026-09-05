import { describe, expect, it, beforeEach } from 'vitest';
import {
	connectionMetaLine,
	defaultsForType,
	injectConnectionPassword,
	loadRecentConnectionNames,
	migrateSavedConnectionSecrets,
	normalizeConnectionInput,
	passwordFromConnection,
	rememberRecentConnection,
	stripConnectionSecrets,
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
