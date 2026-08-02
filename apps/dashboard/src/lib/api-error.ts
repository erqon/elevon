export type ApiError = {
  status: number;
  code: string;
  message: string;
};

export type ApiResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: ApiError };
