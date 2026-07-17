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
  path: string,
  method: string,
  options?: RequestOptions,
): Promise<ApiResult<T>> {
  const url = `${API_BASE_URL}${path}`;
  const headers = new Headers(options?.headers);

  let body: BodyInit | null = null;
  if (options?.body !== undefined) {
    headers.set("Content-Type", "application/json");
    body = JSON.stringify(options.body);
  }

  let response: Response;
  try {
    response = await fetch(url, {
      method,
      headers,
      body,
      credentials: "include",
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
  get: <T>(path: string, headers?: Record<string, string>) =>
    request<T>(path, "GET", { headers }),

  post: <T>(path: string, body?: unknown, headers?: Record<string, string>) =>
    request<T>(path, "POST", { body, headers }),

  put: <T>(path: string, body?: unknown, headers?: Record<string, string>) =>
    request<T>(path, "PUT", { body, headers }),

  delete: <T>(path: string, headers?: Record<string, string>) =>
    request<T>(path, "DELETE", { headers }),
};
