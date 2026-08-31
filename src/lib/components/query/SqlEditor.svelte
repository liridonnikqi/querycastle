<script lang="ts">
	import { onMount } from 'svelte';
	import { EditorView } from 'codemirror';
	import { Compartment, EditorState } from '@codemirror/state';
	import { keymap, lineNumbers } from '@codemirror/view';
	import { MySQL, PostgreSQL, SQLite, sql } from '@codemirror/lang-sql';
	import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
	import { tags } from '@lezer/highlight';
	import { Play, Save, WandSparkles } from '@lucide/svelte';
	import { theme } from '$lib/theme.svelte';
	import type { DatabaseExplorer, DatabaseType } from '$lib/rpc';
	import { explorerToSqlSchema } from '$lib/utils/schema-objects';

	let {
		value,
		onChange,
		onRun,
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
		onSaveQuery: () => void;
		onFormatQuery: () => void;
		running: boolean;
		disabled: boolean;
		explorer?: DatabaseExplorer | null;
		databaseType?: DatabaseType;
	} = $props();

	let editorContainer = $state<HTMLDivElement | null>(null);
	let editorView = $state<EditorView | null>(null);
	let applyingExternalUpdate = false;
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
					: PostgreSQL;
		return sql({
			dialect,
			schema: explorerToSqlSchema(explorer),
			upperCaseKeywords: true,
		});
	}

	function getSelectedQuery() {
		if (!editorView) return '';
		const selection = editorView.state.selection.main;
		if (selection.from === selection.to) return '';
		return editorView.state.doc
			.sliceString(selection.from, selection.to)
			.trim();
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
				'.cm-content': {
					fontFamily: '"JetBrains Mono Variable", "JetBrains Mono", ui-monospace, monospace',
					fontSize: '13px',
					lineHeight: '1.6',
					padding: '10px 0',
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
				'.cm-activeLineGutter': { backgroundColor: '#f4f4f5', color: '#18181b' },
				'.cm-activeLine': { backgroundColor: '#f4f4f5' },
				'.cm-cursor': { borderLeftColor: '#18181b', borderLeftWidth: '2px' },
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
					backgroundColor: 'var(--qc-bg)',
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
				'.cm-activeLine': { backgroundColor: 'var(--qc-panel)' },
				'.cm-activeLineGutter': {
					backgroundColor: 'var(--qc-panel)',
					color: 'var(--qc-fg)',
				},
				'.cm-cursor': {
					borderLeftColor: 'var(--qc-fg)',
					borderLeftWidth: '2px',
				},
				'.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
					backgroundColor: 'var(--qc-select-row)',
				},
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
		if (disabled || running) return;
		const selected = getSelectedQuery();
		if (selected.length > 0) {
			onRun(selected);
			return;
		}
		onRun();
	}

	onMount(() => {
		if (!editorContainer) return;
		const isDark = theme.value === 'dark';

		const state = EditorState.create({
			doc: value,
			extensions: [
				EditorView.lineWrapping,
				lineNumbers({
					formatNumber: (lineNo) => lineNo.toString(),
				}),
				languageCompartment.of(sqlLanguageExtension()),
				highlightCompartment.of(editorHighlight(isDark)),
				themeCompartment.of(editorThemeExtensions(isDark)),
				keymap.of([
					{ key: 'Mod-Enter', run: () => (runEditorAction(), true) },
					{ key: 'Mod-s', run: () => (onSaveQuery(), true) },
					{ key: 'Shift-Alt-f', run: () => (onFormatQuery(), true) },
				]),
				EditorView.updateListener.of((update) => {
					if (!update.docChanged || applyingExternalUpdate) return;
					onChange(update.state.doc.toString());
				}),
			],
		});

		editorView = new EditorView({ state, parent: editorContainer });
		return () => {
			editorView?.destroy();
			editorView = null;
		};
	});

	$effect(() => {
		if (!editorView) return;
		const current = editorView.state.doc.toString();
		if (current === value) return;

		applyingExternalUpdate = true;
		editorView.dispatch({
			changes: { from: 0, to: current.length, insert: value },
		});
		applyingExternalUpdate = false;
	});

	$effect(() => {
		if (!editorView) return;
		explorer;
		databaseType;
		editorView.dispatch({
			effects: languageCompartment.reconfigure(sqlLanguageExtension()),
		});
	});

	$effect(() => {
		if (!editorView) return;
		const isDark = theme.value === 'dark';
		editorView.dispatch({
			effects: [
				themeCompartment.reconfigure(editorThemeExtensions(isDark)),
				highlightCompartment.reconfigure(editorHighlight(isDark)),
			],
		});
	});
</script>

<div class="flex-1 flex flex-col min-h-0 bg-qc-bg">
	<div
		class="flex items-center justify-between px-3 h-10 border-b border-qc-border bg-qc-bg shrink-0"
	>
		<div class="flex items-center gap-1.5">
			<button
				onclick={runEditorAction}
				disabled={disabled || running}
				class="flex items-center gap-1.5 btn-primary disabled:opacity-60 h-7 px-2.5 text-[12px] font-medium"
			>
				<Play size={14} />
				<span>{running ? 'Running...' : 'Run'}</span>
			</button>
			<button
				onclick={onFormatQuery}
				class="flex items-center gap-1.5 btn-secondary h-7 px-2.5 text-[12px] font-medium"
			>
				<WandSparkles size={14} />
				<span>Format</span>
			</button>
		</div>
		<button
			onclick={onSaveQuery}
			class="w-7 h-7 text-qc-muted hover:text-qc-fg hover:bg-qc-hover rounded-md flex items-center justify-center"
			title="Save (Ctrl+S)"
		>
			<Save size={16} />
		</button>
	</div>
	<div class="flex-1 min-h-0 overflow-hidden">
		<div bind:this={editorContainer} class="h-full"></div>
	</div>
</div>
