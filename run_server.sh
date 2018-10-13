#!/usr/bin/env bash

sudo timeout $(($1 + 5))s cset shield --exec chrt -f 99 ./target/release/spatiub_demo_server -- -c $2
