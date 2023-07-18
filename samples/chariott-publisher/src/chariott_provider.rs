// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Implements [`proto::chariott_provider`], allowing other services to find the Pub Sub Service
//! through Chariott.
//!
//! Implements the Discover intent, which allows external services to find the gRPC server endpoint
//! for the Pub Sub Service. Then, services can directly communicate with the Pub Sub Service.

use std::collections::HashMap;

use async_trait::async_trait;
use tonic::{Request, Response, Status};

use url::Url;

use proto::{
    chariott_common::{
        discover_fulfillment::Service, fulfillment::Fulfillment as FulfillmentEnum,
        intent::Intent as IntentEnum, DiscoverFulfillment, Fulfillment as FulfillmentMessage,
    },
    chariott_provider::{
        provider_service_server::ProviderService, FulfillRequest, FulfillResponse,
    },
    publisher,
};

/// Serves gRPC requests from Chariott.
///
/// Implements [`proto::chariott_provider`] to enable service discovery of the provider through
/// Chariott.
pub struct ChariottProvider {
    url: Url,
}

impl ChariottProvider {
    /// Instantiates a new ChariottProvider.
    ///
    /// # Arguments
    ///
    /// * `url` - The provider service url to discover through Chariott.
    pub fn new(url: Url) -> Self {
        Self { url }
    }
}

#[async_trait]
impl ProviderService for ChariottProvider {
    async fn fulfill(
        &self,
        request: Request<FulfillRequest>,
    ) -> Result<Response<FulfillResponse>, Status> {
        // Fullfill request to get the intent that is to be fulfilled.
        let fulfillment = match request
            .into_inner()
            .intent
            .and_then(|i| i.intent)
            .ok_or_else(|| Status::invalid_argument("Intent must be specified."))?
        {
            // Construct information about the service that can be used to directly communicate
            // with the service.
            IntentEnum::Discover(_intent) => Ok(FulfillmentEnum::Discover(DiscoverFulfillment {
                services: vec![Service {
                    url: self.url.to_string(),
                    schema_kind: publisher::v1::SCHEMA_KIND.to_owned(),
                    schema_reference: publisher::v1::SCHEMA_REFERENCE.to_owned(),
                    metadata: HashMap::new(),
                }],
            })),
            _ => Err(Status::invalid_argument("Unsupported or unknown intent."))?,
        };

        // Create FulfillResponse to return back to Chariott.
        fulfillment.map(|f| {
            Response::new(FulfillResponse {
                fulfillment: Some(FulfillmentMessage {
                    fulfillment: Some(f),
                }),
            })
        })
    }
}
