import { define } from "@/utils.ts";
import { getCookies } from "@std/http";

export default define.middleware(async (ctx) => {
  const cookies = getCookies(ctx.req.headers);
  
  
  return await ctx.next();
});
