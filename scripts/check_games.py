#!/usr/bin/env python3

import os.path
import sys
import results_stats


def main():
    paths = sorted(sys.argv[2:], key=get_time)
    games = list(results_stats.collect_data(paths))
    check_games(games, sys.argv[1])


def check_games(games, player):
    for game in games:
        check_game(game, player)


def check_game(game, player):
    results = game['results']
    if player not in results.keys():
        return
    seed = game['seed']
    if sum(1 for v in results.values() if v['score'] == 0) == len(results):
        print('zero_draw', seed, results[player]['position'], results[player]['score'])
    scores = sorted((v['score'] for v in results.values()), reverse=True)
    place = next((n for n, v in enumerate(scores) if v == results[player]['score'])) + 1
    if place != 1:
        print('loss', seed, place, results[player]['position'], results[player]['score'])
    for k, v in results.items():
        if v['crashed']:
            print('crashed', k, seed, place, v['position'], v['score'])
    if results[player]['score'] == 0:
        print('zero_score', seed, results[player]['position'], results[player]['score'])


def get_time(path):
    return int(os.path.basename(path).split('.')[-1])


if __name__ == '__main__':
    main()
