import { api } from "@/lib/api/mod.ts";
import { define } from "@/utils.ts";

export default define.middleware(async (ctx) => {
  const result = await api.auth.me(ctx);

  if (result.ok) {
    ctx.state.user = result.data;
  }

  return await ctx.next();
});
