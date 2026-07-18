import { deleteCookie } from "@std/http";
import { define } from "@/utils.ts";

export default define.middleware(async (ctx) => {
  if (!ctx.state.user) {
    deleteCookie(ctx.req.headers, "session");
    return ctx.redirect("/login");
  }

  return await ctx.next();
});
