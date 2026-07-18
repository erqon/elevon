import { getCookies } from "@std/http";
import type { AppContext } from "@/lib/types.ts";

export interface ApiErrorBody {
  code: string;
  message: string;
}

export interface ApiOk<T> {
  ok: true;
  status: number;
  data: T;
}

export interface ApiErr {
  ok: false;
  status: number;
  error: ApiErrorBody;
}

export type ApiResult<T> = ApiOk<T> | ApiErr;

const API_BASE_URL = Deno.env.get("API_URL");

async function request<T>(
  ctx: AppContext,
  path: string,
  method: string,
  requestBody?: unknown,
): Promise<ApiResult<T>> {
  const url = `${API_BASE_URL}${path}`;
  const headers = new Headers(ctx.req.headers);
  const cookies = getCookies(headers);

  let body: BodyInit | null = null;
  if (requestBody !== undefined) {
    headers.set("Content-Type", "application/json");
    body = JSON.stringify(requestBody);
  }

  if (cookies.session) {
    headers.set("Authorization", `Bearer ${cookies.session}`);
  }

  let response: Response;
  try {
    response = await fetch(url, {
      method,
      headers,
      body,
    });
  } catch (err) {
    return {
      ok: false,
      status: 0,
      error: {
        code: "network",
        message: err instanceof Error ? err.message : "Network error",
      },
    };
  }

  if (!response.ok) {
    const error = await response.json().catch(
      (): ApiErrorBody => ({
        code: "unknown",
        message: `API Error: ${response.status}`,
      }),
    );

    return {
      ok: false,
      status: response.status,
      error: {
        code: typeof error.code === "string" ? error.code : "unknown",
        message:
          typeof error.message === "string"
            ? error.message
            : `API Error: ${response.status}`,
      },
    };
  }

  if (response.status === 204) {
    return { ok: true, status: 204, data: undefined as T };
  }

  return {
    ok: true,
    status: response.status,
    data: (await response.json()) as T,
  };
}

export const http = {
  get: <T>(ctx: AppContext, path: string) => request<T>(ctx, path, "GET"),

  post: <T>(ctx: AppContext, path: string, body?: unknown) =>
    request<T>(ctx, path, "POST", { body }),

  put: <T>(ctx: AppContext, path: string, body?: unknown) =>
    request<T>(ctx, path, "PUT", { body }),

  delete: <T>(ctx: AppContext, path: string) => request<T>(ctx, path, "DELETE"),
};
