export type QueryHistoryItem = {
	time: string;
	sql: string;
	durationMs: number;
	success: boolean;
	error?: string;
	connectionKey?: string;
};

export type SavedQueryItem = {
	id: string;
	title: string;
	sql: string;
	createdAt: number;
	connectionKey: string;
};
