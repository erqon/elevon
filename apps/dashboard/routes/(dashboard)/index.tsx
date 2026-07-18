import { btn } from "@/components/ui.tsx";
import { define } from "@/utils.ts";

export default define.page(() => {
  return (
    <div class="px-4 py-8 mx-auto min-h-screen">
      <div class="max-w-3xl mx-auto flex flex-col items-center justify-center">
        <form action="/logout" method="post">
          <button type="submit" class={btn.primary}>
            Logout
          </button>
        </form>
      </div>
    </div>
  );
});
