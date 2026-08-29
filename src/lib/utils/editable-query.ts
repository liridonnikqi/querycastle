import type { DatabaseExplorer, DatabaseType } from '$lib/rpc';
import { isExplorerView } from '$lib/utils/schema-objects';
import { quoteSqlIdentifier, unquoteIdent } from '$lib/utils/sql';

export type EditableQueryPlan = {
	sql: string;
	context: { schema: string; table: string };
};

function resolveTableSchema(
	explorer: DatabaseExplorer | null,
	tableName: string,
): string | null {
	if (!explorer) return null;
	const exactMatches: string[] = [];
	const lowerMatches: string[] = [];

	for (const schema of explorer.schemas) {
		for (const table of schema.tables) {
			if (table.name === tableName) exactMatches.push(schema.name);
			if (table.name.toLowerCase() === tableName.toLowerCase()) {
				lowerMatches.push(schema.name);
			}
		}
	}

	if (exactMatches.length === 1) return exactMatches[0];
	if (exactMatches.length > 1) return exactMatches.includes('public') ? 'public' : null;
	if (lowerMatches.length === 1) return lowerMatches[0];
	if (lowerMatches.length > 1) return lowerMatches.includes('public') ? 'public' : null;
	return null;
}

function resolvePreferredOrderColumn(
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
): string | null {
	const tableMeta = explorer?.schemas
		.find((item) => item.name === schema)
		?.tables.find((item) => item.name === table);
	return tableMeta?.columns[0]?.name ?? null;
}

export function buildMysqlRowHashExpression(
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
	columnPrefix?: string,
): string | null {
	const tableMeta = explorer?.schemas
		.find((item) => item.name === schema)
		?.tables.find((item) => item.name === table);
	if (!tableMeta || tableMeta.columns.length === 0) return null;

	const parts = tableMeta.columns.map((column) => {
		const safeColumn = quoteSqlIdentifier('mysql', column.name);
		const qualifiedColumn = columnPrefix
			? `${columnPrefix}.${safeColumn}`
			: safeColumn;
		return `coalesce(cast(${qualifiedColumn} as char), '__querycastle_null__')`;
	});
	return `md5(concat_ws(char(31), ${parts.join(', ')}))`;
}

export function tryBuildEditableQuery(params: {
	sql: string;
	databaseType: DatabaseType;
	explorer: DatabaseExplorer | null;
}): EditableQueryPlan | null {
	const { sql, databaseType, explorer } = params;
	if (
		databaseType !== 'postgres' &&
		databaseType !== 'mysql' &&
		databaseType !== 'sqlite'
	) {
		return null;
	}

	const cleaned = sql.trim().replace(/;+\s*$/, '');
	if (!/^select\b/i.test(cleaned)) return null;

	const selectMatch = cleaned.match(/^\s*select\s+([\s\S]+?)\s+from\s+([\s\S]+)$/i);
	if (!selectMatch) return null;

	const selectPart = selectMatch[1];
	const fromAndTail = selectMatch[2];
	if (/\bdistinct\b/i.test(selectPart)) return null;
	if (/\b(with|join|group\s+by|having|union|intersect|except)\b/i.test(fromAndTail)) {
		return null;
	}

	const tableMatch = fromAndTail.match(
		/^\s*((?:"(?:[^"]|"")+"|[A-Za-z_][A-Za-z0-9_$]*)(?:\s*\.\s*(?:"(?:[^"]|"")+"|[A-Za-z_][A-Za-z0-9_$]*))?)([\s\S]*)$/s,
	);
	if (!tableMatch) return null;

	const tableRef = tableMatch[1];
	const tail = tableMatch[2] ?? '';
	if (/\bfrom\b/i.test(tail)) return null;
	if (/^\s*,/.test(tail)) return null;

	const qualifiedIdMatch = tableRef.match(
		/^\s*(?:"((?:[^"]|"")*)"|([A-Za-z_][A-Za-z0-9_$]*))\s*\.\s*(?:"((?:[^"]|"")*)"|([A-Za-z_][A-Za-z0-9_$]*))\s*$/s,
	);
	const unqualifiedIdMatch = tableRef.match(
		/^\s*(?:"((?:[^"]|"")*)"|([A-Za-z_][A-Za-z0-9_$]*))\s*$/s,
	);

	let contextSchema = '';
	let contextTable = '';
	if (qualifiedIdMatch) {
		const rawSchema = qualifiedIdMatch[1]
			? `"${qualifiedIdMatch[1]}"`
			: (qualifiedIdMatch[2] ?? '');
		const rawTable = qualifiedIdMatch[3]
			? `"${qualifiedIdMatch[3]}"`
			: (qualifiedIdMatch[4] ?? '');
		if (!rawSchema || !rawTable) return null;
		contextSchema = unquoteIdent(rawSchema);
		contextTable = unquoteIdent(rawTable);
	} else if (unqualifiedIdMatch) {
		const rawTable = unqualifiedIdMatch[1]
			? `"${unqualifiedIdMatch[1]}"`
			: (unqualifiedIdMatch[2] ?? '');
		const tableName = unquoteIdent(rawTable);
		const resolvedSchema = resolveTableSchema(explorer, tableName);
		if (!resolvedSchema) return null;
		contextSchema = resolvedSchema;
		contextTable = tableName;
	} else {
		return null;
	}

	if (isExplorerView(explorer, contextSchema, contextTable)) return null;

	let effectiveTail = tail;
	if (!/\border\s+by\b/i.test(effectiveTail)) {
		const preferredOrderColumn = resolvePreferredOrderColumn(
			explorer,
			contextSchema,
			contextTable,
		);
		const orderByClause = preferredOrderColumn
			? databaseType === 'mysql'
				? ` order by ${quoteSqlIdentifier(databaseType, preferredOrderColumn)} asc`
				: ` order by ${quoteSqlIdentifier(databaseType, preferredOrderColumn)} asc nulls last`
			: databaseType === 'sqlite'
				? ' order by rowid asc'
				: ' order by ctid asc';
		const limitLikeMatch = effectiveTail.match(/\b(limit|offset|fetch)\b/i);
		if (limitLikeMatch && limitLikeMatch.index !== undefined) {
			const insertAt = limitLikeMatch.index;
			effectiveTail = `${effectiveTail.slice(0, insertAt)}${orderByClause} ${effectiveTail.slice(insertAt)}`;
		} else {
			effectiveTail = `${effectiveTail}${orderByClause}`;
		}
	}

	if (databaseType === 'sqlite') {
		return {
			sql: `select cast(rowid as text) as _querycastle_ctid, ${selectPart} from ${tableRef}${effectiveTail};`,
			context: { schema: contextSchema, table: contextTable },
		};
	}

	if (databaseType === 'mysql') {
		const rowHashExpression = buildMysqlRowHashExpression(
			explorer,
			contextSchema,
			contextTable,
		);
		if (!rowHashExpression) return null;
		const mysqlSelectPart = selectPart.trim() === '*' ? `${tableRef}.*` : selectPart;
		return {
			sql: `select ${rowHashExpression} as _querycastle_ctid, ${mysqlSelectPart} from ${tableRef}${effectiveTail};`,
			context: { schema: contextSchema, table: contextTable },
		};
	}

	return {
		sql: `select ctid::text as _querycastle_ctid, ${selectPart} from ${tableRef}${effectiveTail};`,
		context: { schema: contextSchema, table: contextTable },
	};
}
