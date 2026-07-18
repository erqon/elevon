import { deleteCookie } from "@std/http";
import { define } from "@/utils.ts";

export default define.middleware(async (ctx) => {
  if (!ctx.state.user) {
    const headers = new Headers();
    deleteCookie(headers, "session");
    headers.set("Location", "/login");

    return new Response(null, {
      status: 303,
      headers,
    });
  }

  return await ctx.next();
});
