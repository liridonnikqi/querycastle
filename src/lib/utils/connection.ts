import type { ConnectionInput, DatabaseType } from '$lib/rpc';

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
		useConnectionString: input.useConnectionString ?? false,
		connectionString: input.connectionString ?? '',
	};
}
