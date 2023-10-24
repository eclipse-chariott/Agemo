# Config Overrides

The pub sub service supports configuration overrides that enable users to override the default
settings and provide custom configuration. This is achieved with configuration layering. Default
configuration files are defined in `.agemo/config` at the root of the project, and this is often
suitable for basic scenarios or getting started quickly. The service relies on the environment
variable `$AGEMO_HOME` to find the default configuration files. This variable is set by default to
point to `{path_to_project_root}/.agemo` when running the service with `cargo run`. The default
configuration files can be overridden at runtime using custom values. When loading configuration,
the service will probe for and unify config in the following order, with values near the end of the
list taking higher precedence:

- The default config
- A config file in the working directory of the executable (for example, the directory you were in
when you ran the `cargo run` command)
- `$AGEMO_HOME/config/{config_name}.yaml`. If you have not set a `$AGEMO_HOME` directory or are
not running the service with `cargo run`, this defaults to:
  - Unix: `$HOME/.agemo/config/{config_name}.yaml`
  - Windows: `%USERPROFILE%\.agemo\config\{config_name}.yaml` (note that Windows support is not
  guaranteed by Agemo)

Because the config is layered, the overrides can be partially defined and only specify the
top-level configuration fields that should be overridden. Anything not specified in an override
file will use the default value.

Samples handles configuration in the same way, except it utilizes the `.agemo-samples` directory
and the `$AGEMO_SAMPLES_HOME` env variable to point to that directory.
