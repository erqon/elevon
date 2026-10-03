ALTER TABLE "deployment_runtime_options" DROP COLUMN "options_port";
-- #[toasty::breakpoint]
ALTER TABLE "deployments" DROP COLUMN "port";
-- #[toasty::breakpoint]
ALTER TABLE "deployments" ADD COLUMN "web_domain" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "deployments" ADD COLUMN "web_port" INTEGER;
-- #[toasty::breakpoint]
ALTER TABLE "deployments" ADD COLUMN "web" BOOLEAN;
-- #[toasty::breakpoint]
ALTER TABLE "apps" DROP COLUMN "domain";
-- #[toasty::breakpoint]
ALTER TABLE "apps" ADD COLUMN "role" TEXT NOT NULL DEFAULT 'web' CHECK ("role" IN ('web', 'worker'));
