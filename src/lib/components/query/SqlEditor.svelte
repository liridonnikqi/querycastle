<script lang="ts">
	import { untrack } from 'svelte';
	import {
		autocompletion,
		closeBrackets,
		closeBracketsKeymap,
		completionKeymap,
	} from '@codemirror/autocomplete';
	import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
	import { MSSQL, MySQL, PostgreSQL, SQLite, sql } from '@codemirror/lang-sql';
	import {
		HighlightStyle,
		bracketMatching,
		defaultHighlightStyle,
		indentOnInput,
		syntaxHighlighting,
	} from '@codemirror/language';
	import { Compartment, EditorState } from '@codemirror/state';
	import {
		EditorView,
		drawSelection,
		highlightActiveLine,
		highlightActiveLineGutter,
		highlightSpecialChars,
		keymap,
		lineNumbers,
	} from '@codemirror/view';
	import { tags } from '@lezer/highlight';
	import { Play, Save, Square, WandSparkles } from '$lib/icons';
	import { theme } from '$lib/theme.svelte';
	import type { DatabaseExplorer, DatabaseType } from '$lib/rpc';
	import { explorerToSqlSchema } from '$lib/utils/schema-objects';

	let {
		value,
		onChange,
		onRun,
		onCancel,
		onSaveQuery,
		onFormatQuery,
		running,
		disabled,
		explorer = null,
		databaseType = 'postgres',
	}: {
		value: string;
		onChange: (value: string) => void;
		onRun: (query?: string) => void;
		onCancel?: () => void;
		onSaveQuery: () => void;
		onFormatQuery: () => void;
		running: boolean;
		disabled: boolean;
		explorer?: DatabaseExplorer | null;
		databaseType?: DatabaseType;
	} = $props();

	let editorContainer = $state<HTMLDivElement | undefined>();
	let editorView = $state.raw<EditorView | null>(null);
	let applyingExternalUpdate = false;
	let skipNextLanguageSync = true;
	let skipNextThemeSync = true;
	let selectedQuery = $state('');
	const hasSelection = $derived(selectedQuery.length > 0);
	const languageCompartment = new Compartment();
	const themeCompartment = new Compartment();
	const highlightCompartment = new Compartment();

	const sqlHighlightStyleLight = HighlightStyle.define([
		{
			tag: [tags.keyword, tags.operatorKeyword],
			color: '#2563eb',
			fontWeight: '600',
		},
		{ tag: [tags.name, tags.variableName], color: '#15803d' },
		{ tag: [tags.propertyName, tags.attributeName], color: '#1d4ed8' },
		{ tag: [tags.string, tags.special(tags.string)], color: '#0f766e' },
		{ tag: [tags.number, tags.integer, tags.float], color: '#b91c1c' },
		{
			tag: [tags.comment, tags.lineComment, tags.blockComment],
			color: '#71717a',
			fontStyle: 'italic',
		},
		{
			tag: [tags.function(tags.name), tags.function(tags.variableName)],
			color: '#7c3aed',
		},
		{ tag: [tags.operator], color: '#334155' },
	]);

	const sqlHighlightStyleDark = HighlightStyle.define([
		{
			tag: [tags.keyword, tags.operatorKeyword],
			color: '#c4b5fd',
			fontWeight: '600',
		},
		{ tag: [tags.name, tags.variableName], color: '#86efac' },
		{ tag: [tags.propertyName, tags.attributeName], color: '#93c5fd' },
		{ tag: [tags.string, tags.special(tags.string)], color: '#5eead4' },
		{ tag: [tags.number, tags.integer, tags.float], color: '#fca5a5' },
		{
			tag: [tags.comment, tags.lineComment, tags.blockComment],
			color: '#71717a',
			fontStyle: 'italic',
		},
		{
			tag: [tags.function(tags.name), tags.function(tags.variableName)],
			color: '#d8b4fe',
		},
		{ tag: [tags.operator], color: '#a1a1aa' },
	]);

	function sqlLanguageExtension() {
		const dialect =
			databaseType === 'mysql'
				? MySQL
				: databaseType === 'sqlite'
					? SQLite
					: databaseType === 'mssql'
						? MSSQL
						: PostgreSQL;
		return sql({
			dialect,
			schema: explorerToSqlSchema(explorer),
			upperCaseKeywords: true,
		});
	}

	function selectedQueryFrom(state: EditorState) {
		const selection = state.selection.main;
		if (selection.from === selection.to) return '';
		return state.doc.sliceString(selection.from, selection.to).trim();
	}

	function lightEditorTheme() {
		return EditorView.theme(
			{
				'&': {
					height: '100%',
					backgroundColor: '#ffffff',
					color: '#18181b',
					fontSize: '13px',
				},
				'.cm-scroller': { backgroundColor: '#ffffff' },
				'.cm-content': {
					fontFamily:
						'"JetBrains Mono Variable", "JetBrains Mono", ui-monospace, monospace',
					fontSize: '13px',
					lineHeight: '1.6',
					padding: '10px 0',
					caretColor: '#18181b',
					backgroundColor: 'transparent',
				},
				'.cm-gutters': {
					backgroundColor: '#ffffff',
					borderRight: '1px solid #e4e4e7',
					color: '#71717a',
					fontFamily:
						'"JetBrains Mono Variable", "JetBrains Mono", ui-monospace, monospace',
					fontSize: '13px',
					lineHeight: '1.6',
				},
				'.cm-lineNumbers .cm-gutterElement': {
					fontSize: '13px',
					lineHeight: '1.6',
					minWidth: '2.4rem',
					padding: '0 10px 0 8px',
				},
				'.cm-activeLine': { backgroundColor: 'rgba(24, 24, 27, 0.045)' },
				'.cm-activeLineGutter': {
					backgroundColor: 'rgba(24, 24, 27, 0.045)',
					color: '#18181b',
				},
				'.cm-cursor': { borderLeftColor: '#18181b', borderLeftWidth: '2px' },
				'.cm-selectionBackground': {
					backgroundColor: 'rgba(37, 99, 235, 0.22)',
				},
				'&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground':
					{
						backgroundColor: 'rgba(37, 99, 235, 0.28)',
					},
				'.cm-content ::selection': { backgroundColor: 'transparent' },
				'.cm-tooltip': {
					backgroundColor: '#ffffff',
					border: '1px solid #e4e4e7',
					borderRadius: '6px',
				},
			},
			{ dark: false },
		);
	}

	function darkEditorTheme() {
		return EditorView.theme(
			{
				'&': {
					height: '100%',
					backgroundColor: 'var(--qc-bg)',
					color: 'var(--qc-fg)',
					fontSize: '13px',
				},
				'.cm-scroller': { backgroundColor: 'var(--qc-bg)' },
				'.cm-content': {
					fontFamily:
						'"JetBrains Mono Variable", "JetBrains Mono", ui-monospace, monospace',
					fontSize: '13px',
					lineHeight: '1.6',
					padding: '10px 0',
					caretColor: 'var(--qc-fg)',
					backgroundColor: 'transparent',
				},
				'.cm-gutters': {
					backgroundColor: 'var(--qc-bg)',
					borderRight: '1px solid var(--qc-border)',
					color: 'var(--qc-muted)',
					fontFamily:
						'"JetBrains Mono Variable", "JetBrains Mono", ui-monospace, monospace',
					fontSize: '13px',
					lineHeight: '1.6',
				},
				'.cm-lineNumbers .cm-gutterElement': {
					fontSize: '13px',
					lineHeight: '1.6',
					minWidth: '2.4rem',
					padding: '0 10px 0 8px',
				},
				'.cm-activeLine': {
					backgroundColor: 'color-mix(in srgb, var(--qc-fg) 4.5%, transparent)',
				},
				'.cm-activeLineGutter': {
					backgroundColor: 'color-mix(in srgb, var(--qc-fg) 4.5%, transparent)',
					color: 'var(--qc-fg)',
				},
				'.cm-cursor': {
					borderLeftColor: 'var(--qc-fg)',
					borderLeftWidth: '2px',
				},
				'.cm-selectionBackground': {
					backgroundColor: 'var(--qc-select-row)',
				},
				'&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground':
					{
						backgroundColor: 'var(--qc-select-row)',
					},
				'.cm-content ::selection': { backgroundColor: 'transparent' },
				'.cm-tooltip': {
					backgroundColor: 'var(--qc-elevated)',
					border: '1px solid var(--qc-border)',
					borderRadius: '6px',
					color: 'var(--qc-fg)',
				},
			},
			{ dark: true },
		);
	}

	function editorThemeExtensions(isDark: boolean) {
		return isDark ? darkEditorTheme() : lightEditorTheme();
	}

	function editorHighlight(isDark: boolean) {
		return syntaxHighlighting(isDark ? sqlHighlightStyleDark : sqlHighlightStyleLight);
	}

	function runEditorAction() {
		if (disabled) return;
		if (running) {
			onCancel?.();
			return;
		}
		if (selectedQuery.length > 0) {
			onRun(selectedQuery);
			return;
		}
		onRun();
	}

	$effect(() => {
		const parent = editorContainer;
		if (!parent) return;

		const view = untrack(() => {
			const isDark = theme.value === 'dark';
			const state = EditorState.create({
				doc: value,
				extensions: [
					EditorView.lineWrapping,
					lineNumbers(),
					highlightSpecialChars(),
					history(),
					drawSelection(),
					highlightActiveLine(),
					highlightActiveLineGutter(),
					indentOnInput(),
					bracketMatching(),
					closeBrackets(),
					autocompletion(),
					syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
					languageCompartment.of(sqlLanguageExtension()),
					highlightCompartment.of(editorHighlight(isDark)),
					themeCompartment.of(editorThemeExtensions(isDark)),
					keymap.of([
						{ key: 'Mod-Enter', run: () => (runEditorAction(), true) },
						{ key: 'Mod-s', run: () => (onSaveQuery(), true) },
						{ key: 'Shift-Alt-f', run: () => (onFormatQuery(), true) },
						indentWithTab,
						...closeBracketsKeymap,
						...defaultKeymap,
						...historyKeymap,
						...completionKeymap,
					]),
					EditorView.updateListener.of((update) => {
						if (update.selectionSet || update.docChanged) {
							selectedQuery = selectedQueryFrom(update.state);
						}
						if (!update.docChanged || applyingExternalUpdate) return;
						onChange(update.state.doc.toString());
					}),
				],
			});
			return new EditorView({ state, parent });
		});

		skipNextLanguageSync = true;
		skipNextThemeSync = true;
		editorView = view;
		selectedQuery = selectedQueryFrom(view.state);
		return () => {
			view.destroy();
			if (editorView === view) editorView = null;
			selectedQuery = '';
		};
	});

	$effect(() => {
		const view = editorView;
		if (!view) return;
		const next = value;
		untrack(() => {
			const current = view.state.doc.toString();
			if (current === next) return;
			applyingExternalUpdate = true;
			try {
				view.dispatch({
					changes: { from: 0, to: current.length, insert: next },
				});
			} finally {
				applyingExternalUpdate = false;
			}
		});
	});

	$effect(() => {
		explorer;
		databaseType;
		const view = editorView;
		if (!view) return;
		if (skipNextLanguageSync) {
			skipNextLanguageSync = false;
			return;
		}
		untrack(() => {
			view.dispatch({
				effects: languageCompartment.reconfigure(sqlLanguageExtension()),
			});
		});
	});

	$effect(() => {
		const isDark = theme.value === 'dark';
		const view = editorView;
		if (!view) return;
		if (skipNextThemeSync) {
			skipNextThemeSync = false;
			return;
		}
		untrack(() => {
			view.dispatch({
				effects: [
					themeCompartment.reconfigure(editorThemeExtensions(isDark)),
					highlightCompartment.reconfigure(editorHighlight(isDark)),
				],
			});
		});
	});
</script>

<div class="flex-1 flex flex-col min-h-0 bg-qc-bg">
	<div
		class="h-10 px-2 border-b border-qc-border bg-qc-bg shrink-0 flex items-center"
	>
		<button
			onclick={runEditorAction}
			disabled={disabled || (running && !onCancel)}
			data-tip={running
				? 'Cancel running query'
				: hasSelection
					? 'Run the selected SQL only (Ctrl+Enter)'
					: 'Run query (Ctrl+Enter)'}
			class="btn-primary h-6 px-2 text-[12px] font-medium disabled:opacity-50 inline-flex items-center justify-center gap-1 shrink-0"
		>
			{#if running}
				<Square size={14} />Cancel
			{:else}
				<Play size={14} />Run
			{/if}
		</button>
		<div class="ml-auto flex items-center gap-0.5">
			<button
				type="button"
				onclick={onFormatQuery}
				class="toolbar-icon"
				data-tip="Format SQL (Shift+Alt+F)"
				aria-label="Format SQL"
			>
				<WandSparkles size={14} />
			</button>
			<button
				type="button"
				onclick={onSaveQuery}
				class="toolbar-icon"
				data-tip="Save (Ctrl+S)"
				aria-label="Save query"
			>
				<Save size={14} />
			</button>
		</div>
	</div>
	<div class="flex-1 min-h-0 overflow-hidden relative">
		<div bind:this={editorContainer} class="h-full min-h-0"></div>
		<div
			class={`qc-run-sel ${hasSelection ? 'is-on' : ''}`}
			aria-hidden={!hasSelection}
		>
			<button
				type="button"
				onclick={runEditorAction}
				disabled={disabled || running}
				tabindex={hasSelection ? 0 : -1}
				class="qc-run-sel-btn btn-secondary h-7 px-2.5 text-[12px] font-medium flex items-center gap-1.5"
			>
				<Play size={14} />
				<span>Run selection</span>
				<span class="font-mono text-[10px] text-qc-muted tracking-wide">Ctrl+Enter</span>
			</button>
		</div>
	</div>
</div>

<style>
	.qc-run-sel {
		position: absolute;
		left: 10px;
		bottom: 10px;
		z-index: 6;
		opacity: 0;
		transform: translateY(8px) scale(0.96);
		pointer-events: none;
		transition:
			opacity 180ms cubic-bezier(0.16, 1, 0.3, 1),
			transform 180ms cubic-bezier(0.16, 1, 0.3, 1);
	}

	.qc-run-sel.is-on {
		opacity: 1;
		transform: translateY(0) scale(1);
		pointer-events: auto;
	}

	.qc-run-sel-btn {
		box-shadow:
			inset 0 1px 0 0 var(--qc-btn-highlight),
			0 0 0 1px var(--qc-btn-ring),
			0 8px 20px rgba(0, 0, 0, 0.28);
	}

	.qc-run-sel-btn:not(:disabled):active {
		transform: none;
	}

	@media (prefers-reduced-motion: reduce) {
		.qc-run-sel {
			transition: opacity 80ms linear;
			transform: none;
		}
	}
</style>
