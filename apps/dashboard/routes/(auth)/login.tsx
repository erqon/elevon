import { setCookie } from "@std/http";
import { btn, input } from "@/components/ui.tsx";
import { api } from "@/lib/api/mod.ts";
import { define } from "@/utils.ts";

export const handler = define.handlers({
  async POST(ctx) {
    const form = await ctx.req.formData();
    const key = form.get("key");

    if (!key) {
      return { data: { message: "Access key is required!" } };
    }

    const result = await api.auth.login(ctx, { key: key as string });

    if (!result.ok) {
      return { data: { message: result.error.message } };
    }

    const headers = new Headers();
    setCookie(headers, {
      name: "session",
      value: result.data.sessionToken,
      httpOnly: true,
      secure: false,
      sameSite: "Lax",
      path: "/",
      maxAge: 60 * 60 * 24,
    });
    headers.set("Location", "/");

    return new Response(null, {
      status: 303,
      headers,
    });
  },
});

export default define.page<typeof handler>(({ data }) => {
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
          <form method="post" class="mt-5 space-y-4">
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
                autoComplete="off"
                placeholder="••••••••"
                class={input}
                required
              />
              {data.message ? (
                <p
                  id="key-error"
                  role="alert"
                  class="mt-1.5 text-xs text-red-400"
                >
                  {data.message}
                </p>
              ) : null}
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
