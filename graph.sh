#!/usr/bin/gnuplot -persist
#plot "client_log.csv"
plot for [col=2:2] "client_log.csv" using 1:col
