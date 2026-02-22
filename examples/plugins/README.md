# Plugin Smoke Test Example

This folder contains `smoke-test.yaml`, a ready-to-use plugin for exercising:

- Custom slash commands
- Plugin-defined agent tools
- Permission policies (`host_only`, `dual`, `sandbox_only`)

## Install

1. Create your plugins directory (default):

```sh
mkdir -p ~/.microclaw/plugins
```

2. Copy the example manifest:

```sh
cp examples/plugins/smoke-test.yaml ~/.microclaw/plugins/
```

If you use a custom plugin directory, set it in `microclaw.config.yaml`:

```yaml
plugins:
  enabled: true
  dir: "/absolute/path/to/plugins"
```

## Slash command checks

Run in any chat/channel:

- `/plugin-ping`
- `/plugin-host`
- `/plugin-dual`
- `/plugin-sandbox`
- `/plugin-echo hello world`

## Admin checks (control chat only)

- `/plugins list`
- `/plugins validate`
- `/plugins reload`

## Tool checks (ask the agent)

- "Call `plugin_smoke_echo` with text `hello`"
- "Call `plugin_smoke_dual_time`"
- "Call `plugin_smoke_sandbox_id`"
