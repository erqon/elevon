import { createFileRoute } from "@tanstack/react-router";
import { logoutFn } from "@/actions/auth.functions.ts";
import { Button } from "@/components/ui/button.tsx";

export const Route = createFileRoute("/_authed/")({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <div className="mx-auto flex min-h-svh items-center justify-center px-4 py-8">
      <Button type="button" variant="outline" onClick={() => logoutFn()}>
        Logout
      </Button>
    </div>
  );
}
