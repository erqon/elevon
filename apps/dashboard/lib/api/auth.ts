import type { Context } from "fresh";
import { http } from "@/lib/api/http.ts";
import type { LoginResponse, User } from "@/lib/api/types/auth.ts";
import type { State } from "@/utils.ts";

export const authApi = {
  login: (ctx: Context<State>, payload: { key: string }) => {
    return http.post<LoginResponse>(ctx, "/auth/login", payload);
  },
  me: (ctx: Context<State>) => {
    return http.get<User>(ctx, "/auth/me");
  },
  logout: (ctx: Context<State>) => {
    return http.post<void>(ctx, "/auth/logout");
  },
};
