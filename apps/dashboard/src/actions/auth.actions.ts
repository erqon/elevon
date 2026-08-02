import { createFormAction } from "@/lib/action.ts";
import { loginFn } from "@/actions/auth.functions.ts";

export const loginAction = createFormAction(loginFn);
