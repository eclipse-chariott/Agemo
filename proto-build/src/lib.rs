// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

pub mod pubsub {
    pub mod v1 {
        tonic::include_proto!("pubsub");
        pub const SCHEMA_KIND: &str = "grpc+proto";
        pub const SCHEMA_REFERENCE: &str = "pubsub.v1.proto";
    }
}

pub mod publisher {
    pub mod v1 {
        tonic::include_proto!("publisher");
        pub const SCHEMA_KIND: &str = "grpc+proto";
        pub const SCHEMA_REFERENCE: &str = "publisher.v1.proto";
    }
}

pub mod service_registry {
    pub mod v1 {
        tonic::include_proto!("service_registry");
    }
}
