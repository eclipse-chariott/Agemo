// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Loads configuration from external files.

#![cfg(feature = "yaml")]

use config::{Config, File, FileFormat};
use serde_derive::{Deserialize, Serialize};

const CONFIG_FILE: &str = "target/debug/pub_sub_service_settings";
const CONSTANTS_FILE: &str = "target/debug/constants_settings";

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
    /// The reference API marker for a publisher service.
    pub publisher_reference: String,
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
pub fn load_settings() -> Settings {
    let config = Config::builder()
        .add_source(File::new(CONFIG_FILE, FileFormat::Yaml))
        .build()
        .unwrap();

    let mut settings: Settings = config.try_deserialize().unwrap();

    if settings.chariott_uri.is_some() {
        // Get version of the service for Chariott registration if not defined.
        if settings.version.is_none() {
            let version = env!(
                "CARGO_PKG_VERSION",
                "Expected version to be defined in env variable 'CARGO_PKG_VERSION'."
            );
            settings.version = Some(version.to_string());
        }

        // Throw error if name or namespace are not set as they are needed for Chariott registration.
        settings
            .namespace
            .as_ref()
            .expect("Namespace should be set in config if 'chariott_uri' is set.");
        settings
            .name
            .as_ref()
            .expect("Name should be set in config if 'chariott_uri' is set.");
    }

    settings
}

/// Load the constants.
///
/// Will attempt to load a configuration from the constants file to an object 'T' where 'T' is an
/// object representing a collection of constants. Exits program on failure.
pub fn load_constants<T>() -> T
where
    T: for<'de> serde::Deserialize<'de>,
{
    let config = Config::builder()
        .add_source(File::new(CONSTANTS_FILE, FileFormat::Yaml))
        .build()
        .unwrap();

    let settings: T = config.try_deserialize().unwrap();

    settings
}
