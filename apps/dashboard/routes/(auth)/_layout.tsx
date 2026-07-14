import { define } from "@/utils.ts";

export default define.layout(({ Component }) => {
  return (
    <div class="flex min-h-screen items-center justify-center bg-background px-4">
      <Component />
    </div>
  );
});
