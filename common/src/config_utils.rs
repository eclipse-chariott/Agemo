// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use std::{env, path::Path};

use config::{File, FileFormat, Source};
use home::home_dir;
use include_dir::{include_dir, Dir};
use serde::Deserialize;

pub const YAML_EXT: &str = "yaml";

const CONFIG_DIR: &str = "config";
const DEFAULT: &str = "default";
const DOT_AGEMO_DIR: &str = ".agemo";
const AGEMO_HOME: &str = "AGEMO_HOME";

const DEFAULT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../config");

/// Read config from layered configuration files.
/// Searches for `{config_file_name}.default.{config_file_ext}` as the base configuration,
/// then searches for overrides named `{config_file_name}.{config_file_ext}` in `$AGEMO_HOME`.
/// If `$AGEMO_HOME` is not set, it defaults to `$HOME/.agemo`.
///
/// # Arguments
/// * `config_file_name` - The config file name. This is used to construct the file names to search for.
/// * `config_file_ext` - The config file extension. This is used to construct the file names to search for.
/// * `args` - Optional commandline arguments. Any values set will override values gathered from config files.
pub fn read_from_files<T, A>(
    config_file_name: &str,
    config_file_ext: &str,
    args: Option<A>,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T: for<'a> Deserialize<'a>,
    A: Source + Send + Sync + 'static + Clone,
{
    // Get default config.
    let default_config_filename = format!("{config_file_name}.{DEFAULT}.{config_file_ext}");
    let default_config_file = DEFAULT_DIR.get_file(default_config_filename).unwrap();
    let default_config_contents_str = default_config_file.contents_utf8().unwrap();

    // Get override_files
    let overrides_file = format!("{config_file_name}.{config_file_ext}");

    let config_path = match env::var(AGEMO_HOME) {
        Ok(agemo_home) => {
            // The path below resolves to $AGEMO_HOME/config/
            Path::new(&agemo_home).join(CONFIG_DIR)
        }
        Err(_) => {
            // The path below resolves to $HOME/.agemo/config/
            home_dir()
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Could not retrieve home directory",
                    )
                })?
                .join(DOT_AGEMO_DIR)
                .join(CONFIG_DIR)
        }
    };

    // The path below resolves to {config_path}/{overrides_file}
    let overrides_config_file_path = config_path.join(overrides_file);

    let mut config_sources = config::Config::builder()
        .add_source(File::from_str(
            default_config_contents_str,
            FileFormat::Yaml,
        ))
        .add_source(File::from(overrides_config_file_path).required(false));

    // Adds command line arguments if there are any.
    if let Some(args) = args {
        config_sources = config_sources.add_source(args);
    }

    let config_store = config_sources.build()?;

    config_store.try_deserialize().map_err(|e| e.into())
}
