#!/bin/sh
REPO=$(pwd)
cd /workspace > /dev/null
git clone https://github.com/yijunyu/cargo-geiger
cd cargo-geiger > /dev/null
git checkout datasets
mkdir -p /workspace/.cargo/bin
cp bin/* /workspace/.cargo/bin
cd $REPO > /dev/null
saferatio.sh
