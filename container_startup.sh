#!/bin/bash

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT license.
# SPDX-License-Identifier: MIT

# Exits immediately on failure.
set -e

# Copy any configuration files present to service configuration.
cp -rn /mnt/config /sdv/.agemo

/sdv/service
