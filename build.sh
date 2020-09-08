#!/bin/bash

cargo test
cargo build --release

docker build -t zynaxsoft/belowtherocks:latest .
