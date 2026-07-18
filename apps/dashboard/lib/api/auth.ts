import { http } from "@/lib/api/http.ts";
import type { LoginResponse, User } from "@/lib/api/types/auth.ts";
import type { AppContext } from "@/lib/types.ts";

export const authApi = {
  login: (ctx: AppContext, payload: { key: string }) => {
    return http.post<LoginResponse>(ctx, "/auth/login", payload);
  },
  me: (ctx: AppContext) => {
    return http.get<User>(ctx, "/auth/me");
  },
  logout: (ctx: AppContext) => {
    return http.post<void>(ctx, "/auth/logout");
  },
};
