import { define } from "@/utils.ts";

export default define.page(({ Component }) => {
  return (
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Cirque - Dashboard</title>
      </head>
      <body>
        <Component />
      </body>
    </html>
  );
});
