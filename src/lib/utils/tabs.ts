import type { RelationHop, WorkspaceTab } from '$lib/utils/workspace';
import { createEmptyResult } from '$lib/utils/workspace';

export function createQueryTab(
	queryTabCount: number,
	initialSql = '',
	title?: string,
): WorkspaceTab {
	return {
		id: crypto.randomUUID(),
		title:
			title && title.trim().length > 0
				? title
				: `Query ${queryTabCount + 1}`,
		kind: 'query',
		sql: initialSql,
		lastRunSql: '',
		result: createEmptyResult(),
		sqlError: '',
		resultContext: null,
		relationTrail: [],
	};
}

export function createDataTab(params: {
	title: string;
	sql: string;
	context: { schema: string; table: string } | null;
	relationTrail?: RelationHop[];
}): WorkspaceTab {
	return {
		id: crypto.randomUUID(),
		title: params.title,
		kind: 'data',
		sql: params.sql,
		lastRunSql: '',
		result: createEmptyResult(),
		sqlError: '',
		resultContext: params.context,
		relationTrail: params.relationTrail ?? [],
	};
}

export function createDiagramTab(): WorkspaceTab {
	return {
		id: crypto.randomUUID(),
		title: 'Schema diagram',
		kind: 'diagram',
		sql: '',
		lastRunSql: '',
		result: createEmptyResult(),
		sqlError: '',
		resultContext: null,
		relationTrail: [],
	};
}

export function insertTabAfter(params: {
	tabs: WorkspaceTab[];
	activeTabId: string;
	tab: WorkspaceTab;
}): { tabs: WorkspaceTab[]; activeTabId: string } {
	const index = params.tabs.findIndex((tab) => tab.id === params.activeTabId);
	const insertAt = index === -1 ? params.tabs.length : index + 1;
	const nextTabs = [...params.tabs];
	nextTabs.splice(insertAt, 0, params.tab);
	return { tabs: nextTabs, activeTabId: params.tab.id };
}

export function setSqlInReusableQueryTabState(params: {
	tabs: WorkspaceTab[];
	activeTabId: string;
	sql: string;
}): { tabs: WorkspaceTab[]; activeTabId: string } {
	const activeQueryTab =
		params.tabs.find(
			(tab) => tab.id === params.activeTabId && tab.kind === 'query',
		) ?? null;
	const targetTab = activeQueryTab ?? params.tabs.find((tab) => tab.kind === 'query') ?? null;

	if (!targetTab) {
		const nextTab = createQueryTab(
			params.tabs.filter((tab) => tab.kind === 'query').length,
			params.sql,
		);
		return { tabs: [...params.tabs, nextTab], activeTabId: nextTab.id };
	}

	return {
		tabs: params.tabs.map((tab) =>
			tab.id === targetTab.id ? { ...tab, sql: params.sql, sqlError: '' } : tab,
		),
		activeTabId: targetTab.id,
	};
}

export function closeTabState(params: {
	tabs: WorkspaceTab[];
	activeTabId: string;
	tabId: string;
}): { tabs: WorkspaceTab[]; activeTabId: string } {
	const index = params.tabs.findIndex((tab) => tab.id === params.tabId);
	if (index === -1) return { tabs: params.tabs, activeTabId: params.activeTabId };
	const nextTabs = params.tabs.filter((tab) => tab.id !== params.tabId);
	if (params.activeTabId !== params.tabId) {
		return { tabs: nextTabs, activeTabId: params.activeTabId };
	}
	if (nextTabs.length === 0) return { tabs: nextTabs, activeTabId: '' };
	const nextIndex = Math.max(0, index - 1);
	return { tabs: nextTabs, activeTabId: nextTabs[nextIndex]?.id ?? nextTabs[0]?.id ?? '' };
}
