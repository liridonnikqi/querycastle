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
		document.documentElement.setAttribute('data-theme', next);
		try {
			localStorage.setItem(THEME_STORAGE_KEY, next);
		} catch {
			// ignore quota / private-mode failures
		}
	}

	toggle() {
		this.set(this.value === 'light' ? 'dark' : 'light');
	}
}

export const theme = new ThemeStore();
