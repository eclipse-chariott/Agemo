// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Simple publisher example showing the process fo creating and publishing to a dynamic topic
//! following the Pub Sub Service model.

use env_logger::{Builder, Target};
use log::LevelFilter;
use samples_proto::publisher::v1::publisher_callback_server::PublisherCallbackServer;
use samples_proto::sample_publisher::v1::sample_publisher_server::SamplePublisherServer;
use publisher_impl::PublisherImpl;
use samples_common::{
    load_config::{
        load_settings, CommunicationConstants, SimplePublisherServiceSettings, CONFIG_FILE,
        CONSTANTS_FILE,
    },
    publisher_helper::DynamicPublisher,
};
use tonic::transport::Server;

mod publisher_impl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Setup logging.
    Builder::new()
        .filter(None, LevelFilter::Info)
        .target(Target::Stdout)
        .init();

    // Load in settings for service.
    let settings = load_settings::<SimplePublisherServiceSettings>(CONFIG_FILE)?;
    let communication_consts = load_settings::<CommunicationConstants>(CONSTANTS_FILE)?;

    // Instantiate the gRPC publisher implementation.
    let addr = settings.publisher_authority.parse()?;
    let publisher: PublisherImpl = DynamicPublisher::new(
        settings.publisher_authority,
        settings.pub_sub_uri,
        communication_consts.grpc_kind,
    );

    // Grpc server for handling calls from clients.
    // Note the two services, the `PublisherCallbackServer` handles callbacks from the pub sub
    // service, the `SamplePublisherServer` fields requests from subscribers.
    Server::builder()
        .add_service(PublisherCallbackServer::new(publisher.clone()))
        .add_service(SamplePublisherServer::new(publisher))
        .serve(addr)
        .await?;

    Ok(())
}
