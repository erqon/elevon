import { tw } from "@/lib/utils.ts";

const focusRing = tw(
  "focus-visible:outline-none",
  "focus-visible:ring-2 focus-visible:ring-foreground",
  "focus-visible:ring-offset-2 focus-visible:ring-offset-background",
);

export const btn = {
  primary: tw(
    "inline-flex items-center justify-center gap-2",
    "rounded-none border border-foreground",
    "bg-foreground px-4 py-2.5",
    "text-sm font-semibold tracking-wide text-background",
    "transition-colors hover:bg-background hover:text-foreground",
    focusRing,
  ),
  secondary: tw(
    "inline-flex items-center justify-center gap-2",
    "rounded-none border border-border bg-background",
    "px-4 py-2.5 text-sm font-medium tracking-wide text-foreground",
    "transition-colors hover:border-foreground",
    focusRing,
  ),
  small: tw(
    "inline-flex items-center justify-center gap-1.5",
    "rounded-none border border-border bg-background",
    "px-3 py-1.5 text-xs font-medium tracking-wide text-foreground",
    "transition-colors hover:border-foreground hover:bg-muted",
  ),
};

export const input = tw(
  "w-full rounded-none border border-border bg-background",
  "px-3 py-2.5 text-sm text-foreground",
  "outline-none transition-colors",
  "placeholder:text-muted-foreground",
  "focus:border-foreground",
  "disabled:cursor-not-allowed disabled:opacity-50",
);
