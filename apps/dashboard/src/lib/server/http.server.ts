import { getCookies } from "@tanstack/react-start/server";
import { ApiError } from "@/lib/api-error.ts";

const API_BASE_URL = Deno.env.get("API_URL");

async function request<T>(
  path: string,
  method: string,
  requestBody?: unknown,
): Promise<T> {
  const url = `${API_BASE_URL}${path}`;
  const headers = new Headers();
  const cookies = getCookies();

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
    throw new ApiError(
      0,
      "network",
      err instanceof Error ? err.message : "Network error",
    );
  }

  if (!response.ok) {
    const error = await response.json().catch(() => null);

    throw new ApiError(
      response.status,
      typeof error?.code === "string" ? error.code : "unknown",
      typeof error?.message === "string"
        ? error.message
        : `API Error: ${response.status}`,
    );
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

export const http = {
  get: <T>(path: string) => request<T>(path, "GET"),

  post: <T>(path: string, body?: unknown) => request<T>(path, "POST", body),

  put: <T>(path: string, body?: unknown) => request<T>(path, "PUT", body),

  delete: <T>(path: string) => request<T>(path, "DELETE"),
};
