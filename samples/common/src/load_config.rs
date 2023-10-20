// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Loads configuration from external files.

use serde_derive::{Deserialize, Serialize};

use crate::config_utils;

pub const CONFIG_FILE: &str = "samples_settings";
pub const CONSTANTS_FILE: &str = "constants";

/// Object that contains the necessary information for identifying a specific service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceIdentifier {
    /// The namespace that a service is under in Chariott.
    pub namespace: String,
    /// The name of the service in Chariott.
    pub name: String,
    /// The version of the service.
    pub version: String,
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

/// Object that contains settings for instantiating a Chariott enabled publisher.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChariottPublisherServiceSettings {
    /// URI of the Chariott service.
    pub chariott_uri: String,
    /// Namespace where the Pub Sub service is expected to register.
    pub pub_sub_namespace: String,
    /// The IP address and port number that this Publisher listens on for requests.
    pub publisher_authority: String,
    /// Service identifier for this Publisher.
    pub publisher_identifier: ServiceIdentifier,
    /// The reference API marker for a publisher service.
    pub publisher_reference: String,
}

/// Object that contains settings for instantiating a Chariott enabled subscriber.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChariottSubscriberServiceSettings {
    /// URI of the Chariott service.
    pub chariott_uri: String,
    /// The default service to discover.
    pub publisher_identifier: ServiceIdentifier,
    /// The reference API marker for a publisher service.
    pub publisher_reference: String,
}

/// Object that contains settings for instantiating a simple publisher.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimplePublisherServiceSettings {
    /// The IP address and port number that this Publisher listens on for requests.
    pub publisher_authority: String,
    /// URI of the Pub Sub service.
    pub pub_sub_uri: String,
}

/// Object that contains settings for instantiating a simple subscriber.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleSubscriberServiceSettings {
    /// The IP address and port number that the Publisher listens on for requests.
    pub publisher_authority: String,
}

/// Load the settings.
///
/// Will attempt to load a configuration from the settings file to an object 'T'.
/// Returns error on failure.
///
/// # Arguments
///
/// * `config_file_path` - Path from root of repo to the configuration file. Includes file name.
pub fn load_settings<T>(
    config_file_path: &str,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T: for<'de> serde::Deserialize<'de>,
{
    config_utils::read_from_files(config_file_path, config_utils::YAML_EXT)
}
