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
use log::{error, info, warn, LevelFilter};
use pubsub_connector::PubSubConnector;
use tonic::transport::Server;
use topic_manager::TopicManager;

use proto::pubsub::v1::pub_sub_server::PubSubServer;

use crate::{
    connectors::chariott_connector::{self, ServiceIdentifiers},
    pubsub_connector::{MonitorMessage, TOPIC_DELETED_MSG},
};

pub mod connectors;
pub mod pubsub_connector;
pub mod pubsub_impl;
pub mod topic_manager;

/// Endpoint for the messaging broker.
const BROKER: &str = "mqtt://localhost:1883";
/// Endpoint for the Chariott service.
const CHARIOTT_ENDPOINT: &str = "http://0.0.0.0:50000";
/// Default endpoint for this service.
const SERVICE_ENDPOINT: &str = "[::1]:50051";
/// Name that this service registers under in Chariott.
const SERVICE_NAME: &str = "dynamic.pubsub";
/// Namespace that this service registers under in Chariott.
const SERVICE_NAMESPACE: &str = "sdv.pubsub";

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

    // If Chariott flag is used then connect to Chariott and register the service.
    if use_chariott {
        // Create service identifiers used to uniquely identify the service.
        let service_identifiers = ServiceIdentifiers {
            namespace: SERVICE_NAMESPACE.to_string(),
            name: SERVICE_NAME.to_string(),
            version: "0.0.1".to_string(),
        };

        // connect to and register with Chariott.
        let mut chariott_client =
            chariott_connector::connect_to_chariott_with_retry(CHARIOTT_ENDPOINT).await?;

        chariott_connector::register_with_chariott(
            &mut chariott_client,
            SERVICE_ENDPOINT,
            service_identifiers,
        )
        .await?;
    }

    // Grpc server for handling calls from clients.
    Server::builder()
        .add_service(PubSubServer::new(pubsub))
        .serve(addr)
        .await?;

    Ok(())
}
