export type ActionStatus = "idle" | "pending" | "failed" | "success";

export type ActionState<T = null> = {
  status: ActionStatus;
  data?: T | Promise<T>;
  errorTitle?: string | null;
  errorMessage?: string | null;
};

type ServerFn<TResult extends ActionState = ActionState> = (args: {
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
  serverFn: ServerFn<TResult>,
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
