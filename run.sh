#!/usr/bin/env bash
cargo build --release

sudo cset shield --cpu=2-3 --kthread=on
sleep 3s
sudo timeout 180s cset shield --exec ./target/release/spatiub_demo_server --
sudo cset shield -r

./stats.sh
./graph.sh
