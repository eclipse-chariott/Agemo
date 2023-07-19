// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Chariott client for communication with the Chariott endpoint.
//!
//! Utilizes generated [ChariottServiceClient] code to make for easier method calling. Unwraps the
//! returned object from the gRPC calls and formats it in a readable way. Currently only Implements
//! the [Discover](DiscoverIntent) intent.

use proto::{
    chariott_common::{
        discover_fulfillment::Service, fulfillment::Fulfillment as FulfillmentEnum,
        intent::Intent as IntentEnum, DiscoverIntent, Intent as IntentMessage,
    },
    chariott_runtime::{chariott_service_client::ChariottServiceClient, FulfillRequest},
};

use tonic::{transport::Channel, Request, Status};

/// Handles the calling and interpretation of the Chariott gRPC calls.
pub struct ChariottClient {
    /// Generated client from [proto::chariott_runtime].
    pub client: ChariottServiceClient<Channel>,
}

impl ChariottClient {
    /// Instantiates a new client.
    ///
    /// # Arguments
    ///
    /// * `chariott_url` - The url for Chariott.
    pub async fn new(chariott_url: String) -> Result<Self, Status> {
        let client = ChariottServiceClient::connect(chariott_url)
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        Ok(ChariottClient { client })
    }

    /// Executes a discover call to Chariott. Then returns the list of services received from
    /// Chariott after unwrapping the return.
    ///
    /// # Arguments
    ///
    /// * `namespace` - the namespace to attempt to discover services for.
    pub async fn discover(&mut self, namespace: &str) -> Result<Option<Vec<Service>>, Status> {
        let request = Request::new(FulfillRequest {
            namespace: namespace.to_string(),
            intent: Some(IntentMessage {
                intent: Some(IntentEnum::Discover(DiscoverIntent {})),
            }),
        });

        // Get list of services at the requested namespace, if any.
        let services: Option<Vec<Service>> = self
            .client
            .fulfill(request)
            .await?
            .into_inner()
            .fulfillment
            .and_then(|fulfillment_message| fulfillment_message.fulfillment)
            .and_then(|fulfillment_enum| match fulfillment_enum {
                FulfillmentEnum::Discover(discover) => {
                    Some(discover.services.into_iter().collect())
                }
                _ => None,
            });

        Ok(services)
    }
}
