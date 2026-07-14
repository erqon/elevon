import { define } from "@/utils.ts";

export default define.layout(({ Component, url }) => {
  return (
    <div class="layout">
      <nav>
        <a href="/" class={url.pathname === "/" ? "active" : ""}>
          Home
        </a>
        <a href="/about">About</a>
      </nav>
      <main>
        <Component />
      </main>
      <footer>&copy; 2026</footer>
    </div>
  );
});
