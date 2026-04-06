import type { DatabaseType } from '$lib/rpc';

export function quoteIdent(value: string): string {
	return `"${value.replaceAll('"', '""')}"`;
}

export function quoteSqlIdentifier(
	databaseType: DatabaseType,
	value: string,
): string {
	if (databaseType === 'mysql') {
		return `\`${value.replaceAll('`', '``')}\``;
	}
	return quoteIdent(value);
}

export function unquoteIdent(value: string): string {
	const trimmed = value.trim();
	if (trimmed.startsWith('"') && trimmed.endsWith('"')) {
		return trimmed.slice(1, -1).replaceAll('""', '"');
	}
	return trimmed.toLowerCase();
}
