#!/usr/bin/env bash
cargo build --release

sudo cset shield --cpu=1-3 --kthread=on
sleep 3s
sudo timeout 60s cset shield --exec ./target/release/spatiub_demo_server --
sudo cset shield -r

cat client_log_*.csv > client_log.csv

./stats.sh
./graph.sh
