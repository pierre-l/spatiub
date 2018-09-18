#!/usr/bin/gnuplot -persist
set datafile separator ","
plot "client_log.csv" using 2:1
