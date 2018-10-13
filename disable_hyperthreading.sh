#!/usr/bin/env bash

for CPU in /sys/devices/system/cpu/cpu[0-9]*; do
      CPUID=`basename $CPU | cut -b4-`

      [ -e $CPU/online ] && echo "1" > $CPU/online

      THREAD1=`cat $CPU/topology/thread_siblings_list | cut -f1 -d,`
      if [ $CPUID -ne $THREAD1 ]; then
          echo "0" > $CPU/online
      fi
done