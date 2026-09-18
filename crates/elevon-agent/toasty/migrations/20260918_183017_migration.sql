CREATE TABLE "auth_keys" (
    "id" BLOB NOT NULL,
    "name" TEXT NOT NULL,
    "key_hash" TEXT NOT NULL,
    "enabled" BOOLEAN NOT NULL,
    "last_used_at" TEXT,
    "expires_at" TEXT NOT NULL,
    "revoked_at" TEXT,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("id")
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_auth_keys_by_name" ON "auth_keys" ("name");
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_auth_keys_by_key_hash" ON "auth_keys" ("key_hash");
-- #[toasty::breakpoint]
CREATE TABLE "apps" (
    "id" BLOB NOT NULL,
    "name" VARCHAR(32) NOT NULL,
    "project" TEXT NOT NULL,
    "domain" TEXT,
    "keep_releases" INTEGER NOT NULL,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("id")
);
-- #[toasty::breakpoint]
CREATE INDEX "index_apps_by_name" ON "apps" ("name");
-- #[toasty::breakpoint]
CREATE INDEX "index_apps_by_project" ON "apps" ("project");
-- #[toasty::breakpoint]
CREATE TABLE "deployments" (
    "id" BLOB NOT NULL,
    "app_id" BLOB NOT NULL,
    "image_ref" TEXT,
    "container_id" TEXT,
    "port" INTEGER NOT NULL,
    "status" TEXT NOT NULL CHECK ("status" IN ('pending', 'active', 'drained', 'failed')),
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("id")
);
-- #[toasty::breakpoint]
CREATE INDEX "index_deployments_by_app_id" ON "deployments" ("app_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_deployments_by_image_ref" ON "deployments" ("image_ref");
-- #[toasty::breakpoint]
CREATE INDEX "index_deployments_by_container_id" ON "deployments" ("container_id");
-- #[toasty::breakpoint]
CREATE TABLE "deployment_runtime_options" (
    "id" BLOB NOT NULL,
    "deployment_id" BLOB NOT NULL,
    "options_role" TEXT NOT NULL,
    "options_port" INTEGER,
    "options_cmd" TEXT,
    "options_restart" TEXT,
    "options_memory_limit" BIGINT,
    "options_cpu_limit" BIGINT,
    "options_network" TEXT,
    PRIMARY KEY ("id")
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_deployment_runtime_options_by_deployment_id" ON "deployment_runtime_options" ("deployment_id");
