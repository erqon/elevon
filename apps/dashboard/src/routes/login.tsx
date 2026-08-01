import { createFileRoute, redirect } from "@tanstack/react-router";
import { useState } from "react";
import { Button } from "@/components/ui/button.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card.tsx";
import { Input } from "@/components/ui/input.tsx";
import { Label } from "@/components/ui/label.tsx";
import { safeRedirectPath } from "@/lib/utils.ts";
import { userQueryOptions } from "@/queries/auth.ts";

export const Route = createFileRoute("/login")({
  head: () => ({
    meta: [{ title: "Login - Aileron Dashboard" }],
  }),
  validateSearch: (search: Record<string, unknown>): { redirect?: string } => {
    if (typeof search.redirect === "string") {
      return { redirect: search.redirect };
    }
    return {};
  },
  beforeLoad: async ({ context, search }) => {
    const user = await context.queryClient.ensureQueryData(userQueryOptions);

    if (user) {
      throw redirect({ href: safeRedirectPath(search.redirect) });
    }
  },
  component: RouteComponent,
});

function RouteComponent() {
  const [key, setKey] = useState("");
  const canSubmit = key.trim().length > 0;

  return (
    <div className="flex min-h-svh items-center justify-center px-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle className="font-heading text-2xl">Sign in</CardTitle>
          <CardDescription>
            Enter the shared access key to open the console.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form
            method="post"
            className="flex flex-col gap-4"
            onSubmit={(event) => {
              if (!canSubmit) event.preventDefault();
            }}
          >
            <div className="flex flex-col gap-2">
              <Label htmlFor="key">Access key</Label>
              <Input
                id="key"
                name="key"
                type="password"
                placeholder="••••••••"
                autoComplete="off"
                required
                value={key}
                onValueChange={setKey}
              />
            </div>
            <Button type="submit" className="w-full" disabled={!canSubmit}>
              Sign in
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
