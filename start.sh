#! /bin/bash

if [ $# -lt 1 ];then
    echo "Usage: $0 [debug/release]"
    exit
fi

mode=$1

cd "$(dirname "$0")"
ROOT="$(pwd)"
PJROOT="$ROOT"

logs_dir=$PJROOT/logs
if [ ! -d "$logs_dir" ]; then
    mkdir -p "$logs_dir"
fi

if [ "$mode" == "debug" ];then
    if [ ! -f ./blob-indexer-debug ];then
        echo "binary blob-indexer-debug not found"
    fi
    nohup ./blob-indexer-debug &>> $logs_dir/indexer.log &
elif [ "$mode" == "release" ];then
    if [ ! -f ./blob-indexer-release ];then
        echo "binary blob-indexer-release not found"
    fi
    nohup ./blob-indexer-release &>> $logs_dir/indexer.log &
else
    echo "mode not match, must be debug or release"
fi
