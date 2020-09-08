FROM ubuntu:18.04

RUN apt-get update
RUN apt-get install -y libsqlite3-dev

WORKDIR /opt/
