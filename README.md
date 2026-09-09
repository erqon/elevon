# Elevon

Elevon is an agent-driven deployment and container management system for remote
VMs. It lets you build, publish, deploy, route, and roll back containerized
applications without requiring SSH access to the host during normal operation.

Elevon is currently in beta development. The project is being shaped around a
small operational surface, explicit configuration, and reliable rollbacks.

## What It Is

Elevon has two main parts:

- **The CLI** (`elevon`) runs from a developer or build machine. It reads the
  deployment configuration, builds and pushes images, and sends deployment
  commands to an agent.
- **The agent** (`elevon-agent`) runs on the target VM. It owns the local state
  and database, manages Docker workloads, exposes the control API through a
  Unix socket, and runs the Pingora reverse proxy for application traffic.

The CLI and agent communicate through authenticated API paths; the agent's
internal services use Unix sockets rather than opening direct database
connections from the CLI or proxy.

## Goals

- Be simple to understand, install, and use.
- Make production deployments fast in both execution time and setup time.
- Give individuals and teams of all sizes a practical alternative to expensive
  managed deployment platforms.
- Keep the operational model lightweight so a deployment does not require a
  large platform or a dedicated operations team.
- Make the common deployment workflow predictable and approachable.
- Provide a dashboard soon for monitoring and managing agents from one place.
- Support horizontal scaling of agents and applications.

## Current Features

- YAML-based deployment configuration.
- Docker image build and registry push support.
- Application deployment and rollback commands.
- Multiple applications in one deployment configuration.
- Runtime environment variables and inherited environment values.
- TLS termination with user-provided certificates.
- Pingora-based reverse proxy routing.
- Agent and proxy systemd services.
- Unix-socket communication between agent components.
- Agent auth-key management.
- Retention of a configurable number of previous releases.

## CLI

The deployment CLI uses `.elevon/deploy.yml` by default:

```text
elevon init
elevon check
elevon build
elevon push
elevon deploy --app web
elevon rollback --app web
```

Use `elevon --help` and `elevon <command> --help` for the current options.

The agent provides the host-side lifecycle commands:

```text
sudo elevon-agent init
sudo elevon-agent install --enable
sudo elevon-agent key --help
sudo elevon-agent uninstall
```

The agent binary is installed system-wide by the release installer, normally at
`/usr/local/bin/elevon-agent`. The CLI is installed for the current user under
`~/.config/elevon/bin`, with a link at `~/.local/bin/elevon`.

## Configuration

A deployment configuration describes the application image, registry,
routing, build context, environment, and app-specific runtime settings:

```yaml
name: project-name
image: project/project-name

# Number of previous releases to keep for rollback.
keep_releases: 5

elevon:
  agent:
    url: $AGENT_URL
    key: $AGENT_KEY

registry:
  # server: registry-server-url  # Defaults to ghcr.io
  username: $GHCR_USERNAME
  password: $GHCR_PASSWORD

routing:
  domain: app.example.com
  port: 3000
  tls:
    cert: $TLS_CERT
    key: $TLS_KEY

build:
  path: ./dev
  dockerfile: ./Dockerfile.dev

env:
  vars:
    PORT: 3000
    API_URL: $BASE_API_URL
  inherit:
    - DATABASE_URL

apps:
  api:
    role: web
  worker:
    role: worker
    cmd: "deno run worker.ts"
    runtime:
      restart: unless-stopped
      cpu: 2
      memory: 512mb
      network: my-network
```

Use [`crates/elevon-deploy/templates/config.yml`](crates/elevon-deploy/templates/config.yml)
as the full template, including comments about defaults and environment
variable expansion.

## Installation

Release installers are available in [`scripts`](scripts):

```bash
# Agent: system-wide installation, requires root
sudo bash scripts/install-agent.sh

# CLI: current-user installation
bash scripts/install-cli.sh
```

The installers accept an optional version, for example `v0.1.0`, and use
`ELEVON_REPO` when installing from a fork or another repository.

The current release artifacts are built for Linux `x86_64`. Other platform
names are recognized by the archive resolver, but matching release artifacts
must exist before those platforms are usable.

## Development Notes

This is a Rust workspace. The main crates are:

- `elevon-agent`: host-side API, proxy, Docker orchestration, and lifecycle.
- `elevon-cli`: user-facing command-line entry point.
- `elevon-deploy`: deployment configuration and image operations.
- `elevon-fs`: agent paths, permissions, and service files.
- `elevon-http`: shared HTTP and authentication behavior.
- `elevon-contracts`: shared API and deployment types.

Useful checks during development:

```bash
cargo check --workspace
cargo test --workspace
bash -n scripts/install-agent.sh scripts/install-cli.sh scripts/resolve-release-archive.sh
```

Automated tests are not implemented yet. Until they are added, beta validation
also requires manual checks of installation, systemd startup and shutdown,
deployment, rollback, proxy routing, and uninstall behavior.

## Status and Scope

Elevon is not yet a general-purpose hosted control plane. It currently targets
single-VM agent deployments and keeps the agent's state local to that VM.

See [`TODO.md`](TODO.md) for deferred work, including deployment locking,
additional operational features, and test coverage.
