#! /bin/bash

if [ $# -lt 1 ];then
    echo "Usage: $0 [debug/release]"
    exit
fi

mode=$1
if [ "$mode" == "debug" ];then
    cargo build
    [ -f ./blob-indexer-debug ] && rm -f ./blob-indexer-debug
    cp ./target/debug/blob-indexer ./blob-indexer-debug
elif [ "$mode" == "release" ];then
    cargo build --release
    [ -f ./blob-indexer-release ] && rm -f ./blob-indexer-release
    cp ./target/release/blob-indexer ./blob-indexer-release
else
    echo "mode not match, must be debug or release"
fi
