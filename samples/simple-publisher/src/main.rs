// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Simple publisher example showing the process fo creating and publishing to a dynamic topic
//! following the Pub Sub Service model.

use env_logger::{Builder, Target};
use log::LevelFilter;
use proto::publisher::v1::publisher_server::PublisherServer;
use publisher_impl::{PublisherImpl, ENDPOINT};
use samples_common::publisher_helper::DynamicPublisher;
use tonic::transport::Server;

mod publisher_impl;

/// Default Pub Sub Service url to be used by the publisher.
const PUBSUB: &str = "http://[::1]:50051";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging.
    Builder::new()
        .filter(None, LevelFilter::Info)
        .target(Target::Stdout)
        .init();

    // Instantiate the gRPC publisher implementation.
    let addr = ENDPOINT.parse()?;
    let publisher: PublisherImpl = DynamicPublisher::new(PUBSUB.to_string());

    // Grpc server for handling calls from clients.
    Server::builder()
        .add_service(PublisherServer::new(publisher))
        .serve(addr)
        .await?;

    Ok(())
}
