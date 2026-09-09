import tailwindcss from "@tailwindcss/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { sveltePhosphorOptimize } from "phosphor-svelte/vite";
import { defineConfig } from "vitest/config";

const codemirrorDeps = [
  "codemirror",
  "@codemirror/autocomplete",
  "@codemirror/commands",
  "@codemirror/lang-sql",
  "@codemirror/language",
  "@codemirror/lint",
  "@codemirror/search",
  "@codemirror/state",
  "@codemirror/view",
  "@lezer/common",
  "@lezer/highlight",
  "@lezer/lr",
];

export default defineConfig({
  plugins: [
    sveltePhosphorOptimize(),
    tailwindcss(),
    sveltekit(),
  ],
  resolve: {
    dedupe: codemirrorDeps,
  },
  optimizeDeps: {
    include: codemirrorDeps,
  },
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ["**/.cargo-target/**", "**/src-tauri/**", "**/target/**"],
    },
  },
  clearScreen: false,
  test: {
    include: ["src/**/*.{test,spec}.ts"],
    environment: "node",
  },
});
