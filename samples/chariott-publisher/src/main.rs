// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Chariott-enabled publisher example showing the process for creating and publishing to a dynamic
//! topic following the Pub Sub Service model. Registers with Chariott to be discoverable.

use env_logger::{Builder, Target};
use log::LevelFilter;
use publisher_impl::PublisherImpl;
use samples_common::{
    chariott_helper::{self, ChariottClient},
    load_config::{
        load_settings, ChariottPublisherServiceSettings, CommunicationConstants, ServiceIdentifier,
        CONFIG_FILE, CONSTANTS_FILE,
    },
    publisher_helper::DynamicPublisher,
};
use samples_proto::{
    publisher::v1::publisher_callback_server::PublisherCallbackServer,
    sample_publisher::v1::sample_publisher_server::SamplePublisherServer,
    service_registry::v1::{RegisterRequest, ServiceMetadata},
};
use tonic::{transport::Server, Request, Status};

mod publisher_impl;

/// Helper function that registers service with Chariott.
///
/// # Arguments
///
/// * `chariott_client` - The gRPC client for interacting with the Chariott service.
/// * `provider_authority` - The authority where the provider service hosts the gRPC server.
/// * `provider_identifier` - The identifiers that uniquely describe this service.
/// * `communication_kind` - The kind of communication used by this service.
/// * `communication_reference` - The reference API file used to generate the gRPC service.
async fn register_with_chariott(
    chariott_client: &mut ChariottClient,
    provider_authority: &str,
    provider_identifier: ServiceIdentifier,
    communication_kind: &str,
    communication_reference: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let provider_uri_str = format!("http://{provider_authority}"); // Devskim: ignore DS137138

    let service_metadata = ServiceMetadata {
        namespace: provider_identifier.namespace,
        name: provider_identifier.name,
        version: provider_identifier.version,
        uri: provider_uri_str,
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

/// Calls Chariott to get Pub Sub Service uri.
///
/// # Arguments
///
/// * `chariott_client` - The gRPC client for interacting with the Chariott service.
/// * `namespace` - The namespace where the Pub Sub service is expected to be registered.
/// * `retry_interval_secs` - The interval to wait before retrying the connection.
/// * `communication_kind` - The expected kind of communication.
/// * `communication_reference` - The expected reference API file.
async fn get_pub_sub_uri_with_retry(
    chariott_client: &mut ChariottClient,
    namespace: &str,
    retry_interval_secs: u64,
    communication_kind: &str,
    communication_reference: &str,
) -> Result<String, Status> {
    let service = chariott_helper::get_service_metadata_with_retry(
        chariott_client,
        namespace,
        retry_interval_secs,
        communication_kind,
        communication_reference,
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

    // Load in settings for service.
    let settings = load_settings::<ChariottPublisherServiceSettings>(CONFIG_FILE)?;
    let communication_consts = load_settings::<CommunicationConstants>(CONSTANTS_FILE)?;

    let addr = settings.publisher_authority.parse()?;

    // Attempt to connect with Chariott.
    let mut chariott_client = chariott_helper::connect_to_chariott_with_retry(
        &settings.chariott_uri,
        communication_consts.retry_interval_secs,
    )
    .await?;

    // Wait for Pub Sub Service to register with Chariott.
    let pub_sub_service_uri = get_pub_sub_uri_with_retry(
        &mut chariott_client,
        &settings.pub_sub_namespace,
        communication_consts.retry_interval_secs,
        &communication_consts.grpc_kind,
        &communication_consts.pub_sub_reference,
    )
    .await?;

    // Instantiate the gRPC publisher implementation.
    let publisher: PublisherImpl = DynamicPublisher::new(
        settings.publisher_authority.clone(),
        pub_sub_service_uri,
        communication_consts.grpc_kind.clone(),
    );

    // Register with Chariott.
    register_with_chariott(
        &mut chariott_client,
        &settings.publisher_authority,
        settings.publisher_identifier.clone(),
        &communication_consts.grpc_kind,
        &settings.publisher_reference,
    )
    .await?;

    // Grpc server for handling calls from clients.
    Server::builder()
        // Handles callbacks from the pub sub service.
        .add_service(PublisherCallbackServer::new(publisher.clone()))
        // Fields request from subscribers for subscription information.
        .add_service(SamplePublisherServer::new(publisher))
        .serve(addr)
        .await?;

    Ok(())
}
