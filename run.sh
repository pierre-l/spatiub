#!/usr/bin/env bash
cargo build --release

sudo rm client_log*

sudo cset shield --cpu=1-3 --kthread=on
sleep 3s

./run_server.sh &
sleep 5s

sudo timeout 180s cset shield --exec ./target/release/spatiub_demo_server -- client

sudo cset shield -r

cat client_log_*.csv > client_log.csv

./stats.sh
./graph.sh

sudo rm client_log*
