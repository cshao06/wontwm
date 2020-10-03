#!/usr/bin/env bash

killall wontwm
killall Xephyr

cargo build

CUR_DIR="$(dirname $(readlink -f $0))"
# SCREEN_SIZE=${SCREEN_SIZE:-800x600}
SCREEN_SIZE=${SCREEN_SIZE:-1280x720}
XDISPLAY=${XDISPLAY:-:1}
# EXAMPLE=${EXAMPLE:-local_test}
# APP=${APP:-st}
APP=${APP:-alacritty}

Xephyr ${XDISPLAY} +extension RANDR -screen ${SCREEN_SIZE} -xinerama -ac &
# Xephyr +extension RANDR -screen ${SCREEN_SIZE} +xinerama ${XDISPLAY} -ac &
# Xephyr -screen ${SCREEN_SIZE} ${XDISPLAY} -ac &
XEPHYR_PID=$!
echo $XEPHYR_PID

sleep 1
env DISPLAY=${XDISPLAY} "$CUR_DIR/target/debug/wontwm" &
WM_PID=$!
echo $WM_PID

# trap "kill $XEPHYR_PID && kill $WM_PID" SIGINT SIGTERM exit

# env DISPLAY=${XDISPLAY} ${APP} &
# wait $WM_PID
# kill $XEPHYR_PID
