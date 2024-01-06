// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Loads configuration from external files.

use std::env;

use clap::Parser;
use common::config_utils;
use log::error;
use proc_macros::ConfigSource;
use serde_derive::{Deserialize, Serialize};

const CONFIG_FILE_NAME: &str = "pub_sub_service_settings";
const CONSTANTS_FILE_NAME: &str = "constants";

/// Object containing commandline config options for the Pub Sub service.
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

/// Load a configuration file.
///
/// # Arguments
/// * `config_file_name` - Name of the config file to load settings from.
pub fn load_config<T>(
    config_file_name: &str,
    args: Option<CmdConfigOptions>,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T: for<'de> serde::Deserialize<'de>,
{
    config_utils::read_from_files(config_file_name, config_utils::YAML_EXT, args)
}

/// Load the settings.
///
/// Will attempt to load the settings from the service configuration file. If the necessary config
/// is set will run in Chariott enabled mode, otherwise the service will run in standalone mode.
pub fn load_settings(
    args: CmdConfigOptions,
) -> Result<Settings, Box<dyn std::error::Error + Send + Sync>> {
    let mut settings: Settings = load_config(CONFIG_FILE_NAME, Some(args))
        .map_err(|e| {
            format!(
                "Failed to load required configuration settings with error: {e}. See --help for more details."
            )
        })?;

    println!("after config: {:?}", settings);

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
    load_config(CONSTANTS_FILE_NAME, None)
}
