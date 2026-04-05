import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [
    tailwindcss(),
    svelte({
      inspector: { showToggleButton: "always", toggleButtonPos: "bottom-right" },
    }),
  ],
  build: { outDir: "dist", emptyOutDir: true },
  server: { port: 5173, strictPort: true },
  clearScreen: false,
});
