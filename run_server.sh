#!/bin/bash

sudo timeout 185s cset shield --exec chrt -f 99 ./target/release/spatiub_demo_server
