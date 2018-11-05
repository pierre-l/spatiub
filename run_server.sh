#!/usr/bin/env bash

sudo timeout $(($1 + 5))s chrt -f 99 ./target/release/spatiub_demo_server
