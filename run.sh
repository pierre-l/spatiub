#!/usr/bin/env bash
cargo build --release

sudo rm client_log*

sudo cset shield --cpu=1-3 --kthread=on
sleep 3s

DURATION=$1
if [ -z "$1" ]
then
	DURATION=180
fi

./run_server.sh $DURATION &
sleep 5s

sudo timeout $DURATION"s" cset shield --exec chrt -f 99 ./target/release/spatiub_demo_client -- -r 10

sudo cset shield -r

cat client_log_*.csv > client_log.csv

./stats.sh
./graph.sh

sudo rm client_log_*
