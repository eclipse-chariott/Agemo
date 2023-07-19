// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Wrapper service for a publish/subscribe messaging broker.
//!
//! This service provides publish/subscribe functionality as a provider for
//! [Eclipse Chariott](https://github.com/eclipse-chariott/chariott). It allows for pluggability
//! of a messaging broker (default is [Mosquitto MQTT broker](https://mosquitto.org/)), utilizing
//! the broker's publish/subscribe functionality. In addition, the service allows for dynamic topic
//! management, giving a publisher full control over the lifetime of the topic channel.

// Tells cargo to warn if a doc comment is missing and should be provided.
#![warn(missing_docs)]

use std::{env, sync::mpsc};

use env_logger::{Builder, Target};
use log::{error, info, LevelFilter};
use pubsub_connector::PubSubConnector;
use tonic::transport::Server;
use topic_manager::TopicManager;

use proto::{
    chariott_provider::provider_service_server::ProviderServiceServer,
    chariott_runtime::{
        intent_registration::Intent, intent_service_registration::ExecutionLocality,
    },
    pubsub::v1::pub_sub_server::PubSubServer,
};
use url::Url;

use crate::{
    connectors::chariott::{
        chariott_provider::ChariottProvider,
        chariott_provider_client::{ChariottProviderClient, RegisterParams},
    },
    pubsub_connector::{MonitorMessage, TOPIC_DELETED_MSG},
};

pub mod connectors;
pub mod pubsub_connector;
pub mod pubsub_impl;
pub mod topic_manager;

/// Endpoint for the messaging broker.
const BROKER: &str = "mqtt://localhost:1883";
/// Endpoint for the Chariott service.
const CHARIOTT_ENDPOINT: &str = "http://0.0.0.0:4243";
/// Default endpoint for this service.
const SERVICE_ENDPOINT: &str = "[::1]:50051";
/// Name that this service registers under in Chariott.
const SERVICE_NAME: &str = "dynamic.pubsub";
/// Namespace that this service registers under in Chariott.
const SERVICE_NAMESPACE: &str = "sdv.pubsub";

/// Helper function that registers service with Chariott and returns a provider for service
/// discovery through Chariott.
///
/// # Arguments
///
/// * `chariott_url` - The url of the Chariott service.
/// * `provider_endpoint` - The endpoint where the provider service hosts the gRPC server.
async fn initiate_chariott_provider(
    chariott_url: &str,
    provider_endpoint: &str,
) -> Result<ChariottProvider, Box<dyn std::error::Error + Send + Sync>> {
    let provider_url_str = format!("http://{provider_endpoint}");

    let register_params: RegisterParams = RegisterParams {
        name: SERVICE_NAME.to_string(),
        namespace: SERVICE_NAMESPACE.to_string(),
        version: "0.0.1".to_string(),
        intents: [Intent::Discover].to_vec(),
        provider_url: provider_url_str.clone(),
        chariott_url: chariott_url.to_string(),
        locality: ExecutionLocality::Local,
    };

    let mut chariott_client = ChariottProviderClient { register_params };

    // Intitate provider registration and announce heartbeat.
    chariott_client.register_and_announce_provider(5).await?;

    let provider_url = Url::parse(&chariott_client.get_provider_url()).unwrap();

    Ok(ChariottProvider::new(provider_url))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Setup logging.
    Builder::new()
        .filter(None, LevelFilter::Info)
        .target(Target::Stdout)
        .init();

    let mut use_chariott = false;
    let args: Vec<String> = env::args().collect();

    // Check if the Chariott flag is used to determine if the service needs to register.
    for arg in args {
        if arg.eq("--chariott") {
            use_chariott = true;
        }
    }

    // Initialize pub sub service
    let topic_manager = TopicManager::new();
    let broker_endpoint = BROKER.to_string();
    let broker_protocol = "mqtt".to_string();

    info!("Setting up deletion channel...");
    let (deletion_sender, deletion_receiver) = mpsc::channel::<MonitorMessage>();

    info!("Getting sender from monitor...");
    let connector_sender = topic_manager.monitor(deletion_sender.clone()).await;

    let addr = "[::1]:50051".parse()?;
    let pubsub = pubsub_impl::PubSubImpl {
        active_topics: topic_manager.get_active_topics_handle(),
        endpoint: broker_endpoint,
        protocol: broker_protocol,
    };

    // Interface with messaging broker to monitor and clean up topics in a separate thread.
    let _monitor_handle = tokio::spawn(async move {
        let client_id = "pubsub_connector_client".to_string();

        // This line will need to be changed if a different broker is used to utilize the correct connector.
        let mut connector: connectors::mosquitto_connector::MqttFiveBrokerConnector =
            PubSubConnector::new(client_id, BROKER.to_string());

        let _connection_res = connector.monitor_topics(connector_sender).await;
        loop {
            let delete_msg = deletion_receiver.recv();

            match delete_msg {
                Ok(msg) => {
                    let _res = connector
                        .delete_topic(msg.context, TOPIC_DELETED_MSG.to_string())
                        .await;
                }
                Err(err) => {
                    error!("error from topic manager: {err}");
                    info!("no longer able to delete topics..");
                    break;
                }
            }
        }
    });

    // Instantiate chariott provider service.
    let mut chariott_provider_svc = None;

    // If chariott flag is used then create chariott provider.
    if use_chariott {
        let chariott_provider =
            initiate_chariott_provider(CHARIOTT_ENDPOINT, SERVICE_ENDPOINT).await?;
        chariott_provider_svc = Some(ProviderServiceServer::new(chariott_provider));
    }

    // Grpc server for handling calls from clients.
    Server::builder()
        .add_service(PubSubServer::new(pubsub))
        .add_optional_service(chariott_provider_svc)
        .serve(addr)
        .await?;

    Ok(())
}
