<script lang="ts">
	import { onMount } from 'svelte';
	import { EditorView } from 'codemirror';
	import { Compartment, EditorState } from '@codemirror/state';
	import { keymap, lineNumbers } from '@codemirror/view';
	import { sql as sqlLanguage } from '@codemirror/lang-sql';
	import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
	import { tags } from '@lezer/highlight';
	import { autocompletion, CompletionContext } from '@codemirror/autocomplete';
	import { linter } from '@codemirror/lint';
	import { Play, Save, WandSparkles } from '@lucide/svelte';

	let {
		value,
		onChange,
		onRun,
		onSaveQuery,
		onFormatQuery,
		running,
		disabled,
		completions,
	}: {
		value: string;
		onChange: (value: string) => void;
		onRun: (query?: string) => void;
		onSaveQuery: () => void;
		onFormatQuery: () => void;
		running: boolean;
		disabled: boolean;
		completions: string[];
	} = $props();

	let editorContainer = $state<HTMLDivElement | null>(null);
	let editorView = $state<EditorView | null>(null);
	let applyingExternalUpdate = false;
	const autocompleteCompartment = new Compartment();

	const sqlHighlightStyle = HighlightStyle.define([
		{
			tag: [tags.keyword, tags.operatorKeyword],
			color: '#0066cc',
			fontWeight: '600',
		},
		{ tag: [tags.name, tags.variableName], color: '#22863a' },
		{ tag: [tags.propertyName, tags.attributeName], color: '#005a9c' },
		{ tag: [tags.string, tags.special(tags.string)], color: '#032f62' },
		{ tag: [tags.number, tags.integer, tags.float], color: '#d73a49' },
		{
			tag: [tags.comment, tags.lineComment, tags.blockComment],
			color: '#6a737d',
			fontStyle: 'italic',
		},
		{
			tag: [tags.function(tags.name), tags.function(tags.variableName)],
			color: '#6f42c1',
		},
		{ tag: [tags.operator], color: '#005a9c' },
	]);

	const statementKeywords = [
		'SELECT',
		'INSERT',
		'UPDATE',
		'DELETE',
		'CREATE',
		'ALTER',
		'DROP',
		'TRUNCATE',
		'WITH',
		'EXPLAIN',
		'VALUES',
		'SHOW',
		'BEGIN',
		'COMMIT',
		'ROLLBACK',
	] as const;

	function levenshteinDistance(a: string, b: string) {
		const rows = a.length + 1;
		const cols = b.length + 1;
		const dp: number[][] = Array.from({ length: rows }, () =>
			Array.from({ length: cols }, () => 0),
		);

		for (let i = 0; i < rows; i++) dp[i][0] = i;
		for (let j = 0; j < cols; j++) dp[0][j] = j;

		for (let i = 1; i < rows; i++) {
			for (let j = 1; j < cols; j++) {
				const cost = a[i - 1] === b[j - 1] ? 0 : 1;
				dp[i][j] = Math.min(
					dp[i - 1][j] + 1,
					dp[i][j - 1] + 1,
					dp[i - 1][j - 1] + cost,
				);
			}
		}

		return dp[a.length][b.length];
	}

	function findFirstKeywordTypo(doc: string) {
		const firstTokenMatch = doc.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)/);
		if (!firstTokenMatch) return null;

		const typed = firstTokenMatch[1];
		const typedUpper = typed.toUpperCase();
		if (
			statementKeywords.includes(
				typedUpper as (typeof statementKeywords)[number],
			)
		) {
			return null;
		}

		let best: { keyword: string; distance: number } | null = null;
		for (const keyword of statementKeywords) {
			const distance = levenshteinDistance(typedUpper, keyword);
			if (distance > 2) continue;
			if (!best || distance < best.distance) {
				best = { keyword, distance };
			}
		}

		if (!best) return null;
		const from = firstTokenMatch.index ?? 0;
		return {
			from,
			to: from + typed.length,
			message: `Unknown SQL keyword '${typed}'. Did you mean '${best.keyword}'?`,
		};
	}

	function buildAutocompleteExtension() {
		return autocompletion({
			override: [
				(context: CompletionContext) => {
					const word = context.matchBefore(/\w*/);
					if (!word || (word.from === word.to && !context.explicit))
						return null;

					// Get unique completions and sort them
					const uniqueCompletions = Array.from(new Set(completions));
					uniqueCompletions.sort();

					const options = uniqueCompletions.map((label) => {
						const isKeyword = label.toUpperCase() === label;
						return {
							label,
							type: isKeyword ? 'keyword' : 'variable',
							boost: isKeyword ? 1 : 0,
						};
					});

					return {
						from: word.from,
						options,
						validFor: /^\w*$/,
					};
				},
			],
			closeOnBlur: true,
		});
	}

	function sqlLinter(view: EditorView) {
		const diagnostics: Array<{
			from: number;
			to: number;
			severity: 'error' | 'warning';
			message: string;
		}> = [];
		const doc = view.state.doc.toString().trim();

		if (!doc) return diagnostics;

		let parenCount = 0;
		let bracketCount = 0;
		let inString = false;
		let stringChar = '';
		let i = 0;

		// Check for unclosed parentheses and brackets
		while (i < doc.length) {
			const char = doc[i];

			// Handle string literals
			if ((char === "'" || char === '"') && (i === 0 || doc[i - 1] !== '\\')) {
				if (!inString) {
					inString = true;
					stringChar = char;
				} else if (char === stringChar) {
					inString = false;
				}
			}

			// Count brackets only when not in string
			if (!inString) {
				if (char === '(') parenCount++;
				else if (char === ')') parenCount--;
				else if (char === '[') bracketCount++;
				else if (char === ']') bracketCount--;

				// Check for invalid bracket nesting
				if (parenCount < 0) {
					diagnostics.push({
						from: i,
						to: i + 1,
						severity: 'error',
						message: 'Unexpected closing parenthesis',
					});
					parenCount = 0;
				}
				if (bracketCount < 0) {
					diagnostics.push({
						from: i,
						to: i + 1,
						severity: 'error',
						message: 'Unexpected closing bracket',
					});
					bracketCount = 0;
				}
			}

			i++;
		}

		// Check for unclosed parentheses at end
		if (parenCount > 0) {
			diagnostics.push({
				from: Math.max(0, doc.length - 1),
				to: doc.length,
				severity: 'error',
				message: `Missing ${parenCount} closing parenthesis/parentheses`,
			});
		}

		// Check for unclosed brackets at end
		if (bracketCount > 0) {
			diagnostics.push({
				from: Math.max(0, doc.length - 1),
				to: doc.length,
				severity: 'error',
				message: `Missing ${bracketCount} closing bracket/brackets`,
			});
		}

		// Check for unclosed string
		if (inString) {
			diagnostics.push({
				from: Math.max(0, doc.length - 1),
				to: doc.length,
				severity: 'error',
				message: `Unclosed string literal (${stringChar})`,
			});
		}

		const typo = findFirstKeywordTypo(doc);
		if (typo) {
			diagnostics.push({
				from: typo.from,
				to: typo.to,
				severity: 'error',
				message: typo.message,
			});
		}

		return diagnostics;
	}

	function getSelectedQuery() {
		if (!editorView) return '';
		const selection = editorView.state.selection.main;
		if (selection.from === selection.to) return '';
		return editorView.state.doc
			.sliceString(selection.from, selection.to)
			.trim();
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

		const editorTheme = EditorView.theme(
			{
				'&': {
					height: '100%',
					backgroundColor: '#ffffff',
					color: '#24292e',
					fontSize: '13px',
				},
				'.cm-content': {
					fontFamily:
						'"JetBrains Mono", "SF Mono", Menlo, Monaco, "Courier New", monospace',
					fontSize: '13px',
					lineHeight: '1.6',
					padding: '10px 0',
				},
				'.cm-gutters': {
					backgroundColor: '#f6f8fa',
					borderRight: '1px solid #e1e4e8',
					color: '#6a737d',
					padding: '0 8px',
				},
				'.cm-linenumber': {
					color: '#959da5',
					minWidth: '45px',
					textAlign: 'right',
					paddingRight: '8px',
					fontSize: '12px',
				},
				'.cm-activeLineGutter': {
					backgroundColor: '#fffbdd',
					color: '#24292e',
					fontWeight: '500',
				},
				'.cm-activeLine': {
					backgroundColor: '#fffbdd',
				},
				'.cm-selectionBackground': {
					backgroundColor: 'rgba(3, 47, 98, 0.15) !important',
				},
				'.cm-selection': {
					backgroundColor: 'rgba(3, 47, 98, 0.15) !important',
				},
				'.cm-cursor': {
					borderLeftColor: '#24292e',
					borderLeftWidth: '2px',
				},
				'.cm-searchMatch': {
					backgroundColor: 'rgba(250, 182, 6, 0.3)',
					outline: '1px solid #f9b82e',
				},
				'.cm-searchMatch.cm-searchMatch-selected': {
					backgroundColor: 'rgba(250, 182, 6, 0.5)',
				},
				// Autocomplete styling
				'.cm-completionLabel': {
					fontWeight: '500',
				},
				'.cm-tooltip': {
					backgroundColor: '#f6f8fa',
					border: '1px solid #e1e4e8',
					borderRadius: '4px',
					boxShadow: '0 4px 12px rgba(0, 0, 0, 0.1)',
				},
				'.cm-completionOption': {
					padding: '4px 8px',
					color: '#24292e',
					fontSize: '12px',
				},
				'.cm-completionOption[aria-selected]': {
					backgroundColor: '#f0f4f8',
					color: '#0066cc',
				},
				'.cm-completionInfo': {
					color: '#6a737d',
					fontSize: '11px',
					borderLeftColor: '#e1e4e8',
				},
				// Lint styling
				'.cm-diagnostic': {
					padding: '4px 6px',
					marginLeft: '4px',
					borderRadius: '3px',
					fontSize: '12px',
				},
				'.cm-diagnostic-error': {
					borderLeft: '3px solid #d1130c',
					backgroundColor: 'rgba(209, 19, 12, 0.08)',
				},
				'.cm-lintRange-error': {
					backgroundImage:
						"url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='6' height='3'%3E%3Cpath d='M0 3 Q 1.5 0 3 3 Q 1.5 3 0 3' fill='%23d1130c' /%3E%3C/svg%3E\")",
					backgroundRepeat: 'repeat-x',
					backgroundPosition: '0 100%',
					backgroundSize: '6px 3px',
				},
				'.cm-gutter-lint': {
					width: '30px',
					paddingRight: '4px',
					textAlign: 'center',
				},
				'.cm-lint-marker-error': {
					content: "'●'",
					color: '#d1130c',
					fontSize: '16px',
					lineHeight: '1',
				},
			},
			{ dark: false },
		);

		const state = EditorState.create({
			doc: value,
			extensions: [
				EditorView.lineWrapping,
				lineNumbers({
					formatNumber: (lineNo) => lineNo.toString(),
				}),
				sqlLanguage(),
				syntaxHighlighting(sqlHighlightStyle),
				editorTheme,
				linter(sqlLinter),
				keymap.of([
					{ key: 'Mod-Enter', run: () => (runEditorAction(), true) },
					{ key: 'Mod-s', run: () => (onSaveQuery(), true) },
					{ key: 'Shift-Alt-f', run: () => (onFormatQuery(), true) },
				]),
				autocompleteCompartment.of(buildAutocompleteExtension()),
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
		editorView.dispatch({
			effects: autocompleteCompartment.reconfigure(
				buildAutocompleteExtension(),
			),
		});
	});
</script>

<div class="flex-1 flex flex-col min-h-0 bg-white">
	<div
		class="flex items-center justify-between px-4 py-2 border-b border-gray-100 bg-white shrink-0"
	>
		<div class="flex items-center space-x-2">
			<button
				onclick={runEditorAction}
				disabled={disabled || running}
				class="flex items-center space-x-1.5 bg-emerald-500 hover:bg-emerald-600 disabled:opacity-60 text-white px-3 py-1.5 rounded text-xs font-medium transition-colors shadow-sm"
			>
				<Play size={14} />
				<span>{running ? 'Running...' : 'Run'}</span>
			</button>
			<button
				onclick={onFormatQuery}
				class="flex items-center space-x-1.5 bg-white border border-gray-200 hover:bg-gray-50 text-gray-700 px-3 py-1.5 rounded text-xs font-medium transition-colors shadow-sm"
			>
				<WandSparkles size={14} class="text-gray-500" />
				<span>Format</span>
			</button>
		</div>
		<button
			onclick={onSaveQuery}
			class="p-1.5 text-gray-400 hover:text-gray-700 hover:bg-gray-100 rounded transition-colors flex items-center justify-center"
			title="Save (Ctrl+S)"
		>
			<Save size={16} />
		</button>
	</div>
	<div class="flex-1 min-h-0 overflow-hidden">
		<div bind:this={editorContainer} class="h-full"></div>
	</div>
</div>
