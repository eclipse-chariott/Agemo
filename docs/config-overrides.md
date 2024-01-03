# Config Overrides

The pub sub service supports configuration overrides that enable users to override the default
settings and provide custom configuration. This is achieved with configuration layering. Default
configuration files are defined in `config` at the root of the project, and this is often
suitable for basic scenarios or getting started quickly. The service includes the default
configuration files at build time. The service relies on the environment variable `$AGEMO_HOME` to
find any override configuration files. This variable is set by default to point to
`{path_to_project_root}/.agemo` when running the service with `cargo run`. Template configuration
files to use to override can be found under [config\template](../config/template/). The default
configuration files can be overridden at runtime using custom values. When loading configuration,
the service will probe for and unify config in the following order, with values near the end of the
list taking higher precedence:

- `$AGEMO_HOME/config/{config_name}.yaml`. If you have not set a `$AGEMO_HOME` directory or are
not running the service with `cargo run`, this defaults to:
  - Unix: `$HOME/.agemo/config/{config_name}.yaml`
  - Windows: `%USERPROFILE%\.agemo\config\{config_name}.yaml` (note that Windows support is not
  guaranteed by Agemo)
- Command line arguments.

Because the config is layered, the overrides can be partially defined and only specify the
top-level configuration fields that should be overridden. Anything not specified in an override
file will use the default value.

Samples handles configuration in the same way, except it utilizes the `$AGEMO_SAMPLES_HOME` env
variable to point to the `.agemo-samples` directory at the project root.

## Command Line Arguments

The service leverages [clap (command line argument parser)](https://github.com/clap-rs/clap) to
override individual configuration values through command line arguments when starting the service.
To see the list of possible parameters, run:

```shell
cargo run -p pub-sub-service -- --help
```
