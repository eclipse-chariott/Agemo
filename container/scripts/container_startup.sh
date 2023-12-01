#!/bin/bash

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT license.
# SPDX-License-Identifier: MIT

# Exits immediately on failure.
set -eu

# Copy any configuration files present to service configuration.
cp -rf /mnt/config /sdv/.agemo

/sdv/service
