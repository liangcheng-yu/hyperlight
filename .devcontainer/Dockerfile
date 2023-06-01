FROM --platform=linux/amd64 ubuntu:22.04

WORKDIR /home/

COPY . .

ENV PATH="/root/.cargo/bin:$PATH"

RUN bash ./setup.sh