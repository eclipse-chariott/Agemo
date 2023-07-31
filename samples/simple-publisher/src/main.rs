// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Simple publisher example showing the process fo creating and publishing to a dynamic topic
//! following the Pub Sub Service model.

use env_logger::{Builder, Target};
use log::LevelFilter;
use proto::publisher::v1::publisher_server::PublisherServer;
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging.
    Builder::new()
        .filter(None, LevelFilter::Info)
        .target(Target::Stdout)
        .init();

    // Load in settings for service.
    let settings = load_settings::<SimplePublisherServiceSettings>(CONFIG_FILE);
    let communication_consts = load_settings::<CommunicationConstants>(CONSTANTS_FILE);

    // Instantiate the gRPC publisher implementation.
    let addr = settings.publisher_authority.parse()?;
    let publisher: PublisherImpl = DynamicPublisher::new(
        settings.publisher_authority,
        settings.pub_sub_uri,
        communication_consts.grpc_kind,
    );

    // Grpc server for handling calls from clients.
    Server::builder()
        .add_service(PublisherServer::new(publisher))
        .serve(addr)
        .await?;

    Ok(())
}
