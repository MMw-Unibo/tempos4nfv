#!/bin/bash

if [ "$EUID" -ne 0 ]
  then echo "Please run as root"
  exit
fi

# check if ptp4l command exists
if ! [ -x "$(command -v ptp4l)" ]; then
  echo 'Error: ptp4l is not installed.' >&2
  exit 1
fi

# check if phc2sys command exists
if ! [ -x "$(command -v phc2sys)" ]; then
  echo 'Error: phc2sys is not installed.' >&2
  exit 1
fi

# check if ptp4l is running and kill it
if pgrep -x "ptp4l" > /dev/null
then
    echo "ptp4l is running, killing it"
    killall ptp4l
fi

# check if phc2sys is running and kill it
if pgrep -x "phc2sys" > /dev/null
then
    echo "phc2sys is running, killing it"
    killall phc2sys
fi

IFACE=eth0
CONFIG=../config/ptp/
LOGLVL=6

# get iface from first argument
if [ -n "$1" ]; then
    IFACE=$1
fi

# get filename from second argument and append to config path
if [ -n "$2" ]; then
    CONFIG=$CONFIG$2
fi

# check if iface exists
ip link show $IFACE > /dev/null
if [ ! $? -eq 0 ]; then
    echo "Interface $IFACE does not exist"
    exit 1
fi

echo "Starting ptp4l and phc2sys on $IFACE"

#check if tmux session is running
if tmux has-session -t ptp 2>/dev/null; then
    echo "tmux session ptp is running, killing it"
    tmux kill-session -t ptp
fi

# start tmux session
tmux new-session -d -s ptp  
# split window two panes
tmux split-window -h
# start ptp4l in first pane
tmux send-keys -t ptp:0.0 "ptp4l -i $IFACE -f $CONFIG -m -l $LOGLVL" C-m
# start phc2sys in second pane
# - 's' is the clock source
# - 'c' is the clock to sync 
tmux send-keys -t ptp:0.1 "phc2sys -s CLOCK_REALTIME -c $IFACE -O 0 -m -l $LOGLVL" C-m
