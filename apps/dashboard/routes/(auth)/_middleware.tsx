import { define } from "@/utils.ts";

export default define.middleware(async (ctx) => {
  if (ctx.state.user) {
    return ctx.redirect("/");
  }

  return await ctx.next();
});
