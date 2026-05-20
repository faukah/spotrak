import { defineConfig } from "astro/config";
import deno from "@deno/astro-adapter";
import svelte from "@astrojs/svelte";
import tailwindcss from "@tailwindcss/vite";

const repoRoot = new URL("../", import.meta.url).pathname;

export default defineConfig({
  integrations: [svelte()],
  prefetch: {
    prefetchAll: false,
    defaultStrategy: "hover",
  },
  output: "server",
  adapter: deno({ start: false, hostname: "0.0.0.0", port: 4322 }),
  vite: {
    plugins: [tailwindcss()],
    server: {
      fs: {
        allow: [repoRoot],
      },
    },
    optimizeDeps: {
      exclude: ["layerchart"],
    },
    ssr: {
      noExternal: ["@lucide/svelte", "layerchart"],
    },
  },
});
