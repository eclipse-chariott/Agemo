// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use std::{
    env,
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
        _ => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
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
#[derive(Debug, PartialEq, Eq)]
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
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid file name format. Expected an extension in name '{file_name}'."),
            )));
        }

        let parsed_ext = split_name.pop().unwrap();
        let ext = try_into_format(parsed_ext)?;

        if split_name.join(FILE_SEPARATOR).is_empty() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Invalid file name format. File cannot have an empty file_stem '{file_name}'."
                ),
            )));
        }

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
    let file = default_dir
        .get_file(&config_file.name)
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Unable to find default file '{}'.", &config_file.name),
        ))?;

    let file_contents = file.contents_utf8().ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!(
            "Unable to parse default file '{}' contents.",
            &config_file.name
        ),
    ))?;

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
        Ok(svc_home) => {
            // The path below resolves to $SVC_HOME/{config_dir_name}/
            Path::new(&svc_home).join(&svc_home_metadata.config_dir)
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

#[cfg(test)]
mod pubsub_impl_tests {
    use config::Value;
    use include_dir::{DirEntry, File};

    use super::*;

    #[test]
    fn try_into_format_success() {
        // Ini format
        let ini_ext = "ini";
        let ini_format = try_into_format(ini_ext).unwrap();
        assert_eq!(ini_format, FileFormat::Ini);

        // Json format
        let json_ext = "json";
        let json_format = try_into_format(json_ext).unwrap();
        assert_eq!(json_format, FileFormat::Json);

        // Json5 format
        let json5_ext = "json5";
        let json5_format = try_into_format(json5_ext).unwrap();
        assert_eq!(json5_format, FileFormat::Json5);

        // Ron format
        let ron_ext = "ron";
        let ron_format = try_into_format(ron_ext).unwrap();
        assert_eq!(ron_format, FileFormat::Ron);

        // Toml format
        let toml_ext = "toml";
        let toml_format = try_into_format(toml_ext).unwrap();
        assert_eq!(toml_format, FileFormat::Toml);

        // Yaml format
        let yaml_ext = "yaml";
        let yaml_format = try_into_format(yaml_ext).unwrap();
        assert_eq!(yaml_format, FileFormat::Yaml);

        let yaml_ext_2 = "yml";
        let yaml_format_2 = try_into_format(yaml_ext_2).unwrap();
        assert_eq!(yaml_format_2, FileFormat::Yaml);
    }

    #[test]
    fn try_into_format_invalid_err() {
        let ext_1 = "invalid";
        let result_1 = try_into_format(ext_1);
        assert!(result_1.is_err());

        let ext_2 = "";
        let result_2 = try_into_format(ext_2);
        assert!(result_2.is_err());

        let ext_3 = "123@";
        let result_3 = try_into_format(ext_3);
        assert!(result_3.is_err());
    }

    #[test]
    fn new_config_metadata_from_file_name() {
        let expected_name = "test.yaml";
        let expected_metadata = ConfigFileMetadata {
            name: expected_name.to_string(),
            ext: FileFormat::Yaml,
        };

        let metadata = ConfigFileMetadata::new(expected_name).unwrap();
        assert_eq!(metadata, expected_metadata);

        let expected_name_2 = "test.default.json";
        let expected_metadata_2 = ConfigFileMetadata {
            name: expected_name_2.to_string(),
            ext: FileFormat::Json,
        };

        let metadata_2 = ConfigFileMetadata::new(expected_name_2).unwrap();
        assert_eq!(metadata_2, expected_metadata_2);
    }

    #[test]
    fn new_config_metadata_from_invalid_str_err() {
        let result = ConfigFileMetadata::new("no_extension");
        let err = result.err().unwrap().downcast::<std::io::Error>().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

        let result_2 = ConfigFileMetadata::new("");
        let err_2 = result_2
            .err()
            .unwrap()
            .downcast::<std::io::Error>()
            .unwrap();
        assert_eq!(err_2.kind(), std::io::ErrorKind::InvalidInput);

        let result_3 = ConfigFileMetadata::new(".yaml");
        let err_3 = result_3
            .err()
            .unwrap()
            .downcast::<std::io::Error>()
            .unwrap();
        assert_eq!(err_3.kind(), std::io::ErrorKind::InvalidInput);

        let result_4 = ConfigFileMetadata::new("test.bad_extension");
        let err_4 = result_4
            .err()
            .unwrap()
            .downcast::<std::io::Error>()
            .unwrap();
        assert_eq!(err_4.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn load_default_config_from_file_success() {
        let file_name = "config.yaml";
        let config_file = ConfigFileMetadata {
            name: file_name.to_string(),
            ext: FileFormat::Yaml,
        };

        // Expected property to be returned.
        let expected_property_name = "test";
        let expected_property_value = Value::new(None, config::ValueKind::I64(1));
        let expected_properties_list_len = 1;

        // u8 representation of the expected property in yaml: "test: 1".
        let contents: &[u8] = &[116, 101, 115, 116, 58, 32, 49];

        // Create directory object.
        let expected_file = File::new(file_name, contents);
        let entry = DirEntry::File(expected_file);
        let entries = &[entry];
        let dir = Dir::new("", entries);

        let source = load_default_config_from_file(&config_file, &dir).unwrap();
        let properties = source.collect().unwrap();

        assert_eq!(properties.len(), expected_properties_list_len);
        assert!(properties.contains_key(expected_property_name));
        assert_eq!(
            properties.get(expected_property_name).unwrap(),
            &expected_property_value
        );
    }

    #[test]
    fn load_default_config_from_file_non_existent() {
        let non_existent_file = ConfigFileMetadata {
            name: "non_existent.yaml".to_string(),
            ext: FileFormat::Yaml,
        };

        let dir = Dir::new("", &[]);

        let result = load_default_config_from_file(&non_existent_file, &dir);
        let err = result.err().unwrap().downcast::<std::io::Error>().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn load_default_config_from_file_malformed_contents() {
        let file_name = "config.yaml";
        let config_file = ConfigFileMetadata {
            name: file_name.to_string(),
            ext: FileFormat::Yaml,
        };

        // Malformed bytes.
        let contents: &[u8] = &[0, 159, 146, 150];

        // Create directory object.
        let expected_file = File::new(file_name, contents);
        let entry = DirEntry::File(expected_file);
        let entries = &[entry];
        let dir = Dir::new("", entries);

        let result = load_default_config_from_file(&config_file, &dir);
        let err = result.err().unwrap().downcast::<std::io::Error>().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn get_config_home_path_from_env_success() {
        let env_var_key = "TEST_ENV_VAR";
        let env_var_value = "test_dir";
        let svc_home_dir = ".svc";
        let config_dir = "config";

        let expected_path = Path::new(env_var_value).join(config_dir);

        // Set the test environment variable.
        env::set_var(env_var_key, env_var_value);

        let svc_home_metadata = SvcConfigHomeMetadata {
            home_env_var: env_var_key.to_string(),
            home_dir: svc_home_dir.to_string(),
            config_dir: config_dir.to_string(),
        };

        let path = get_config_home_path_from_env(&svc_home_metadata);

        // Unset the environment variable.
        env::remove_var(env_var_key);

        assert_eq!(path.unwrap(), expected_path);
    }

    #[test]
    fn get_config_home_path_from_env_no_svc_home() {
        let env_var_key = "TEST_ENV_VAR";
        let home_dir = home_dir();
        let svc_home_dir = ".svc";
        let config_dir = "config";

        // If the environment variable happens to be set, we want to unset it.
        if env::var(env_var_key).is_ok() {
            env::remove_var(env_var_key);
        }

        let expected_path = home_dir.unwrap().join(svc_home_dir).join(config_dir);

        let svc_home_metadata = SvcConfigHomeMetadata {
            home_env_var: env_var_key.to_string(),
            home_dir: svc_home_dir.to_string(),
            config_dir: config_dir.to_string(),
        };

        let path = get_config_home_path_from_env(&svc_home_metadata).unwrap();
        assert_eq!(path, expected_path);
    }
}
