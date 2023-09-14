// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Collection of methods and enums to help with connection to the Pub Sub Service.

use std::{thread, time::Duration};

use log::{info, warn};
use samples_proto::service_registry::v1::{
    service_registry_client::ServiceRegistryClient, DiscoverByNamespaceRequest, ServiceMetadata,
};
use tonic::{transport::Channel, Code, Request, Status};

pub type ChariottClient = ServiceRegistryClient<Channel>;

/// Helper function for initiating the Chariott client connection. Retries on failure.
/// Returns a gRPC client.
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
        client_opt = match ServiceRegistryClient::connect(chariott_uri.to_string()).await {
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

/// Helper function for getting service metadata from Chariott. Retries on failure.
///
/// # Arguments
///
/// * `chariott_client` - The Chariott client.
/// * `namespace` - The namespace to attempt to get service information about.
/// * `retry_interval_secs` - The interval to wait before retrying the connection.
/// * `communication_kind` - The required kind of communication a service must have.
/// * `communication_reference` - The required reference file a service must have.
pub async fn get_service_metadata_with_retry(
    chariott_client: &mut ChariottClient,
    namespace: &str,
    retry_interval_secs: u64,
    communication_kind: &str,
    communication_reference: &str,
) -> Result<ServiceMetadata, Status> {
    // Check if the service exists, and if not, wait for service to register with Chariott.
    let mut service = None;

    while service.is_none() {
        let request = Request::new(DiscoverByNamespaceRequest {
            namespace: namespace.to_string(),
        });
        let mut reason =
            format!("No service found at namespace '{namespace}' that meets the requirements");

        service = match chariott_client.discover_by_namespace(request).await {
            Ok(response) => response
                .into_inner()
                .services
                .into_iter()
                .filter(|svc| {
                    svc.communication_kind == communication_kind
                        && svc.communication_reference == communication_reference
                })
                .collect::<Vec<ServiceMetadata>>()
                .first()
                .cloned(),
            Err(status) => {
                reason = format!("Chariott request failed with '{status:?}'");
                None
            }
        }
        .or_else(|| {
            warn!("{reason}, retrying in {retry_interval_secs} seconds...");
            thread::sleep(Duration::from_secs(retry_interval_secs));
            None
        });
    }

    Ok(service.unwrap())
}
