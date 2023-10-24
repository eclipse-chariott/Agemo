// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use std::{env, path::Path};

use config::File;
use home::home_dir;
use serde::Deserialize;

pub const YAML_EXT: &str = "yaml";

const CONFIG_DIR: &str = "config";
const DOT_AGEMO_DIR: &str = ".agemo";
const AGEMO_HOME: &str = "AGEMO_HOME";

/// Read config from layered configuration files.
/// Searches for `{config_file_name}.default.{config_file_ext}` as the base configuration in `$AGEMO_HOME`,
/// then searches for overrides named `{config_file_name}.{config_file_ext}` in the current directory and `$AGEMO_HOME`.
/// If `$AGEMO_HOME` is not set, it defaults to `$HOME/.agemo`.
///
/// # Arguments
/// - `config_file_name`: The config file name. This is used to construct the file names to search for.
/// - `config_file_ext`: The config file extension. This is used to construct the file names to search for.
pub fn read_from_files<T>(
    config_file_name: &str,
    config_file_ext: &str,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T: for<'a> Deserialize<'a>,
{
    let default_config_file = format!("{config_file_name}.default.{config_file_ext}");
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

    // The path below resolves to {config_path}/{default_config_file}.
    let default_config_file_path = config_path.join(default_config_file);

    // The path below resolves to {current_dir}/{overrides_file}.
    let current_dir_config_file_path = env::current_dir()?.join(overrides_file.clone());

    // The path below resolves to {config_path}/{overrides_file}
    let overrides_config_file_path = config_path.join(overrides_file);

    let config_store = config::Config::builder()
        .add_source(File::from(default_config_file_path))
        .add_source(File::from(current_dir_config_file_path).required(false))
        .add_source(File::from(overrides_config_file_path).required(false))
        .build()?;

    config_store.try_deserialize().map_err(|e| e.into())
}
