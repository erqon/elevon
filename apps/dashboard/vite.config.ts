import path from "node:path";
import { fileURLToPath } from "node:url";
import deno from "@deno/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import viteReact from "@vitejs/plugin-react";
import { nitro } from "nitro/vite";
import { defineConfig, type PluginOption } from "vite";

// Deno workspace: npm pkgs live under <repo>/node_modules/.deno — Vite won't
// discover that from deno.json alone, so allow the monorepo root explicitly.
const configDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(configDir, "../..");

const tanstackVirtualIdWorkaround: PluginOption = {
  name: "tanstack-virtual-id-workaround",
  enforce: "pre" as const,
  resolveId(id: string) {
    if (id.includes("virtual:tanstack-start-")) {
      return `\x00${id}`;
    }
    return null;
  },
};

export default defineConfig({
  server: {
    port: 3001,
    fs: {
      allow: [repoRoot],
    },
  },
  resolve: {
    tsconfigPaths: true,
  },
  plugins: [
    tanstackVirtualIdWorkaround,
    tanstackStart(),
    nitro(),
    viteReact(),
    deno(),
    tailwindcss(),
  ],
});
