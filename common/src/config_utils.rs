// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use std::{
    env, io,
    path::{Path, PathBuf},
};

use config::{Config, File, FileFormat, FileStoredFormat, Source};
use home::home_dir;
use include_dir::Dir;
use serde::Deserialize;

pub const FILE_SEPARATOR: &str = ".";

/// Attempts to convert an extension in str format into a FileFormat enum.
/// Throws an error if the extension is unknown.
///
/// # Arguments
/// * `ext` - extension str to convert.
fn try_into_format(ext: &str) -> Result<FileFormat, Box<dyn std::error::Error + Send + Sync>> {
    match ext {
        ext if FileFormat::Ini.file_extensions().contains(&ext) => Ok(FileFormat::Ini),
        ext if FileFormat::Json.file_extensions().contains(&ext) => Ok(FileFormat::Json),
        ext if FileFormat::Json5.file_extensions().contains(&ext) => Ok(FileFormat::Json5),
        ext if FileFormat::Ron.file_extensions().contains(&ext) => Ok(FileFormat::Ron),
        ext if FileFormat::Toml.file_extensions().contains(&ext) => Ok(FileFormat::Toml),
        ext if FileFormat::Yaml.file_extensions().contains(&ext) => Ok(FileFormat::Yaml),
        _ => Err(Box::new(io::Error::new(
            io::ErrorKind::NotFound,
            "No Supported format found.",
        ))),
    }
}

/// Service's home directory metadata.
pub struct SvcConfigHomeMetadata {
    /// Name of the environment variable used to set the service's HOME dir.
    pub home_env_var: String,
    /// Default name for the service's HOME dir, used if `home_env_var` is not set.
    pub home_dir: String,
    /// Name of the config directory where configuration files should live.
    pub config_dir: String,
}

/// Metadata for a config file.
pub struct ConfigFileMetadata {
    /// Config file name with extension.
    pub name: String,
    /// File extension.
    pub ext: FileFormat,
}

impl ConfigFileMetadata {
    /// Create a new instance of ConfigFileMetadata.
    /// Will result in an error if provided file does not have a valid extension.
    ///
    /// # Arguments
    /// * `file_name` - Name of the file including the extension.
    pub fn new(file_name: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let name = file_name.to_string();
        let mut split_name: Vec<&str> = file_name.split(FILE_SEPARATOR).collect();

        if split_name.len() <= 1 {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid file name format. Expected an extension in name '{file_name}'."),
            )));
        }

        let parsed_ext = split_name.pop().unwrap();
        let ext = try_into_format(parsed_ext)?;

        Ok(ConfigFileMetadata { name, ext })
    }
}

/// Loads default config source for the given configuration file.
/// Extracts configuration parameters from the provided directory object which pulled in default
/// configuration files in at build time.
///
/// # Arguments
/// * `config_file` - The default config file to load.
/// * `default_dir` - Object that represents directory to pull default config file from. Generated
///                   by the `include_dir!` macro.
pub fn load_default_config_from_file(
    config_file: &ConfigFileMetadata,
    default_dir: &Dir,
) -> Result<Box<dyn Source + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
    // Get appropriate default config.
    let file = default_dir.get_file(&config_file.name).unwrap();
    let file_contents = file.contents_utf8().unwrap();

    Ok(File::from_str(file_contents, config_file.ext).clone_into_box())
}

/// Retrieve configuration home path from the service's HOME dir environment variable.
/// Attempts to construct a path from the provided service HOME env var. If the given env var is
/// not set, it defaults a path under the $HOME directory.
///
/// # Arguments
/// * `svc_home_metadata` - Metadata related to the service's home and config directories.
pub fn get_config_home_path_from_env(
    svc_home_metadata: &SvcConfigHomeMetadata,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let config_path = match env::var(&svc_home_metadata.home_env_var) {
        Ok(agemo_home) => {
            // The path below resolves to $SVC_HOME/{config_dir_name}/
            Path::new(&agemo_home).join(&svc_home_metadata.config_dir)
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
                .join(&svc_home_metadata.home_dir)
                .join(&svc_home_metadata.config_dir)
        }
    };

    Ok(config_path)
}

/// Read config from a configuration file located at the given config path at runtime.
///
/// # Arguments
/// * `config_file` - The config file to read.
/// * `config_path` - The path to the directory containing the config.
pub fn read_from_file<TPath>(
    config_file: &ConfigFileMetadata,
    config_path: TPath,
) -> Result<Box<dyn Source + Send + Sync>, Box<dyn std::error::Error + Send + Sync>>
where
    TPath: AsRef<Path>,
{
    let config_file_path = config_path.as_ref().join(&config_file.name);

    Ok(File::from(config_file_path)
        .required(false)
        .clone_into_box())
}

/// Builds unified config from provided configuration sources.
///
/// # Arguments
/// * `sources` - List of sources to build configuration from. Sources towards the end of the list
///               take higher precedence over sources at the beginning of the list.
pub fn build_config_from_sources<TConfig>(
    sources: Vec<Box<dyn Source + Send + Sync>>,
) -> Result<TConfig, Box<dyn std::error::Error + Send + Sync>>
where
    TConfig: for<'a> Deserialize<'a>,
{
    Config::builder()
        .add_source(sources)
        .build()?
        .try_deserialize()
        .map_err(|e| e.into())
}

/// Load a unified configuration given a file and commandline arguments.
/// Config will be compiled in the following order, with values from sources near the end of the
/// list taking higher precedence:
///
/// - default config file
/// - config file
/// - commandline args
///
/// Since the configuration is layered, config can be partially defined. Any unspecified
/// configuration will use the default value from the default config source.
///
/// # Arguments
/// * `config_file` - The config file to load configuration from.
/// * `default_config_file` - The default config file to load default config from.
/// * `default_dir` - Object that represents directory to pull default config file from. Generated
///                   by the `include_dir!` macro.
/// * `svc_home_metadata` - Metadata related to the service's home and config directories. Used to
///                         get path to provided config file.
/// * `cmdline_args` - Optional commandline config arguments.
pub fn load_config<TConfig, TArgs>(
    config_file: &ConfigFileMetadata,
    default_config_file: &ConfigFileMetadata,
    default_dir: &Dir,
    svc_home_metadata: &SvcConfigHomeMetadata,
    cmdline_args: Option<TArgs>,
) -> Result<TConfig, Box<dyn std::error::Error + Send + Sync>>
where
    TConfig: for<'de> serde::Deserialize<'de>,
    TArgs: Source + Send + Sync,
{
    // Load default configuration for the given configuration file.
    let default_source = load_default_config_from_file(default_config_file, default_dir)?;

    // Get configuration path from environment variable.
    let config_path = get_config_home_path_from_env(svc_home_metadata)?;

    // Read configuration file for any overrides.
    let file_source = read_from_file(config_file, config_path)?;

    // Create source list from lowest to highest priority.
    let mut sources = vec![default_source, file_source];

    // If commandline args are present, add them to the source list.
    if let Some(args) = cmdline_args {
        sources.push(args.clone_into_box());
    }

    // Build config from source list.
    build_config_from_sources(sources)
}
