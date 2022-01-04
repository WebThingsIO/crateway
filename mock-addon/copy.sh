#!/bin/bash -e

TARGET_DIR="$1/mock-addon/"
mkdir $TARGET_DIR

cp LICENSE $TARGET_DIR
cp manifest.json $TARGET_DIR
cp target/debug/mock-addon $TARGET_DIR
