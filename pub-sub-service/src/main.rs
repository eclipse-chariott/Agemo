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

use std::{str::FromStr, sync::mpsc};

use clap::Parser;
use env_logger::{Builder, Target};
use log::{error, info, warn, LevelFilter};
use pubsub_connector::PubSubConnector;
use tonic::transport::Server;
use topic_manager::TopicManager;

use proto::pubsub::v1::pub_sub_server::PubSubServer;

use crate::{
    connectors::chariott_connector::{self, ServiceIdentifier},
    load_config::{CmdConfigOptions, CommunicationConstants},
    pubsub_connector::MonitorMessage,
};

pub mod connectors;
pub mod load_config;
pub mod pubsub_connector;
pub mod pubsub_impl;
pub mod topic_manager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load command line arguments if any.
    let parsed_args = CmdConfigOptions::parse();

    // Get log level. Defaults to info.
    let log_level =
        LevelFilter::from_str(&parsed_args.log_level).expect("Could not parse log level");

    // Setup logging.
    Builder::new()
        .filter(None, log_level)
        .target(Target::Stdout)
        .init();

    // Load settings in from config file.
    let settings = load_config::load_settings(parsed_args)?;
    let communication_consts = load_config::load_constants::<CommunicationConstants>()?;

    // Initialize pub sub service
    let topic_manager = TopicManager::new();
    let broker_uri = settings.messaging_uri.clone();
    let broker_protocol = communication_consts.mqtt_v5_kind.clone();

    info!("Setting up deletion channel...");
    let (deletion_sender, deletion_receiver) = mpsc::channel::<MonitorMessage>();

    info!("Getting sender from monitor...");
    let connector_sender = topic_manager.monitor(deletion_sender.clone()).await;

    let addr = settings.pub_sub_authority.parse()?;
    let pubsub = pubsub_impl::PubSubImpl {
        active_topics: topic_manager.get_active_topics_handle(),
        uri: broker_uri,
        protocol: broker_protocol,
    };

    // Local variable to pass to the broker monitor client.
    let topic_deletion_message = communication_consts.topic_deletion_message.clone();

    // Interface with messaging broker to monitor and clean up topics in a separate thread.
    let _monitor_handle = tokio::spawn(async move {
        let client_id = "pubsub_connector_client".to_string();

        // This line will need to be changed if a different broker is used to utilize the correct connector.
        let mut connector: connectors::mosquitto_connector::MqttFiveBrokerConnector =
            PubSubConnector::new(client_id, settings.messaging_uri);

        let _connection_res = connector.monitor_topics(connector_sender).await;
        loop {
            let delete_msg = deletion_receiver.recv();

            match delete_msg {
                Ok(msg) => {
                    let _res = connector
                        .delete_topic(msg.context, topic_deletion_message.clone())
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

    // If Chariott is enabled then connect to Chariott and register the service.
    if settings.chariott_uri.is_some() {
        // Create service identifiers used to uniquely identify the service.
        let service_identifier = ServiceIdentifier {
            namespace: settings.namespace.unwrap(),
            name: settings.name.unwrap(),
            version: settings.version.unwrap(),
        };

        // Connect to and register with Chariott.
        let mut chariott_client = chariott_connector::connect_to_chariott_with_retry(
            &settings.chariott_uri.unwrap(),
            communication_consts.retry_interval_secs,
        )
        .await?;

        chariott_connector::register_with_chariott(
            &mut chariott_client,
            &settings.pub_sub_authority,
            service_identifier,
            &communication_consts.grpc_kind,
            &communication_consts.pub_sub_reference,
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
