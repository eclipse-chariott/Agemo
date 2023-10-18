// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Loads configuration from external files.

use std::env;

use config::{Config, File, FileFormat};
use log::error;
use serde_derive::{Deserialize, Serialize};

const CONFIG_FILE: &str = "./config/pub_sub_service_settings";
const CONSTANTS_FILE: &str = "./config/constants_settings";

/// If feature 'containerize' is set, will modify a localhost uri to point to container's localhost
/// DNS alias. Otherwise, returns the uri as a String.
///
/// # Arguments
/// * `uri` - The uri to potentially modify.
pub fn get_uri(uri: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "containerize")]
    let uri = {
        // Container env variable names.
        const HOST_GATEWAY_ENV_VAR: &str = "HOST_GATEWAY";
        const LOCALHOST_ALIAS_ENV_VAR: &str = "LOCALHOST_ALIAS";

        // Return an error if container env variables are not set.
        let host_gateway = env::var(HOST_GATEWAY_ENV_VAR)?;
        let localhost_alias = env::var(LOCALHOST_ALIAS_ENV_VAR)?; // DevSkim: ignore DS162092

        uri.replace(&localhost_alias, &host_gateway) // DevSkim: ignore DS162092
    };

    Ok(uri.to_string())
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// Load the settings.
///
/// Will attempt to load the settigns from the service configuration file. If the necessary config
/// is set will run in Chariott enabled mode, otherwise the service will run in standalone mode.
pub fn load_settings() -> Result<Settings, Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::builder()
        .add_source(File::new(CONFIG_FILE, FileFormat::Yaml))
        .build()
        .map_err(|error| {
            error!("Unable to load file `{CONFIG_FILE}`. Failed with error: {error}.");
            error
        })?;

    let mut settings: Settings = config.try_deserialize().map_err(|error| {
        error!("Deserialize settings from `{CONFIG_FILE}` failed with error: {error}.");
        error
    })?;

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
    let config = Config::builder()
        .add_source(File::new(CONSTANTS_FILE, FileFormat::Yaml))
        .build()
        .map_err(|error| {
            error!("Unable to load file `{CONSTANTS_FILE}`. Failed with error: {error}.");
            error
        })?;

    let settings: T = config.try_deserialize().map_err(|error| {
        error!("Deserialize settings from `{CONSTANTS_FILE}` failed with error: {error}.");
        error
    })?;

    Ok(settings)
}
