/**
 * Production entry for `deno run` / `deno compile`.
 *
 * Requires a prior `deno task build` (Nitro writes `.output/`).
 * Prefer `deno task compile` so public assets are embedded correctly.
 */
import "./.output/server/index.mjs";
