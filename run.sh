#!/usr/bin/env bash

export RUSTFLAGS="-C target-cpu=native"
cargo build --release

sudo rm -f client_log*

DURATION=$1
if [ -z "$1" ]
then
	DURATION=180
fi

./run_server.sh $DURATION &
sleep 5s

sudo timeout $DURATION"s" chrt -f 99 ./target/release/spatiub_demo_client -r 2 -n 125

./stats.sh
./graph.sh

sudo rm client_log.csv