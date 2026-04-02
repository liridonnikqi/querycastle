<script lang="ts">
	import { onMount } from "svelte";
	import { EditorView } from "codemirror";
	import { Compartment, EditorState } from "@codemirror/state";
	import { keymap } from "@codemirror/view";
	import { sql as sqlLanguage } from "@codemirror/lang-sql";
	import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
	import { tags } from "@lezer/highlight";
	import { autocompletion, completeFromList } from "@codemirror/autocomplete";
	import { Play, Save, WandSparkles } from "@lucide/svelte";

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
		{ tag: [tags.keyword, tags.operatorKeyword], color: "#2563eb", fontWeight: "600" },
		{ tag: [tags.name, tags.variableName], color: "#111827" },
		{ tag: [tags.propertyName, tags.attributeName], color: "#1d4ed8" },
		{ tag: [tags.string, tags.special(tags.string)], color: "#059669" },
		{ tag: [tags.number, tags.integer, tags.float], color: "#ea580c" },
		{ tag: [tags.comment, tags.lineComment, tags.blockComment], color: "#9ca3af", fontStyle: "italic" },
		{ tag: [tags.function(tags.name), tags.function(tags.variableName)], color: "#6366f1" },
	]);

	function buildAutocompleteExtension() {
		return autocompletion({
			override: [
				completeFromList(
					completions.map((label) => ({
						label,
						type: "variable",
					})),
				),
			],
		});
	}

	function getSelectedQuery() {
		if (!editorView) return "";
		const selection = editorView.state.selection.main;
		if (selection.from === selection.to) return "";
		return editorView.state.doc.sliceString(selection.from, selection.to).trim();
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

		const state = EditorState.create({
			doc: value,
			extensions: [
				EditorView.lineWrapping,
				sqlLanguage(),
				syntaxHighlighting(sqlHighlightStyle),
				keymap.of([
					{ key: "Mod-Enter", run: () => (runEditorAction(), true) },
					{ key: "Mod-s", run: () => (onSaveQuery(), true) },
					{ key: "Shift-Alt-f", run: () => (onFormatQuery(), true) },
				]),
				autocompleteCompartment.of(buildAutocompleteExtension()),
				EditorView.theme({
					"&": { height: "100%", backgroundColor: "#ffffff" },
					".cm-editor": { backgroundColor: "#ffffff" },
					".cm-content": {
						fontFamily: '"JetBrains Mono", "SF Mono", Menlo, Monaco, monospace',
						fontSize: "13px",
						lineHeight: "1.75",
						color: "#1f2937",
					},
					".cm-scroller": { overflow: "auto" },
					".cm-activeLine": { backgroundColor: "#f8fafc" },
					".cm-selectionBackground": { backgroundColor: "rgba(37,99,235,0.18) !important" },
					".cm-cursor": { borderLeftColor: "#111827" },
					".cm-gutters": {
						backgroundColor: "#f9fafb",
						borderRight: "1px solid #f3f4f6",
						color: "#9ca3af",
					},
				}),
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
		editorView.dispatch({ changes: { from: 0, to: current.length, insert: value } });
		applyingExternalUpdate = false;
	});

	$effect(() => {
		if (!editorView) return;
		editorView.dispatch({ effects: autocompleteCompartment.reconfigure(buildAutocompleteExtension()) });
	});
</script>

<div class="flex-1 flex flex-col min-h-0 bg-white">
	<div class="flex items-center justify-between px-4 py-2 border-b border-gray-100 bg-white shrink-0">
		<div class="flex items-center space-x-2">
			<button
				onclick={runEditorAction}
				disabled={disabled || running}
				class="flex items-center space-x-1.5 bg-emerald-500 hover:bg-emerald-600 disabled:opacity-60 text-white px-3 py-1.5 rounded text-xs font-medium transition-colors shadow-sm"
			>
				<Play size={14} />
				<span>{running ? "Running..." : "Run"}</span>
			</button>
			<button onclick={onFormatQuery} class="flex items-center space-x-1.5 bg-white border border-gray-200 hover:bg-gray-50 text-gray-700 px-3 py-1.5 rounded text-xs font-medium transition-colors shadow-sm">
				<WandSparkles size={14} class="text-gray-500" />
				<span>Format</span>
			</button>
			<div class="h-4 w-px bg-gray-200 mx-1"></div>
			<button onclick={onSaveQuery} class="p-1.5 text-gray-400 hover:text-gray-700 hover:bg-gray-100 rounded transition-colors flex items-center justify-center" title="Save">
				<Save size={16} />
			</button>
		</div>
		<div class="text-xs text-gray-400 font-medium">Saved locally</div>
	</div>
	<div class="flex-1 min-h-0 overflow-hidden">
		<div bind:this={editorContainer} class="h-full"></div>
	</div>
</div>
