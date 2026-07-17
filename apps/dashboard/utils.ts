import { createDefine } from "fresh";
import type { User } from "@/lib/api/types/auth.ts";

export interface State {
  user: User;
}

export const define = createDefine<State>();
