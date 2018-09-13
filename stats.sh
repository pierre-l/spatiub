#!/usr/bin/env bash
tmp_file="tmp_latency.csv"

cut --complement -f 1 -d, client_log.csv > $tmp_file

format="%d"

st --mean --stddev --min --max --format=$format $tmp_file

echo ""

p99=$(st --format=$format --percentile=99 $tmp_file)
echo "99 percentile: $p99"

p999=$(st --format=$format --percentile=99.9 $tmp_file)
echo "99.9 percentile: $p999"

p9999=$(st --format=$format --percentile=99.99 $tmp_file)
echo "99.99 percentile: $p9999"

rm $tmp_file
