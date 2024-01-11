// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use std::{
    collections::HashMap,
    env, io,
    path::{Path, PathBuf},
};

use config::{Config, File, FileFormat, FileStoredFormat, Map, Source};
use home::home_dir;
use include_dir::Dir;
use lazy_static::lazy_static;
use serde::Deserialize;

lazy_static! {
    /// Creates a map between the file extensions as a str and FileFormat enum.
    /// This is used to convert a string into a FileFormat enum.
    static ref FILE_EXTS: HashMap<FileFormat, &'static [&'static str]> = {
        let mut format_map = Map::<FileFormat, &'static [&'static str]>::default();

        format_map.insert(FileFormat::Ini, FileFormat::Ini.file_extensions());
        format_map.insert(FileFormat::Json, FileFormat::Json.file_extensions());
        format_map.insert(FileFormat::Json5, FileFormat::Json5.file_extensions());
        format_map.insert(FileFormat::Ron, FileFormat::Ron.file_extensions());
        format_map.insert(FileFormat::Toml, FileFormat::Toml.file_extensions());
        format_map.insert(FileFormat::Yaml, FileFormat::Yaml.file_extensions());

        format_map
    };
}

/// Attempts to convert an extension in str format into a FileFormat enum.
/// Throws an error if the extension is unknown.
///
/// # Arguments
/// * `ext` - extension str to convert.
fn try_into_format(ext: &str) -> Result<FileFormat, Box<dyn std::error::Error + Send + Sync>> {
    for (format, extensions) in FILE_EXTS.iter() {
        if extensions.contains(&ext) {
            return Ok(*format);
        }
    }

    Err(Box::new(io::Error::new(
        io::ErrorKind::NotFound,
        "No Supported format found.",
    )))
}

/// Loads default config for the given configuration file.
/// Extracts configuration parameters from the provided directory object.
///
/// # Arguments
/// * `config_file_stem` - The default config file name without an extension. This is used to
///                        construct the file name to search for.
/// * `config_file_ext` - The config file extension. This is used to construct the file name to
///                       search for.
/// * `default_dir` - Object that represents directory to pull default config file from. Generated
///                   by the `include_dir!` macro.
pub fn load_default_config_from_file(
    config_file_stem: &str,
    config_file_ext: &str,
    default_dir: Dir,
) -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    // Get appropriate default config.
    let filename = format!("{config_file_stem}.{config_file_ext}");
    let file = default_dir.get_file(filename).unwrap();
    let file_contents = file.contents_utf8().unwrap();
    let file_format = try_into_format(config_file_ext)?;

    Config::builder()
        .add_source(File::from_str(file_contents, file_format))
        .build()
        .map_err(|e| e.into())
}

/// Retrieve configuration home path from the service's HOME dir environment variable.
/// Attempts to construct a path from the provided service HOME env var. If the given env var is
/// not set, it defaults a path under the $HOME directory.
///
/// # Arguments
/// * `svc_home_env_var` - Name of the environment variable used to set the service's HOME dir.
/// * `svc_home_dir_name` - Default name for the service's HOME dir, used if `svc_home_env_var` is
///                         not set.
/// * `config_dir_name` - Name of the config directory where configuration files should live.
pub fn get_config_home_path_from_env(
    svc_home_env_var: &str,
    svc_home_dir_name: &str,
    config_dir_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let config_path = match env::var(svc_home_env_var) {
        Ok(agemo_home) => {
            // The path below resolves to $SVC_HOME/{config_dir_name}/
            Path::new(&agemo_home).join(config_dir_name)
        }
        Err(_) => {
            // The path below resolves to $HOME/{svc_home_dir_name}/{config_dir_name}/
            home_dir()
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Could not retrieve home directory",
                    )
                })?
                .join(svc_home_dir_name)
                .join(config_dir_name)
        }
    };

    Ok(config_path)
}

/// Read config from a configuration file located at the given config path.
///
/// # Arguments
/// * `config_file_stem` - The config file name without an extension. This is used to construct the
///                        file name to search for.
/// * `config_file_ext` - The config file extension. This is used to construct the file name to
///                       search for.
/// * `config_path` - The path to the directory containing the config.
pub fn read_from_file<TPath>(
    config_file_stem: &str,
    config_file_ext: &str,
    config_path: TPath,
) -> Result<Config, Box<dyn std::error::Error + Send + Sync>>
where
    TPath: AsRef<Path>,
{
    let config_file_name = format!("{config_file_stem}.{config_file_ext}");

    // The path below resolves to {config_path}/{config_file_name}
    let config_file_path = config_path.as_ref().join(config_file_name);

    Config::builder()
        .add_source(File::from(config_file_path).required(false))
        .build()
        .map_err(|e| e.into())
}

/// Builds unified config from provided configuration sources.
/// Config will be compiled in the following order, with values from sources near the end of the
/// list taking higher precedence:
///
/// - default source
/// - file source
/// - commandline source
///
/// Since the configuration is layered, config can be partially defined. Any unspecified
/// configuration will use the default value from the default config source.
///
/// # Arguments
/// * `default_source` - Config gathered from default configuration sources.
/// * `file_source` - Config read in from configuration files.
/// * `cmdline_source` - Optional config gathered from commandline parameters.
pub fn build_config<TConfig, TSource>(
    default_source: Config,
    file_source: Config,
    cmdline_source: Option<TSource>,
) -> Result<TConfig, Box<dyn std::error::Error + Send + Sync>>
where
    TConfig: for<'a> Deserialize<'a>,
    TSource: Source + Send + Sync + 'static,
{
    let mut config_sources = Config::builder()
        .add_source(default_source)
        .add_source(file_source);

    // Adds command line arguments if there are any.
    if let Some(cmdline_args) = cmdline_source {
        config_sources = config_sources.add_source(cmdline_args);
    };

    config_sources
        .build()?
        .try_deserialize()
        .map_err(|e| e.into())
}
