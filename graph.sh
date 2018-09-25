#!/usr/bin/gnuplot -persist
set terminal png size 1024,720
set output 'latency.png'

set datafile separator ","
plot "client_log.csv" using 2:1
