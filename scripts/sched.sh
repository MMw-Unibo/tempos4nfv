#!/bin/bash

if [ "$EUID" -ne 0 ]; then
    echo "Please run as root"
    exit
fi

# check if tc command exists
if ! [ -x "$(command -v tc)" ]; then
    echo 'Error: tc is not installed.' >&2
    exit 1
fi

# check if jq command exists
if ! [ -x "$(command -v jq)" ]; then
    echo 'Error: jq is not installed.' >&2
    echo 'Please install jq with "sudo apt install jq"' >&2
    exit 1
fi

IFACE=enp5s0

# check if iface exists
ip link show $IFACE >/dev/null
if [ ! $? -eq 0 ]; then
    echo "Interface $IFACE does not exist"
    exit 1
fi

TIME=$(date +%s%N)
TIME=$(($TIME / 1000000000))
TIME=$(($TIME * 1000000000))

config_taprio() {
    IFACE=$1
    CONFIG=$2

    # check if iface exists
    ip link show $IFACE >/dev/null
    if [ ! $? -eq 0 ]; then
        echo "Interface $IFACE does not exist"
        exit 1
    fi

    # remove existing qdisc
    tc qdisc del dev $IFACE root >/dev/null 2>&1

    # get number of traffic classes
    NUM_TC=$(jq '.queues | length' $CONFIG)

    # get maps
    MAP="$(jq -r '.map  | join (" ")' $CONFIG)"
    MAP_LEN="$(jq -r '.map  | length ' $CONFIG)"

    # check map length equals 16 and and max value is num_tc - 1
    if [ $MAP_LEN -ne 16 ]; then
        echo "Map must be 16 values"
        exit 1
    fi

    for i in $MAP; do
        if [ $i -ge $NUM_TC ]; then
            echo "Map value must be less than num_tc"
            exit 1
        fi
    done

    # get queue list
    QUEUES=$(jq -r '.queues | map("\(.count)@\(.map)") | join(" ")' $CONFIG)

    # get sched entries
    SCHED_ENTRIES=$(jq -r '.sched_entries | map("sched-entry \(.type) \(.mask) \(.time)") | join(" ")' $CONFIG)

    # get clock id
    CLOCK_ID=$(jq -r '.clockid' $CONFIG)

    # print config
    echo "Configuring interface $IFACE with the following parameters:"
    echo "  num_tc: $NUM_TC"
    echo "  map: $MAP"
    echo "  queues: $QUEUES"
    echo "  base_time: $TIME"
    echo "  sched_entries: $SCHED_ENTRIES"
    echo "  clock_id: $CLOCK_ID"

    # create qdisc
    tc qdisc add dev $IFACE root handle 100 taprio \
        num_tc $NUM_TC \
        map $MAP \
        queues $QUEUES \
        base-time $TIME \
        $SCHED_ENTRIES \
        clockid $CLOCK_ID \
        flags 0x1 \
        txtime-delay 200000

    tc qdisc replace dev $IFACE parent 100:1 etf \
        skip_sock_check \
        offload \
        delta 200000 \
        clockid $CLOCK_ID

    # tc show configured qdisc
    tc -s qdisc show dev $IFACE
}

init_interface() {
    IFACE=$1

    # check if iface exists
    ip link show $IFACE >/dev/null
    if [ ! $? -eq 0 ]; then
        echo "Interface $IFACE does not exist"
        exit 1
    fi

    # remove existing qdisc
    tc qdisc del dev $IFACE root >/dev/null 2>&1

    # set an even number of queues

}

config_taprio $IFACE ../config/sched/taprio.json
