export const gridChrome = $state({
	changeCount: 0,
	pendingOpenNonce: 0,
});

export function requestOpenPendingChanges() {
	gridChrome.pendingOpenNonce += 1;
}
