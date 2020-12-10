#!/bin/bash -ex

VERSION=$(date +%Y-%m-%d_%H-%M-%S)-$(git rev-parse --short HEAD)
SRC=${PWD}
DIR=${SRC}/release/${VERSION}

mkdir -p release
mkdir ${DIR}

cd ${DIR}/

git init
git remote add origin git@github.com:elsid/CodeCraft-cgdk.git
git remote add MailRuChamps git@github.com:MailRuChamps/raic-2020.git

git fetch origin patch &
git fetch MailRuChamps main &
wait

git checkout patch
git rebase MailRuChamps/main patch

cd clients/Rust/

cp ${SRC}/src/bot.rs src/
cp ${SRC}/src/config.rs src/
cp ${SRC}/src/entity.rs src/
cp ${SRC}/src/entity_planner.rs src/
cp ${SRC}/src/entity_simulator.rs src/
cp ${SRC}/src/entity_type.rs src/
cp ${SRC}/src/groups.rs src/
cp ${SRC}/src/map.rs src/
cp ${SRC}/src/moving_average.rs src/
cp ${SRC}/src/my_strategy.rs src/
cp ${SRC}/src/path.rs src/
cp ${SRC}/src/positionable.rs src/
cp ${SRC}/src/rect.rs src/
cp ${SRC}/src/roles.rs src/
cp ${SRC}/src/stats.rs src/
cp ${SRC}/src/tasks.rs src/
cp ${SRC}/src/vec2.rs src/
cp ${SRC}/src/world.rs src/

cargo build --release

zip ${SRC}/release/${VERSION}.zip -r Cargo.toml src model

ls -al ${SRC}/release/${VERSION}.zip
