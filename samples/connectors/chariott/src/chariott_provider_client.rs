// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Handles the registration of a service with Chariott.
//!
//! This client handles the registration and announce pattern used to register a service as a
//! provider in Chariott.

use std::time::Duration;

use log::{info, warn};
use proto::chariott_runtime::{
    chariott_service_client::ChariottServiceClient, intent_registration::Intent,
    intent_service_registration::ExecutionLocality, AnnounceRequest, IntentRegistration,
    IntentServiceRegistration, RegisterRequest, RegistrationState,
};

use tokio::time::sleep;
use tonic::{transport::Channel, Code, Status};

/// Structure that contains necessary context for registering a service with Chariott.
#[derive(Clone)]
pub struct RegisterParams {
    /// Name to register under in Chariott.
    pub name: String,
    /// Namespace to register under in Chariott.
    pub namespace: String,
    /// Version of the service.
    pub version: String,
    /// List of Chariott [`intents`][Intent] that the service supports.
    pub intents: Vec<Intent>,
    /// The url of the service that is registering with Chariott.
    pub provider_url: String,
    /// The url of the Chariott service.
    pub chariott_url: String,
    /// Whether the service is local or lives in the cloud.
    pub locality: ExecutionLocality,
}

/// Client that handles the registration and announce pattern to Chariott service.
pub struct ChariottProviderClient {
    /// Parameters used to register a service with Chariott as a provider.
    pub register_params: RegisterParams,
}

impl ChariottProviderClient {
    async fn connect_chariott_client(
        client: &mut Option<ChariottServiceClient<Channel>>,
        chariott_url: String,
    ) -> Result<(), Status> {
        *client = Some(
            ChariottServiceClient::connect(chariott_url)
                .await
                .map_err(|e| {
                    *client = None; // Set client back to None on error.
                    Status::from_error(Box::new(e))
                })?,
        );

        Ok(())
    }

    async fn register_and_announce_once(
        client: &mut Option<ChariottServiceClient<Channel>>,
        reg_params: RegisterParams,
    ) -> Result<(), Status> {
        // If there is no client, need to attempt connection.
        if client.is_none() {
            Self::connect_chariott_client(client, reg_params.chariott_url.clone()).await?;
        }

        let service = Some(IntentServiceRegistration {
            name: reg_params.name,
            url: reg_params.provider_url,
            version: reg_params.version,
            locality: reg_params.locality as i32,
        });

        let announce_req = AnnounceRequest {
            service: service.clone(),
        };

        // Always announce to Chariott.
        let registration_state = client
            .as_mut()
            .expect("No client found")
            .announce(announce_req.clone())
            .await?
            .into_inner()
            .registration_state;

        // Only attempt registration with Chariott if the announced state is 'ANNOUNCED'.
        // The 'ANNOUNCED' state means that this service is not currently registered in Chariott.
        // This also handles re-registration if Chariott crashes and comes back online.
        if registration_state == RegistrationState::Announced as i32 {
            let register_req = RegisterRequest {
                service: service.clone(),
                intents: reg_params
                    .intents
                    .iter()
                    .map(|i| IntentRegistration {
                        intent: *i as i32,
                        namespace: reg_params.namespace.clone(),
                    })
                    .collect(),
            };

            info!("Registered with Chariott runtime: {register_req:?}");

            let _client = client
                .as_mut()
                .expect("No client found")
                .register(register_req.clone())
                .await?;
        }

        Ok(())
    }

    /// Loop that continuously announces the service to Chariott and registers if the service is
    /// not present.
    ///
    /// The current pattern in Chariott requires a service that registers with Chariott to announce
    /// that the service is still alive at least every 15 seconds, otherwise the service will be
    /// dropped from Chariott.
    ///
    /// # Arguments
    ///
    /// * `ttl_seconds` - Interval that client announces to Chariott. Must be shorter interval than
    ///                   Chariott provider TTL, which defaults to 15 seconds.
    pub async fn register_and_announce_provider(
        &mut self,
        ttl_seconds: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let reg_params = self.register_params.clone();

        // Initiate registration and announce thread.
        let _handle = tokio::task::spawn(async move {
            let mut client = None;

            // Loop that handles provider registration and announce heartbeat pattern.
            loop {
                if let Err(e) =
                    Self::register_and_announce_once(&mut client, reg_params.clone()).await
                {
                    let reason = match e.code() {
                        Code::NotFound | Code::Unavailable => {
                            String::from("No chariott service found")
                        }
                        _ => format!("Chariott request failed with '{e:?}'"),
                    };

                    warn!("{reason}, retrying in {ttl_seconds:?} seconds...");
                }

                // Interval between announce heartbeats or connection retries.
                sleep(Duration::from_secs(ttl_seconds)).await;
            }
        });

        Ok(())
    }

    /// Returns the provider url used to register the service under.
    pub fn get_provider_url(&self) -> String {
        self.register_params.provider_url.clone()
    }
}
