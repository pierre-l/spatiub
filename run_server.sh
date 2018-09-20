#!/bin/bash

sudo timeout 185s cset shield --exec ./target/release/spatiub_demo_server -- server
