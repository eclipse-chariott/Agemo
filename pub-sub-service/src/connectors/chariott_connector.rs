// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Set of helper functions for interacting with Chariott through the generated gRPC client.

use log::{info, warn};
use std::{thread, time::Duration};
use tonic::{transport::Channel, Code, Request, Status};

use proto::{
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
/// * `chariott_uri` - The uri for Chariott.
/// * `retry_interval_secs` - The interval to wait before retrying the connection.
pub async fn connect_to_chariott_with_retry(
    chariott_uri: &str,
    retry_interval_secs: u64,
) -> Result<ChariottClient, Box<dyn std::error::Error + Send + Sync>> {
    let mut client_opt: Option<ChariottClient> = None;
    let mut reason = String::new();

    while client_opt.is_none() {
        client_opt = match ServiceRegistryClient::connect(chariott_uri.to_owned()).await {
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
            warn!("{reason}, retrying in {retry_interval_secs} seconds...");
            thread::sleep(Duration::from_secs(retry_interval_secs));
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
/// * `provider_authority` - The authority where the provider service hosts the gRPC server.
/// * `service_identifier` - Information needed for uniquely identifying the service in Chariott.
/// * `communication_kind` - The kind of communication used by this service.
/// * `communication_reference` - The reference API file used to generate the gRPC service.
pub async fn register_with_chariott(
    chariott_client: &mut ChariottClient,
    provider_authority: &str,
    service_identifier: ServiceIdentifier,
    communication_kind: &str,
    communication_reference: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let provider_uri_str = format!("http://{provider_authority}"); // Devskim: ignore DS137138

    let service_metadata = ServiceMetadata {
        namespace: service_identifier.namespace,
        name: service_identifier.name,
        version: service_identifier.version,
        uri: provider_uri_str.clone(),
        communication_kind: communication_kind.to_string(),
        communication_reference: communication_reference.to_string(),
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
