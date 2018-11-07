#!/usr/bin/env bash

docker build -t spatiub:latest ./

rm -f client_log*

DURATION=$1
if [ -z "$1" ]
then
	DURATION=180
fi

TOTAL_DURATION=$(($DURATION + 5))

docker network create spatiub

docker run -d --name spatiub_server --network spatiub --privileged spatiub:latest \
    timeout $TOTAL_DURATION"s" chrt -f 99 ./spatiub_demo_server -b 0.0.0.0:6142

sleep 5s

SERVER_ADDRESS=$(docker run --rm --network spatiub -it debian getent hosts spatiub_server.spatiub | awk '{ print $1 }')

echo "Server address: "$SERVER_ADDRESS

docker run -d --name spatiub_client --network spatiub --privileged spatiub:latest \
    timeout ${DURATION}"s" chrt -f 99 /spatiub_demo_client -r 2 -n 125 -a ${SERVER_ADDRESS}:6142

docker logs -f spatiub_client
docker logs spatiub_server

docker cp spatiub_client:/client_log.csv ./

docker rm -f spatiub_client
docker rm -f spatiub_server
docker network rm spatiub

./stats.sh
./graph.sh

rm client_log.csv