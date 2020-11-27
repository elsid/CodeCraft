#!/bin/bash -ex

if ! [[ "${CONFIG}" ]]; then
    CONFIG=${1:?}
fi

SRC="${PWD}"

if ! [[ "${RUNNER}" ]]; then
  if [[ ${OS} == "Windows_NT" ]]; then
    RUNNER=windows
  else
    RUNNER=linux
  fi
fi

cd cgdk/runners/${RUNNER}/

./aicup2020 --config ${SRC}/etc/${CONFIG:?}.json
