// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Loads configuration from external files.

use std::env;

use clap::Parser;
use common::config_utils::{self, ConfigFileMetadata, SvcConfigHomeMetadata};
use include_dir::{include_dir, Dir};
use log::{debug, error};
use proc_macros::ConfigSource;
use serde_derive::{Deserialize, Serialize};

// Config file stems
const CONFIG_FILE_STEM: &str = "pub_sub_service_settings";
const CONSTANTS_FILE_STEM: &str = "constants";

// Config file extensions
const YAML_EXT: &str = "yaml";

// Default config file marker
const DEFAULT: &str = "default";

// Config directory consts
const CONFIG_DIR: &str = "config";
const DOT_AGEMO_DIR: &str = ".agemo";
const AGEMO_HOME_ENV_VAR: &str = "AGEMO_HOME";

// Default directory struct
const DEFAULT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../config");

/// Object containing commandline config options for the Pub Sub service.
/// Non-optional fields must be passed in via the commandline and will override any values from
/// configuration files.
#[derive(Clone, Debug, Parser, Serialize, Deserialize, ConfigSource)]
#[command(author, about, long_about = None)]
pub struct CmdConfigOptions {
    /// The IP address and port number that the Pub Sub service listens on for requests.
    /// Required if not set in configuration files. (eg. "0.0.0.0:50051").
    #[arg(short, long)]
    pub pub_sub_authority: Option<String>,
    /// The URI of the messaging service used to facilitate publish and subscribe functionality.
    /// Required if not set in configuration files. (eg. "mqtt://0.0.0.0:1883").
    #[arg(short, long)]
    pub messaging_uri: Option<String>,
    /// The URI that the Chariott service listens on for requests. (eg. "http://0.0.0.0:50000").
    #[arg(short, long)]
    pub chariott_uri: Option<String>,
    /// The namespace of the Pub Sub service.
    #[arg(short = 's', long)]
    pub namespace: Option<String>,
    /// The name of the Pub Sub service.
    #[arg(short, long)]
    pub name: Option<String>,
    /// The current version of the Pub Sub Service.
    #[arg(short, long)]
    pub version: Option<String>,
    /// The log level of the program.
    #[arg(short, long, default_value = "info")]
    pub log_level: String,
}

/// Object that contains constants used for establishing connection between services.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommunicationConstants {
    /// The topic deletion message constant.
    pub topic_deletion_message: String,
    /// String constant for gRPC.
    pub grpc_kind: String,
    /// String constant for MQTT v5.
    pub mqtt_v5_kind: String,
    /// The reference API marker for the Pub Sub service.
    pub pub_sub_reference: String,
    /// Interval for attempting to retry finding a service.
    pub retry_interval_secs: u64,
}

/// Object containing configuration settings to run the Pub Sub service.
#[derive(Clone, Debug, Parser, Serialize, Deserialize)]
pub struct Settings {
    /// The IP address and port number that the Pub Sub service listens on for requests.
    pub pub_sub_authority: String,
    /// The URI of the messaging service used to facilitate publish and subscribe functionality.
    pub messaging_uri: String,
    /// The URI that the Chariott service listens on for requests.
    pub chariott_uri: Option<String>,
    /// The namespace of the Pub Sub service.
    pub namespace: Option<String>,
    /// The name of the Pub Sub service.
    pub name: Option<String>,
    /// The current version of the Pub Sub Service.
    pub version: Option<String>,
}

/// Load configuration given a file and commandline arguments.
///
/// # Arguments
/// * `config_file_name` - Name of the config file to load override settings from.
/// * `default_file_name` - Name of default config file to load settings from.
/// * `args` - Optional commandline config arguments.
pub fn load_config<T>(
    config_file_name: &str,
    default_file_name: &str,
    args: Option<CmdConfigOptions>,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let config_file = ConfigFileMetadata::new(config_file_name)?;
    let default_config_file = ConfigFileMetadata::new(default_file_name)?;

    let default_dir = DEFAULT_DIR;

    let svc_home_metadata = SvcConfigHomeMetadata {
        home_env_var: AGEMO_HOME_ENV_VAR.to_string(),
        home_dir: DOT_AGEMO_DIR.to_string(),
        config_dir: CONFIG_DIR.to_string(),
    };

    config_utils::load_config(
        &config_file,
        &default_config_file,
        &default_dir,
        &svc_home_metadata,
        args,
    )
}

/// Load the settings.
///
/// Will attempt to load the settings from the service configuration file. If the necessary config
/// is set will run in Chariott enabled mode, otherwise the service will run in standalone mode.
///
/// # Arguments
/// * `args` - Commandline config arguments.
pub fn load_settings(
    args: CmdConfigOptions,
) -> Result<Settings, Box<dyn std::error::Error + Send + Sync>> {
    let file_name = format!("{CONFIG_FILE_STEM}.{YAML_EXT}");
    let default_file_name = format!("{CONFIG_FILE_STEM}.{DEFAULT}.{YAML_EXT}");

    let mut settings: Settings = load_config(&file_name, &default_file_name, Some(args))
        .map_err(|e| {
            format!(
                "Failed to load required configuration settings due to error: {e}. See --help for more details."
            )
        })?;

    debug!("settings config: {:?}", settings);

    if settings.chariott_uri.is_some() {
        // Get version of the service for Chariott registration if not defined.
        if settings.version.is_none() {
            let version = env!(
                "CARGO_PKG_VERSION",
                "Expected version to be defined in env variable 'CARGO_PKG_VERSION'."
            );
            settings.version = Some(version.to_string());
        }

        // Error if name or namespace are not set as they are needed for Chariott registration.
        if settings.namespace.is_none() {
            error!("Namespace should be set in config if 'chariott_uri' is set.");
            return Err(Box::from("Namespace not set"));
        }

        if settings.name.is_none() {
            error!("Name should be set in config if 'chariott_uri' is set.");
            return Err(Box::from("Name not set"));
        }
    }

    Ok(settings)
}

/// Load the constants.
///
/// Will attempt to load a configuration from the constants file to an object 'T' where 'T' is an
/// object representing a collection of constants. Returns error on failure.
pub fn load_constants<T>() -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let file_name = format!("{CONSTANTS_FILE_STEM}.{YAML_EXT}");
    let default_file_name = format!("{CONSTANTS_FILE_STEM}.{DEFAULT}.{YAML_EXT}");

    load_config(&file_name, &default_file_name, None)
}
