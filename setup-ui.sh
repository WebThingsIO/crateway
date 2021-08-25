#!/usr/bin/env sh

git clone https://github.com/WebThingsIO/gateway.git
cd gateway || exit
npm ci
echo 'Running webpack'
node_modules/.bin/webpack
cd ..
