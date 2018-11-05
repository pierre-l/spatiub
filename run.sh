#!/usr/bin/env bash

export RUSTFLAGS="-C target-cpu=native"
cargo build --release

sudo rm -f client_log*

TOTAL_NUM_PU=$(nproc)
THREADS_PER_CORE=$(lscpu | grep 'per core:' | awk '{print $4}')
TOTAL_NUM_CORES=$((TOTAL_NUM_PU / $THREADS_PER_CORE))

if [ ${TOTAL_NUM_PU} -ne ${TOTAL_NUM_CORES} ]; then
    CPU_SET=$THREADS_PER_CORE"-"$(($TOTAL_NUM_PU - 1))
else
    CPU_SET="1-"$(($TOTAL_NUM_PU - 1))
fi

sudo cset shield --cpu=${CPU_SET} --kthread=on
sleep 3s

DURATION=$1
if [ -z "$1" ]
then
	DURATION=180
fi

./run_server.sh $DURATION $(($TOTAL_NUM_CORES - 2)) &
sleep 5s

sudo timeout $DURATION"s" cset shield --exec chrt -f 99 ./target/release/spatiub_demo_client -- -r 2 -n 125 -c $(($TOTAL_NUM_CORES - 2))

sudo cset shield -r

cat client_log_*.csv > client_log.csv

./stats.sh
./graph.sh

sudo rm client_log_*