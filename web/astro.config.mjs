import { defineConfig } from "astro/config";
import deno from "@deno/astro-adapter";
import svelte from "@astrojs/svelte";
import tailwindcss from "@tailwindcss/vite";

const repoRoot = new URL("../", import.meta.url).pathname;

export default defineConfig({
  integrations: [svelte()],
  output: "server",
  adapter: deno({ start: false, hostname: "0.0.0.0", port: 4322 }),
  vite: {
    plugins: [tailwindcss()],
    server: {
      fs: {
        allow: [repoRoot],
      },
    },
    ssr: {
      noExternal: ["@lucide/svelte"],
    },
  },
});
