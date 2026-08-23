import { check } from '@tauri-apps/plugin-updater';
import { ask, message } from '@tauri-apps/plugin-dialog';

export type UpdaterStatus =
	| 'idle'
	| 'checking'
	| 'available'
	| 'downloading'
	| 'ready'
	| 'uptodate'
	| 'error';

export interface UpdateInfo {
	version: string;
	body?: string;
	date?: string;
}

/**
 * Check for updates. Returns Update object or null if no update.
 * Wrapper to handle isTauri check outside.
 */
export async function checkForUpdate() {
	return await check();
}

/**
 * Download and install with optional progress callback.
 * After install, asks user to restart via dialog.
 * Uses `relaunch` if `@tauri-apps/plugin-process` is installed,
 * otherwise just informs user to restart manually.
 */
export async function downloadAndInstallUpdate(
	update: Awaited<ReturnType<typeof check>>,
	onProgress?: (progress: { downloaded: number; total?: number }) => void
) {
	if (!update) return;
	let downloaded = 0;
	let contentLength: number | undefined;

	await update.downloadAndInstall((event) => {
		switch (event.event) {
			case 'Started':
				contentLength = event.data.contentLength;
				break;
			case 'Progress':
				downloaded += event.data.chunkLength;
				onProgress?.({ downloaded, total: contentLength });
				break;
			case 'Finished':
				break;
		}
	});

	// Try to relaunch if plugin-process is available, otherwise inform user
	try {
		// dynamic import so build doesn't fail if plugin not installed
		const { relaunch } = await import('@tauri-apps/plugin-process');
		const shouldRelaunch = await ask(
			`Update ${update.version} installed. Restart now?`,
			{ title: 'Update ready', kind: 'info' }
		);
		if (shouldRelaunch) await relaunch();
	} catch {
		await message(`Update ${update.version} installed. Please restart QueryCastle to apply.`, {
			title: 'Update ready',
			kind: 'info'
		});
	}
}
