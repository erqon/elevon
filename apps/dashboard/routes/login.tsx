import { define } from "@/utils.ts";
import { btn, input } from "@/components/ui.tsx";

export default define.page(() => {
  return (
    <div class="flex min-h-screen items-center justify-center bg-background px-4">
      <div class="w-full max-w-sm">
        <div class="rounded-none border border-border bg-card p-6">
          <h2 class="text-sm font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            Sign in
          </h2>
          <p class="mt-2 text-sm text-muted-foreground">
            Enter the shared access key to open the console.
          </p>
          <form class="mt-5 space-y-4">
            <div>
              <label
                htmlFor="key"
                class="mb-1.5 block text-xs font-medium uppercase tracking-wider text-muted-foreground"
              >
                Access key
              </label>
              <input
                id="key"
                name="key"
                type="password"
                autoFocus
                autoComplete="off"
                placeholder="••••••••"
                class={input}
              />
            </div>

            <button type="submit" class={`${btn.primary} w-full`}>
              Sign in
            </button>
          </form>
        </div>
      </div>
    </div>
  );
});
