import { writable } from 'svelte/store';

export type SnackbarType = 'info' | 'success' | 'warning' | 'error' | 'update';

export interface Snackbar {
	id: string;
	message: string;
	description?: string;
	type: SnackbarType;
	duration?: number; // 0 = persistent
	actionLabel?: string;
	onAction?: () => void;
	showClose?: boolean;
}

export const snackbars = writable<Snackbar[]>([]);

export function showSnackbar(snackbar: Omit<Snackbar, 'id'>): string {
	const id = crypto.randomUUID();
	const entry: Snackbar = { id, showClose: true, duration: 4000, ...snackbar };
	snackbars.update((all) => [...all, entry]);
	if (entry.duration && entry.duration > 0) {
		setTimeout(() => dismissSnackbar(id), entry.duration);
	}
	return id;
}

export function dismissSnackbar(id: string) {
	snackbars.update((all) => all.filter((s) => s.id !== id));
}

export function updateSnackbar(id: string, patch: Partial<Snackbar>) {
	snackbars.update((all) => all.map((s) => (s.id === id ? { ...s, ...patch } : s)));
}

export function clearSnackbar() {
	snackbars.set([]);
}
