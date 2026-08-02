import type { ApiResult } from "@/lib/api-error.ts";

export type ActionStatus = "idle" | "pending" | "failed" | "success";

export type ActionState<T = null> = {
  status: ActionStatus;
  data?: T | Promise<T>;
  errorTitle?: string | null;
  errorMessage?: string | null;
};

export const actionOk = <T = null>(data?: T): ActionState<T> => ({
  status: "success",
  ...(data !== undefined ? { data } : {}),
});

export const actionFail = <T = null>(
  errorMessage: string,
  errorTitle?: string | null,
): ActionState<T> => ({
  status: "failed",
  errorMessage,
  errorTitle,
});

export function fromApiResult<T>(result: ApiResult<T>): ActionState<T> {
  if (!result.ok) {
    return actionFail(result.error.message, result.error.code);
  }
  return actionOk(result.data);
}

type FormServerFn<TResult extends ActionState = ActionState> = (opts: {
  data: FormData;
}) => Promise<TResult>;

type Options<TResult extends ActionState> = {
  onSuccess?: (
    result: TResult,
    formData: FormData,
  ) => TResult | Promise<TResult>;
  onError?: (error: unknown, formData: FormData) => ActionState;
};

export function createFormAction<TResult extends ActionState = ActionState>(
  serverFn: FormServerFn<TResult>,
  options?: Options<TResult>,
) {
  return async (_: ActionState, formData: FormData): Promise<ActionState> => {
    try {
      const result = await serverFn({ data: formData });
      return options?.onSuccess
        ? await options.onSuccess(result, formData)
        : result;
    } catch (error) {
      return (
        options?.onError?.(error, formData) ?? {
          status: "failed",
          errorMessage: "Something went wrong",
        }
      );
    }
  };
}
