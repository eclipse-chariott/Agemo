// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Set of helper functions for interacting with Chariott through the generated gRPC client.

use log::{info, warn};
use std::{thread, time::Duration};
use tonic::{transport::Channel, Code, Request, Status};

use proto::{
    pubsub,
    service_registry::v1::service_registry_client::ServiceRegistryClient,
    service_registry::v1::{RegisterRequest, ServiceMetadata},
};

type ChariottClient = ServiceRegistryClient<Channel>;

/// Object that contains the necessary information for identifying a specific service.
pub struct ServiceIdentifier {
    /// The namespace that a service is under in Chariott.
    pub namespace: String,
    /// The name of the service in Chariott.
    pub name: String,
    /// The version of the service.
    pub version: String,
}

/// Helper function for initiating the Chariott client connection. Retries on failure.
///
/// # Arguments
///
/// * `chariott_url` - The url for Chariott.
pub async fn connect_to_chariott_with_retry(
    chariott_url: &str,
) -> Result<ChariottClient, Box<dyn std::error::Error + Send + Sync>> {
    let mut client_opt: Option<ChariottClient> = None;
    let mut reason = String::new();

    while client_opt.is_none() {
        client_opt = match ServiceRegistryClient::connect(chariott_url.to_string()).await {
            Ok(client) => Some(client),
            Err(e) => {
                let status = Status::from_error(Box::new(e));
                if status.code() == Code::Unavailable {
                    reason = String::from("No chariott service found");
                } else {
                    reason = format!("Chariott request failed with '{status:?}'");
                };
                None
            }
        }
        .or_else(|| {
            let secs = 5;
            warn!("{reason}, retrying in {secs} seconds...");
            thread::sleep(Duration::from_secs(secs));
            None
        });
    }

    info!("Successfully connected to Chariott.");

    Ok(client_opt.unwrap())
}

/// Helper function that registers service with Chariott.
///
/// # Arguments
///
/// * `chariott_client` - The gRPC client for interacting with the Chariott service.
/// * `provider_endpoint` - The endpoint where the provider service hosts the gRPC server.
/// * `service_identifier` - Information needed for uniquely identifying the service in Chariott.
pub async fn register_with_chariott(
    chariott_client: &mut ChariottClient,
    provider_endpoint: &str,
    service_identifier: ServiceIdentifier,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let provider_url_str = format!("http://{provider_endpoint}");

    let service_metadata = ServiceMetadata {
        namespace: service_identifier.namespace,
        name: service_identifier.name,
        version: service_identifier.version,
        uri: provider_url_str.clone(),
        communication_kind: pubsub::v1::SCHEMA_KIND.to_string(),
        communication_reference: pubsub::v1::SCHEMA_REFERENCE.to_string(),
    };

    let register_request = Request::new(RegisterRequest {
        service: Some(service_metadata),
    });
    chariott_client
        .register(register_request)
        .await?
        .into_inner();

    Ok(())
}
