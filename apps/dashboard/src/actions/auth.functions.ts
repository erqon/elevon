import { redirect } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { clearSessionCookie } from "@/actions/auth.server.ts";
import { ApiError } from "@/lib/api-error.ts";
import { fetchUser, logout } from "@/lib/server/endpoints/mod.server.ts";

export const getCurrentUserFn = createServerFn({ method: "GET" }).handler(
  async () => {
    try {
      return await fetchUser();
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) return null;
      console.error("getCurrentUser failed:", err);
      return null;
    }
  },
);

export const logoutFn = createServerFn({ method: "POST" }).handler(async () => {
  try {
    await logout();
  } catch (err) {
    // Session already gone server-side - still clear the cookie locally.
    if (!(err instanceof ApiError && err.status === 401)) throw err;
  }

  clearSessionCookie();
  throw redirect({ to: "/login" });
});
