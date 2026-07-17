import { http } from "@/lib/api/http.ts";

export const authApi = {
  login: (payload: { key: string }) => {
    return http.post<void>("/auth/login", payload);
  },
};
