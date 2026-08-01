import { deleteCookie } from "@tanstack/react-start/server";

export const clearSessionCookie = () => {
  deleteCookie("session", { path: "/" });
};
