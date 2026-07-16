import { define } from "@/utils.ts";

export default define.page(({ Component }) => {
  return (
    <html lang="en" class="dark">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <meta name="color-scheme" content="dark" />
        <title>Aileron - Dashboard</title>
      </head>
      <body class="bg-background text-foreground">
        <Component />
      </body>
    </html>
  );
});
