export type ToastKind = 'success' | 'error';

export type ToastItem = {
	id: number;
	kind: ToastKind;
	message: string;
};

class ToastStore {
	items = $state<ToastItem[]>([]);
	#nextId = 1;
	#timers = new Map<number, ReturnType<typeof setTimeout>>();

	show(kind: ToastKind, message: string, durationMs = kind === 'error' ? 6000 : 3500) {
		const text = message.trim();
		if (!text) return;
		const id = this.#nextId++;
		this.items = [...this.items, { id, kind, message: text }];
		const timer = setTimeout(() => this.dismiss(id), durationMs);
		this.#timers.set(id, timer);
	}

	success(message: string) {
		this.show('success', message);
	}

	error(message: string) {
		this.show('error', message);
	}

	dismiss(id: number) {
		const timer = this.#timers.get(id);
		if (timer) clearTimeout(timer);
		this.#timers.delete(id);
		this.items = this.items.filter((item) => item.id !== id);
	}
}

export const toast = new ToastStore();
