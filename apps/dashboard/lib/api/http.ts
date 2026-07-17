import { getCookies } from "@std/http";
import type { Context } from "fresh";
import type { State } from "@/utils.ts";

type RequestOptions = {
  body?: unknown;
  headers?: Record<string, string>;
};

export type ApiErrorBody = {
  code: string;
  message: string;
};

export type ApiOk<T> = {
  ok: true;
  status: number;
  data: T;
};

export type ApiErr = {
  ok: false;
  status: number;
  error: ApiErrorBody;
};

export type ApiResult<T> = ApiOk<T> | ApiErr;

const API_BASE_URL = Deno.env.get("API_URL");

async function request<T>(
  ctx: Context<State>,
  path: string,
  method: string,
  options?: RequestOptions,
): Promise<ApiResult<T>> {
  const url = `${API_BASE_URL}${path}`;
  const headers = new Headers(options?.headers);
  const cookies = getCookies(ctx.req.headers);

  let body: BodyInit | null = null;
  if (options?.body !== undefined) {
    headers.set("Content-Type", "application/json");
    body = JSON.stringify(options.body);
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
  get: <T>(
    ctx: Context<State>,
    path: string,
    headers?: Record<string, string>,
  ) => request<T>(ctx, path, "GET", { headers }),

  post: <T>(
    ctx: Context<State>,
    path: string,
    body?: unknown,
    headers?: Record<string, string>,
  ) => request<T>(ctx, path, "POST", { body, headers }),

  put: <T>(
    ctx: Context<State>,
    path: string,
    body?: unknown,
    headers?: Record<string, string>,
  ) => request<T>(ctx, path, "PUT", { body, headers }),

  delete: <T>(
    ctx: Context<State>,
    path: string,
    headers?: Record<string, string>,
  ) => request<T>(ctx, path, "DELETE", { headers }),
};
