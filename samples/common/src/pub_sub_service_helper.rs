// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Collection of methods and enums to help with connection to the Pub Sub Service.

use log::error;
use serde_json::{json, Value};
use strum_macros::{Display, EnumString};

use samples_proto::{
    pubsub::v1::{
        pub_sub_client::PubSubClient, CreateTopicRequest, DeleteTopicRequest, DeleteTopicResponse,
    },
    sample_publisher::v1::SubscriptionInfoResponse,
};
use tonic::{Request, Response, Status};

/// Actions that are returned from the Pub Sub Service.
#[derive(Clone, EnumString, Display, Debug, PartialEq)]
pub enum TopicAction {
    /// Enum for the intitial state of a topic.
    #[strum(serialize = "INIT")]
    Init,
    /// Enum correlating to a START action from the Pub Sub Service.
    #[strum(serialize = "START")]
    Start,
    /// Enum correlating to a STOP action from the Pub Sub Service.
    #[strum(serialize = "STOP")]
    Stop,
    /// Enum correlating to a DELETE action from the Pub Sub Service.
    #[strum(serialize = "DELETE")]
    Delete,
}

/// Handles creation request to Pub Sub Service.
///
/// # Arguments
///
/// * `pub_sub_uri` - URI of the Pub Sub Service. (ex. "http://\[::1\]:50051")
/// * `client_id` - The client id of the service calling the method.
/// * `management_authority` - The management authority of the service calling the method.
/// * `management_protocol` - The protocol used by the given management callback.
pub async fn create_topic(
    pub_sub_uri: String,
    client_id: String,
    management_authority: String,
    management_protocol: String,
) -> Result<SubscriptionInfoResponse, Status> {
    let mut ps_client = PubSubClient::connect(pub_sub_uri).await.map_err(|e| {
        error!("Error connecting to pub sub wrapper client: {e:?}");
        Status::from_error(Box::new(e))
    })?;

    let request = Request::new(CreateTopicRequest {
        publisher_id: client_id,
        management_callback: format!("http://{management_authority}"), // Devskim: ignore DS137138
        management_protocol,
    });

    // Add returned information to the topic maps.
    let topic_info = ps_client.create_topic(request).await?.into_inner();

    let generated_topic = topic_info.generated_topic;
    let subscription_metadata = json!({ "topic": generated_topic }).to_string();

    let topic_subscription_info = SubscriptionInfoResponse {
        protocol_kind: topic_info.broker_protocol,
        subscription_uri: topic_info.broker_uri,
        subscription_metadata,
    };

    Ok(topic_subscription_info)
}

/// Handles deletion request to Pub Sub Service.
///
/// # Arguments
///
/// * `pub_sub_uri` - URI of the Pub Sub Service. (ex. "http://\[::1\]:50051")
/// * `topic` - The generated topic returned from the `create_topic` method call.
pub async fn delete_topic(
    pub_sub_uri: String,
    topic: String,
) -> Result<Response<DeleteTopicResponse>, Status> {
    // Call Pub Sub Service and get the topic and subscription information.
    let mut ps_client = PubSubClient::connect(pub_sub_uri).await.map_err(|e| {
        error!("Error connecting to pub sub wrapper client: {e:?}");
        Status::from_error(Box::new(e))
    })?;

    let request = Request::new(DeleteTopicRequest { topic });

    ps_client.delete_topic(request).await
}

// Get the generated topic name from the Subscription Response.
pub fn get_topic_from_subscription_response(sub_response: &SubscriptionInfoResponse) -> String {
    serde_json::from_str::<Value>(&sub_response.subscription_metadata).unwrap()["topic"]
        .as_str()
        .unwrap()
        .to_string()
}
