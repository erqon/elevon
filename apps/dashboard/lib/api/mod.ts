import { authApi } from "@/lib/api/auth.ts";

export type { ApiErr, ApiErrorBody, ApiOk, ApiResult } from "@/lib/api/http.ts";

export const api = {
  auth: authApi,
};
