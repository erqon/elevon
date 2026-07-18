import { deleteCookie } from "@std/http";
import { api } from "@/lib/api/mod.ts";
import { define } from "@/utils.ts";

export const handler = define.handlers({
  async POST(ctx) {
    const result = await api.auth.logout(ctx);

    if (!result.ok) {
      return ctx.redirect("/");
    }

    const headers = new Headers();
    deleteCookie(headers, "session");
    headers.set("Location", "/login");

    return new Response(null, {
      status: 303,
      headers,
    });
  },
});
