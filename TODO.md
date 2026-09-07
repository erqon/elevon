# Elevon TODO List

Elevon features todo list

Markers for features before the release

- **M** - Must have
- **N** - Nice-to-Have
- **P** - Post / Defer

## Alpha release (14st Sep 2026)

### Elevon Agent

- [x] TLS termination with Let's Encrypt or user-provided certificates **(M) - (user provided at least)**
~~ - [ ] Streaming application logs **(M)** ~~
~~ - [ ] Metrics pipeline (CPU, mem, req latency, error rate) **(M)** ~~
~~ - [ ] Running infrastructure Docker services (Postgres, Redis, etc.) **(N)** ~~
- [x] Zero-downtime rollback
~~ - [ ] Deployment history and audit trail ~~
~~ - [ ] Multi-environment support (dev/stage/prod) ~~
~~ - [ ] Hooks system (pre/post deploy scripts) **(N)** ~~
- [ ] Fix/add install script **(N/P)**
- [ ] Fix/restrict file permissions, file access, users etc.
- [ ] Restrict db calls to API only. Move internall calls over UNIX socket instead of TCP (for both the proxy and the cli). 

### Elevon Deploy

- [x] Init command **(M)**
- [ ] Fix/add install script **(N/P)**
- [x] App runtime configuration **(M)**
- [x] Deploy rollback command **(M)**
~~ - [ ] Interactive rollback selector (choose previous release) **(N)** ~~
~~ - [ ] Release snapshot export/import (images + env + runtime config) **(M)** ~~
~~ - [ ] Dry-run mode (validate config and rollout plan without deploying) **(M)** ~~
- [ ] Deploy lock (allow only one deployment per app at a time) **(M)**
~~ - [ ] Multi-app support across multiple config files combined in a single config file **(N)** ~~
~~ - [ ] Deploy multiple replicas of the same app from one config (load balancing) **(N)** ~~
~~ - [ ] Health-check gated promotion (switch traffic only after new container is healthy) **(N)** ~~
~~ - [ ] Rollback-on-failure policy (auto rollback on health-check failure or deploy timeout) ~~
