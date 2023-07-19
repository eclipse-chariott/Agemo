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

pub mod chariott {
    pub mod common {
        pub mod v1 {
            tonic::include_proto!("chariott.common.v1");
        }
    }

    pub mod provider {
        pub mod v1 {
            tonic::include_proto!("chariott.provider.v1");
        }
    }

    pub mod runtime {
        pub mod v1 {
            tonic::include_proto!("chariott.runtime.v1");
        }
    }
}

pub use chariott::common::v1 as chariott_common;
pub use chariott::provider::v1 as chariott_provider;
pub use chariott::runtime::v1 as chariott_runtime;
