// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Loads configuration from external files.

#![cfg(feature = "yaml")]

use config::{Config, File, FileFormat};
use serde_derive::Deserialize;

const CONFIG_FILE: &str = "target/debug/pub_sub_service_settings";

#[derive(Debug, Deserialize)]
/// Object containing configuration settings to run the Pub Sub service.
pub struct Settings {
    /// The IP address and port number that the Pub Sub service listens on for requests.
    pub pub_sub_authority: String,
    /// The URL of the messaging service used to facilitate publish and subscribe functionality.
    pub messaging_url: String,
    /// The URL that the Chariott service listens on for requests.
    pub chariott_url: Option<String>,
    /// The namespace of the Pub Sub service.
    pub namespace: Option<String>,
    /// The name of the Pub Sub service.
    pub name: Option<String>,
    /// The current version of the Pub Sub Service.
    pub version: Option<String>,
}

/// Load the settings.
pub fn load_settings() -> Settings {
    let config = Config::builder()
        .add_source(File::new(CONFIG_FILE, FileFormat::Yaml))
        .build()
        .unwrap();

    let mut settings: Settings = config.try_deserialize().unwrap();

    if settings.chariott_url.is_some() {
        // Get version of the service for Chariott registration.
        let version = option_env!("CARGO_PKG_VERSION")
            .expect("Expected version to be defined in 'CARGO_PKG_VERSION'.");
        settings.version = Some(version.to_string());

        // Throw error if name or namespace are not set as they are needed for Chariott registration.
        settings
            .namespace
            .as_ref()
            .expect("Namespace should be set in config if 'chariott_url' is set.");
        settings
            .name
            .as_ref()
            .expect("Name should be set in config if 'chariott_url' is set.");
    }

    settings
}
