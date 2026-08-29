import tailwindcss from "@tailwindcss/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [
    tailwindcss(),
    sveltekit(),
  ],
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
