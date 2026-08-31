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
	if (trimmed.startsWith('"') && trimmed.endsWith('"') && trimmed.length >= 2) {
		return trimmed.slice(1, -1).replaceAll('""', '"');
	}
	if (trimmed.startsWith('`') && trimmed.endsWith('`') && trimmed.length >= 2) {
		return trimmed.slice(1, -1).replaceAll('``', '`');
	}
	return trimmed;
}

export function identifierNeedsQuotes(
	databaseType: DatabaseType,
	name: string,
): boolean {
	if (databaseType === 'mysql') {
		return !/^[A-Za-z_][A-Za-z0-9_$]*$/.test(name) || /[A-Z]/.test(name);
	}
	return !/^[a-z_][a-z0-9_$]*$/.test(name);
}

export function quoteSqlIdentifierIfNeeded(
	databaseType: DatabaseType,
	name: string,
): string {
	return identifierNeedsQuotes(databaseType, name)
		? quoteSqlIdentifier(databaseType, name)
		: name;
}

function uniqueFoldedName(names: Iterable<string>): Map<string, string> {
	const buckets = new Map<string, Set<string>>();
	for (const name of names) {
		const key = name.toLowerCase();
		const bucket = buckets.get(key) ?? new Set<string>();
		bucket.add(name);
		buckets.set(key, bucket);
	}
	const unique = new Map<string, string>();
	for (const [key, bucket] of buckets) {
		if (bucket.size !== 1) continue;
		const [actual] = bucket;
		if (actual) unique.set(key, actual);
	}
	return unique;
}

function isAllCapsKeyword(token: string): boolean {
	return token === token.toUpperCase() && /^[A-Z][A-Z0-9_]*$/.test(token);
}

function skipSqlString(sql: string, start: number, quote: "'" | '"' | '`'): number {
	let i = start + 1;
	while (i < sql.length) {
		if (sql[i] === quote) {
			if (sql[i + 1] === quote) {
				i += 2;
				continue;
			}
			return i + 1;
		}
		i += 1;
	}
	return sql.length;
}

function skipLineComment(sql: string, start: number): number {
	const newline = sql.indexOf('\n', start);
	return newline === -1 ? sql.length : newline + 1;
}

function skipBlockComment(sql: string, start: number): number {
	const end = sql.indexOf('*/', start + 2);
	return end === -1 ? sql.length : end + 2;
}

function skipDollarQuote(sql: string, start: number): number {
	const open = sql.slice(start).match(/^\$[A-Za-z0-9_]*\$/);
	if (!open) return start;
	const delim = open[0];
	const end = sql.indexOf(delim, start + delim.length);
	return end === -1 ? sql.length : end + delim.length;
}

export function quoteCatalogIdentifiersInSql(
	sql: string,
	databaseType: DatabaseType,
	catalogNames: Iterable<string>,
): string {
	const catalog = uniqueFoldedName(catalogNames);
	if (catalog.size === 0) return sql;

	let output = '';
	let i = 0;
	while (i < sql.length) {
		const char = sql[i]!;
		const next = sql[i + 1];

		if (char === '-' && next === '-') {
			const end = skipLineComment(sql, i);
			output += sql.slice(i, end);
			i = end;
			continue;
		}
		if (char === '/' && next === '*') {
			const end = skipBlockComment(sql, i);
			output += sql.slice(i, end);
			i = end;
			continue;
		}
		if (char === "'") {
			const end = skipSqlString(sql, i, "'");
			output += sql.slice(i, end);
			i = end;
			continue;
		}
		if (char === '"') {
			const end = skipSqlString(sql, i, '"');
			output += sql.slice(i, end);
			i = end;
			continue;
		}
		if (char === '`' && databaseType === 'mysql') {
			const end = skipSqlString(sql, i, '`');
			output += sql.slice(i, end);
			i = end;
			continue;
		}
		if (char === '$' && databaseType === 'postgres') {
			const end = skipDollarQuote(sql, i);
			if (end !== i) {
				output += sql.slice(i, end);
				i = end;
				continue;
			}
		}

		const ident = sql.slice(i).match(/^[A-Za-z_][A-Za-z0-9_$]*/);
		if (!ident) {
			output += char;
			i += 1;
			continue;
		}

		const token = ident[0];
		const after = i + token.length;
		const actual = catalog.get(token.toLowerCase());
		const followedByParen = sql[after] === '(';
		const shouldQuote =
			actual &&
			identifierNeedsQuotes(databaseType, actual) &&
			token !== token.toLowerCase() &&
			!isAllCapsKeyword(token) &&
			!followedByParen;

		output += shouldQuote ? quoteSqlIdentifier(databaseType, actual) : token;
		i = after;
	}

	return output;
}
