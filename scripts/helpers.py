#!/usr/bin/env python3

import json


def read_json(path):
    with open(path) as stream:
        return json.load(stream)


def write_json(data, path):
    with open(path, 'w') as stream:
        json.dump(data, stream, indent=4)


def read_lines(path):
    with open(path) as stream:
        return [v.strip() for v in stream]
