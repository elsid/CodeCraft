#!/bin/bash -ex

if ! [[ "${CONFIG}" ]]; then
    CONFIG=${1:?}
fi

SRC="${PWD}"

if ! [[ "${RUNNER}" ]]; then
  if [[ ${OS} == "Windows_NT" ]]; then
    RUNNER=aicup2020-windows
  else
    RUNNER=aicup2020-linux
  fi
fi

cd cgdk/runners/${RUNNER}/

ID=$(date +%Y-%m-%d_%H-%M-%S)

mkdir -p ${SRC}/results/temp

./aicup2020 \
    --config ${SRC}/etc/${CONFIG:?}.json \
    --save-results ${SRC}/results/temp/${ID}.results.json
