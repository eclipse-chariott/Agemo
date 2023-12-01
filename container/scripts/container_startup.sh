#!/bin/bash

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT license.
# SPDX-License-Identifier: MIT

# Exits immediately on failure.
set -eu

# Copy any mounted configuration files present to service configuration at runtime.
# If there is a configuration file with the same name at `/sdv/.agemo/config` this will overwrite
# the file with the mounted configuration file. 
cp -rf /mnt/config /sdv/.agemo

/sdv/service
