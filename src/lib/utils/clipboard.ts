/**
 * Best-effort clipboard copy.
 *
 * `navigator.clipboard.writeText` rejects when the document is not focused
 * (e.g. right after interacting with a Tauri webview menu), so fall back to
 * the legacy `execCommand('copy')` path before reporting failure.
 *
 * Returns true when the text was (probably) copied.
 */
export async function copyTextToClipboard(text: string): Promise<boolean> {
	try {
		if (
			typeof navigator !== 'undefined' &&
			navigator.clipboard &&
			typeof navigator.clipboard.writeText === 'function'
		) {
			await navigator.clipboard.writeText(text);
			return true;
		}
	} catch {
		// Fall through to the legacy path below.
	}
	try {
		if (typeof document === 'undefined') return false;
		const area = document.createElement('textarea');
		area.value = text;
		area.setAttribute('readonly', '');
		area.style.position = 'fixed';
		area.style.top = '-9999px';
		area.style.opacity = '0';
		document.body.appendChild(area);
		area.select();
		const ok = document.execCommand('copy');
		area.remove();
		return ok;
	} catch {
		return false;
	}
}
