# Elevon TODO List

Elevon features todo list

## Elevon Agent

- [ ] TLS termination with Let's Encrypt or user-provided certificates
- [ ] Streaming application logs
- [ ] Metrics pipeline (CPU, mem, req latency, error rate)
- [ ] Running infrastructure Docker services (Postgres, Redis, etc.)
- [ ] Zero-downtime rollback
- [ ] Deployment history and audit trail
- [ ] Multi-environment support (dev/stage/prod)
- [ ] Hooks system (pre/post deploy scripts)

## Elevon Deploy

- [ ] App runtime configuration
- [ ] Deploy rollback command
- [ ] Interactive rollback selector (choose previous release)
- [ ] Release snapshot export/import (images + env + runtime config)
- [ ] Dry-run mode (validate config and rollout plan without deploying)
- [ ] Deploy lock (allow only one deployment per app at a time)
- [ ] Multi-app support across multiple config files combined in a single config file
- [ ] Deploy multiple replicas of the same app from one config (load balancing)
- [ ] Health-check gated promotion (switch traffic only after new container is healthy)
- [ ] Rollback-on-failure policy (auto rollback on health-check failure or deploy timeout)
