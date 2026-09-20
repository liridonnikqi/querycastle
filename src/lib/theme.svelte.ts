export type Theme = 'light' | 'dark';

export const THEME_STORAGE_KEY = 'qc-theme';

function readDomTheme(): Theme {
	if (typeof document === 'undefined') return 'dark';
	const attr = document.documentElement.getAttribute('data-theme');
	return attr === 'light' ? 'light' : 'dark';
}

class ThemeStore {
	value = $state<Theme>(readDomTheme());

	set(next: Theme) {
		this.value = next;
		if (typeof document === 'undefined') return;
		const root = document.documentElement;
		root.classList.add('theme-switching');
		void root.offsetWidth;
		root.setAttribute('data-theme', next);
		try {
			localStorage.setItem(THEME_STORAGE_KEY, next);
		} catch {
			// ignore quota / private-mode failures
		}
		requestAnimationFrame(() => {
			requestAnimationFrame(() => {
				root.classList.remove('theme-switching');
			});
		});
	}

	toggle() {
		this.set(this.value === 'light' ? 'dark' : 'light');
	}
}

export const theme = new ThemeStore();
