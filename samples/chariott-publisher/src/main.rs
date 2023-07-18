// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Chariott-enabled publisher example showing the process for creating and publishing to a dynamic
//! topic following the Pub Sub Service model. Registers with Chariott to be discoverable.

use std::{thread, time::Duration};

use chariott_provider::ChariottProvider;
use env_logger::{Builder, Target};
use log::{warn, LevelFilter};
use proto::{
    chariott_provider::provider_service_server::ProviderServiceServer,
    chariott_runtime::{
        intent_registration::Intent, intent_service_registration::ExecutionLocality,
    },
    publisher::v1::publisher_server::PublisherServer,
    pubsub,
};
use publisher_impl::{PublisherImpl, ENDPOINT};
use sample_chariott_connector::{
    chariott_client::ChariottClient,
    chariott_provider_client::{ChariottProviderClient, RegisterParams},
};
use samples_common::{constants, publisher_helper::DynamicPublisher};
use tonic::{transport::Server, Code, Status};
use url::Url;

mod chariott_provider;
mod publisher_impl;

/// Calls Chariott to get Pub Sub Service endpoint.
///
/// # Arguments
///
/// * `chariott_url` - The url for Chariott.
async fn get_pub_sub_url(chariott_url: &str) -> Result<Option<String>, Status> {
    // Check if the Pub Sub Service exists, and if so register with Chariott.
    let mut chariott_client = ChariottClient::new(chariott_url.to_string()).await?;

    let result = chariott_client
        .discover(constants::PUB_SUB_NAMESPACE)
        .await
        .or_else(|status| {
            if status.code() == Code::NotFound || status.code() == Code::Unavailable {
                Ok(None)
            } else {
                Err(status)
            }
        })?
        .and_then(|services| {
            for service in services {
                if service.schema_kind == pubsub::v1::SCHEMA_KIND
                    && service.schema_reference == pubsub::v1::SCHEMA_REFERENCE
                {
                    return Some(service.url);
                }
            }
            None
        });

    Ok(result)
}

/// Registers the publisher with Chariott.
///
/// This function registers with Chariott. It follows the register announce pattern to maintain
/// connection with Chariott.
///
/// # Arguments
///
/// * `chariott_url` - The url for Chariott.
/// * `provider_endpoint` - The endpoint for the provider service.
async fn initiate_chariott_provider(
    chariott_url: &str,
    provider_endpoint: &str,
) -> Result<ChariottProvider, Box<dyn std::error::Error + Send + Sync>> {
    let provider_url_str = format!("http://{}", provider_endpoint);

    let register_params: RegisterParams = RegisterParams {
        name: "sample.publisher".to_string(),
        namespace: "sdv.chariott.publisher".to_string(),
        version: "0.0.1".to_string(),
        intents: [Intent::Discover].to_vec(),
        provider_url: provider_url_str.clone(),
        chariott_url: chariott_url.to_string(),
        locality: ExecutionLocality::Local,
    };

    let mut chariott_provider_client = ChariottProviderClient { register_params };

    // Intitate provider registration and announce heartbeat.
    chariott_provider_client
        .register_and_announce_provider(5)
        .await?;

    let provider_url = Url::parse(&chariott_provider_client.get_provider_url()).unwrap();

    Ok(ChariottProvider::new(provider_url))
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

    // Wait for Pub Sub Service to register with Chariott.
    let mut pub_sub_service_url = None;

    while pub_sub_service_url.is_none() {
        let mut reason = String::new();

        pub_sub_service_url = get_pub_sub_url(&chariott_url)
            .await
            .transpose()
            .or_else(|| {
                reason = String::from("No pub sub service found");
                None
            })
            .and_then(|res| match res {
                Ok(val) => Some(val),
                Err(e) => {
                    if e.code() == Code::Unavailable {
                        reason = String::from("No chariott service found");
                    } else {
                        reason = format!("Chariott request failed with '{e:?}'");
                    }
                    None
                }
            })
            .or_else(|| {
                let secs = 5;
                warn!("{reason}, retrying in {secs} seconds...");
                thread::sleep(Duration::from_secs(secs));
                None
            });
    }

    // Instantiate the gRPC publisher implementation.
    let publisher: PublisherImpl = DynamicPublisher::new(pub_sub_service_url.unwrap());

    // Instantiate Chariott provider.
    let chariott_provider = initiate_chariott_provider(&chariott_url, ENDPOINT).await?;

    // Grpc server for handling calls from clients.
    Server::builder()
        .add_service(PublisherServer::new(publisher))
        .add_service(ProviderServiceServer::new(chariott_provider))
        .serve(addr)
        .await?;

    Ok(())
}
