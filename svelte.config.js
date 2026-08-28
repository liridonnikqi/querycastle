import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  vitePlugin: {
     inspector: {
            toggleKeyCombo: 'alt-x',
            showToggleButton: 'always',
            toggleButtonPos: 'bottom-right',
        },
  },
  kit: {
    adapter: adapter({
      pages: "dist",
      assets: "dist",
      fallback: "index.html",
    })
 
  },
};
