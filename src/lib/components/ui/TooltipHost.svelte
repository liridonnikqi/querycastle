<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { clampTooltipToAnchor } from '$lib/utils/viewport';

	const DELAY_MS = 420;
	const MAX_CHARS = 400;

	let text = $state('');
	let x = $state(0);
	let y = $state(0);
	let visible = $state(false);
	let tipEl = $state<HTMLDivElement | null>(null);
	let timer: ReturnType<typeof setTimeout> | undefined;
	let source: Element | null = null;

	function normalize(raw: string) {
		const next = raw.trim();
		if (!next) return '';
		return next.length > MAX_CHARS ? `${next.slice(0, MAX_CHARS - 1)}…` : next;
	}

	function hide() {
		clearTimeout(timer);
		timer = undefined;
		visible = false;
		text = '';
		source = null;
	}

	function place(el: Element) {
		const anchor = el.getBoundingClientRect();
		const width = tipEl?.offsetWidth ?? 160;
		const height = tipEl?.offsetHeight ?? 28;
		const next = clampTooltipToAnchor(
			anchor,
			width,
			height,
			window.innerWidth,
			window.innerHeight,
		);
		x = next.x;
		y = next.y;
	}

	function tipFrom(event: Event): Element | null {
		const target = event.target;
		if (!(target instanceof Element)) return null;
		return target.closest('[data-tip]');
	}

	function schedule(el: Element) {
		const next = normalize(el.getAttribute('data-tip') ?? '');
		if (!next) {
			hide();
			return;
		}
		clearTimeout(timer);
		source = el;
		timer = setTimeout(() => {
			void (async () => {
				if (source !== el) return;
				text = next;
				visible = true;
				place(el);
				await tick();
				if (source === el) place(el);
			})();
		}, DELAY_MS);
	}

	onMount(() => {
		const onOver = (event: PointerEvent) => {
			const el = tipFrom(event);
			if (!el) return;
			const related =
				event.relatedTarget instanceof Element
					? event.relatedTarget.closest('[data-tip]')
					: null;
			if (related === el && source === el) return;
			schedule(el);
		};
		const onOut = (event: PointerEvent) => {
			const leaving = tipFrom(event);
			if (!leaving || leaving !== source) return;
			const entering =
				event.relatedTarget instanceof Element
					? event.relatedTarget.closest('[data-tip]')
					: null;
			if (entering === leaving) return;
			hide();
		};
		const onCancel = () => hide();

		document.addEventListener('pointerover', onOver, true);
		document.addEventListener('pointerout', onOut, true);
		document.addEventListener('pointerdown', onCancel, true);
		document.addEventListener('keydown', onCancel, true);
		document.addEventListener('scroll', onCancel, true);
		document.addEventListener('contextmenu', onCancel, true);
		window.addEventListener('blur', onCancel);
		return () => {
			hide();
			document.removeEventListener('pointerover', onOver, true);
			document.removeEventListener('pointerout', onOut, true);
			document.removeEventListener('pointerdown', onCancel, true);
			document.removeEventListener('keydown', onCancel, true);
			document.removeEventListener('scroll', onCancel, true);
			document.removeEventListener('contextmenu', onCancel, true);
			window.removeEventListener('blur', onCancel);
		};
	});
</script>

{#if visible && text}
	<div
		bind:this={tipEl}
		class="qc-tooltip"
		style={`left:${x}px;top:${y}px;`}
		role="tooltip"
	>
		{text}
	</div>
{/if}
