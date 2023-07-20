// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Chariott-enabled publisher example showing the process for creating and publishing to a dynamic
//! topic following the Pub Sub Service model. Registers with Chariott to be discoverable.

use env_logger::{Builder, Target};
use log::LevelFilter;
use proto::{
    publisher::{self, v1::publisher_server::PublisherServer},
    pubsub,
    service_registry::v1::{RegisterRequest, ServiceMetadata},
};
use publisher_impl::{PublisherImpl, ENDPOINT};
use samples_common::{
    chariott_helper::{self, ChariottClient},
    constants,
    publisher_helper::DynamicPublisher,
};
use tonic::{transport::Server, Request, Status};

mod publisher_impl;

/// Helper function that registers service with Chariott.
///
/// # Arguments
///
/// * `chariott_client` - The gRPC client for interacting with the Chariott service.
/// * `provider_endpoint` - The endpoint where the provider service hosts the gRPC server.
async fn register_with_chariott(
    chariott_client: &mut ChariottClient,
    provider_endpoint: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let provider_url_str = format!("http://{provider_endpoint}");

    let service_metadata = ServiceMetadata {
        namespace: "sdv.chariott.publisher".to_string(),
        name: "sample.publisher".to_string(),
        version: "0.0.1".to_string(),
        uri: provider_url_str.clone(),
        communication_kind: publisher::v1::SCHEMA_KIND.to_string(),
        communication_reference: publisher::v1::SCHEMA_REFERENCE.to_string(),
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

/// Calls Chariott to get Pub Sub Service endpoint.
///
/// # Arguments
///
/// * `chariott_url` - The Chariott url.
/// * `namespace` - The namespace used to get pub sub information about.
async fn get_pub_sub_url_with_retry(
    chariott_client: &mut ChariottClient,
    retry_interval_secs: u64,
) -> Result<String, Status> {
    let service = chariott_helper::get_service_metadata_with_retry(
        chariott_client,
        constants::PUB_SUB_NAMESPACE,
        retry_interval_secs,
        pubsub::v1::SCHEMA_KIND,
        pubsub::v1::SCHEMA_REFERENCE,
    )
    .await?;

    Ok(service.uri)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Setup logging.
    Builder::new()
        .filter(None, LevelFilter::Info)
        .target(Target::Stdout)
        .init();

    let addr = ENDPOINT.parse()?;
    let chariott_url = constants::CHARIOTT_ENDPOINT.to_string();
    let retry_interval_secs = 5;

    // Attempt to connect with Chariott.
    let mut chariott_client =
        chariott_helper::connect_to_chariott_with_retry(&chariott_url, retry_interval_secs).await?;

    // Wait for Pub Sub Service to register with Chariott.
    let pub_sub_service_url =
        get_pub_sub_url_with_retry(&mut chariott_client, retry_interval_secs).await?;

    // Instantiate the gRPC publisher implementation.
    let publisher: PublisherImpl = DynamicPublisher::new(pub_sub_service_url);

    // Register with Chariott.
    register_with_chariott(&mut chariott_client, ENDPOINT).await?;

    // Grpc server for handling calls from clients.
    Server::builder()
        .add_service(PublisherServer::new(publisher))
        .serve(addr)
        .await?;

    Ok(())
}
