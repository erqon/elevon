import { z } from "zod";

export const loginSchema = z.object({
  key: z.string().min(1),
});

export type LoginDto = z.infer<typeof loginSchema>;

export function parseLoginForm(data: FormData): LoginDto {
  return loginSchema.parse({
    key: String(data.get("key") ?? ""),
  });
}
