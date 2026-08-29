import type {
	DatabaseExplorer,
	DatabaseIndex,
	DatabaseRoutine,
	DatabaseSchema,
	DatabaseSequence,
	DatabaseTable,
	DatabaseTrigger,
} from '$lib/rpc';

export function schemaRoutines(schema: DatabaseSchema): DatabaseRoutine[] {
	return schema.routines ?? [];
}

export function schemaSequences(schema: DatabaseSchema): DatabaseSequence[] {
	return schema.sequences ?? [];
}

export function tableIndexes(table: DatabaseTable): DatabaseIndex[] {
	return table.indexes ?? [];
}

export function tableTriggers(table: DatabaseTable): DatabaseTrigger[] {
	return table.triggers ?? [];
}

export function schemaFunctions(schema: DatabaseSchema): DatabaseRoutine[] {
	return schemaRoutines(schema).filter((item) => item.kind === 'function');
}

export function schemaProcedures(schema: DatabaseSchema): DatabaseRoutine[] {
	return schemaRoutines(schema).filter((item) => item.kind === 'procedure');
}

export function schemaTables(schema: DatabaseSchema): DatabaseTable[] {
	return schema.tables.filter((item) => item.kind !== 'view');
}

export function schemaViews(schema: DatabaseSchema): DatabaseTable[] {
	return schema.tables.filter((item) => item.kind === 'view');
}

export function findExplorerTable(
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
): DatabaseTable | null {
	if (!explorer) return null;
	return (
		explorer.schemas
			.find((item) => item.name === schema)
			?.tables.find((item) => item.name === table) ?? null
	);
}

export function isExplorerView(
	explorer: DatabaseExplorer | null,
	schema: string,
	table: string,
): boolean {
	return findExplorerTable(explorer, schema, table)?.kind === 'view';
}

export function routineSignature(routine: DatabaseRoutine): string {
	return `${routine.name}(${routine.identityArgs?.trim() ?? ''})`;
}

export function definitionTabTitle(
	kind: string,
	name: string,
	identityArgs?: string | null,
): string {
	const args = identityArgs?.trim();
	if ((kind === 'function' || kind === 'procedure') && args) {
		return `${name}(${args})`;
	}
	return name;
}

export function explorerObjectCount(explorer: DatabaseExplorer | null): number {
	if (!explorer) return 0;
	return explorer.schemas.reduce((sum, schema) => {
		return (
			sum +
			schema.tables.length +
			schemaRoutines(schema).length +
			schemaSequences(schema).length
		);
	}, 0);
}

export function filterExplorer(
	explorer: DatabaseExplorer,
	query: string,
): DatabaseExplorer {
	const normalized = query.trim().toLowerCase();
	if (!normalized) return explorer;

	const schemas = explorer.schemas
		.map((schema): DatabaseSchema | null => {
			const tables = schema.tables
				.map((table) => {
					const nameMatch = table.name.toLowerCase().includes(normalized);
					const columnMatch = table.columns.some((column) =>
						column.name.toLowerCase().includes(normalized),
					);
					const indexMatch = tableIndexes(table).some((index) =>
						index.name.toLowerCase().includes(normalized),
					);
					const triggerMatch = tableTriggers(table).some((trigger) =>
						trigger.name.toLowerCase().includes(normalized),
					);
					if (nameMatch || columnMatch || indexMatch || triggerMatch) return table;
					return null;
				})
				.filter((table): table is DatabaseTable => table !== null);

			const routines = schemaRoutines(schema).filter((routine) => {
				return (
					routine.name.toLowerCase().includes(normalized) ||
					routineSignature(routine).toLowerCase().includes(normalized)
				);
			});
			const sequences = schemaSequences(schema).filter((sequence) =>
				sequence.name.toLowerCase().includes(normalized),
			);

			if (tables.length === 0 && routines.length === 0 && sequences.length === 0) {
				return null;
			}
			return { ...schema, tables, routines, sequences } satisfies DatabaseSchema;
		})
		.filter((schema): schema is DatabaseSchema => schema !== null);

	return { ...explorer, schemas };
}
