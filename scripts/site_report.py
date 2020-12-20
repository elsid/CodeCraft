#!/usr/bin/env python3

import click
import requests
import statistics
import sys
import termtables

from pyquery import PyQuery
from collections import namedtuple, defaultdict


Game = namedtuple('Game', ('id', 'type', 'creator', 'players', 'players_by_name'))
Player = namedtuple('Player', ('name', 'version', 'place', 'score'))


@click.command()
@click.option('--profile', default='elsid', help='Player profile')
@click.option('--opponent', default=None, help='Opponent profile (default: all)')
@click.option('--first_page', default=1, help='First pages to fetch', type=int)
@click.option('--last_page', default=1, help='Last pages to fetch', type=int)
@click.option('--first_game_id', default=1, help='First game id to fetch', type=int)
@click.option('--last_game_id', default=sys.maxsize, help='Last game id to fetch', type=int)
@click.option('--version', default=None, help='Version to check (default: all)', type=int)
@click.option('--sort_by', default='n', help='Sort by field')
@click.option('--creator', default=None, help='Game creator (default: all)')
@click.option('--category', default='allGames', help='Game stage name (default: allGames)', type=click.Choice((
    'allGames',
    'ownGames',
    'byOtherCreator',
    'contest1',
    'contest2',
    'contest3',
    'contest4',
    'gamesWithCrash',
)))
@click.option('--game_type', default=None, help='Game type (default: all)')
def main(profile, opponent, first_page, last_page, first_game_id, last_game_id, version, sort_by, creator, category, game_type):
    games = list(fetch_games(profile, first_page, last_page, category))
    show_report(games, profile, version, opponent, first_game_id, last_game_id, sort_by, creator, game_type)


def show_report(games, profile, version, opponent, first_game_id, last_game_id, sort_by, creator, game_type):
    games = [v for v in games if check_game(v, profile, version, opponent, first_game_id, last_game_id, creator, game_type)]
    show_stats_by_game_type(games, profile, sort_by)
    show_stats_by_opponent(games, profile, sort_by)
    show_stats_by_opponent_and_version(games, profile, sort_by)
    show_stats_by_opponent_and_game_type(games, profile, sort_by)
    show_stats_by_opponent_and_version_and_game_type(games, profile, sort_by)


def check_game(game, profile, version, opponent, first_game_id, last_game_id, creator, game_type):
    return (
            profile in game.players_by_name
            and (opponent is None or opponent in game.players_by_name)
            and (version is None or game.players_by_name[profile].version == version)
            and first_game_id <= game.id <= last_game_id
            and (creator is None or game.creator == creator)
            and (game_type is None or game.type == game_type)
    )


def show_stats_by_game_type(games, profile, sort_by):
    stats = defaultdict(lambda: dict(scores=list(), places=list()))
    for game in games:
        stats[game.type]['scores'].append(game.players_by_name[profile].score)
        stats[game.type]['places'].append(game.players_by_name[profile].place)
    termtables.print(
        list(make_stats_rows(stats, sort_by)),
        header=['game_type', 'n', 'games', 'total_score', 'mean_score', 'median_score', 'mean_place', 'median_place'],
        style=termtables.styles.markdown,
    )
    print()


def show_stats_by_opponent(games, profile, sort_by):
    stats = defaultdict(lambda: dict(scores=list(), places=list()))
    for game in games:
        opponent = next((v for v in game.players_by_name.keys() if v != profile), None)
        if opponent is not None:
            stats[opponent]['scores'].append(game.players_by_name[profile].score)
            stats[opponent]['places'].append(game.players_by_name[profile].place)
    termtables.print(
        list(make_stats_rows(stats, sort_by)),
        header=['opponent', 'n', 'games', 'total_score', 'mean_score', 'median_score', 'mean_place', 'median_place'],
        style=termtables.styles.markdown,
    )
    print()


def show_stats_by_opponent_and_version(games, profile, sort_by):
    stats = defaultdict(lambda: dict(scores=list(), places=list()))
    for game in games:
        opponent = next((v for v in game.players_by_name.keys() if v != profile), None)
        if opponent is not None:
            key = (opponent, game.players_by_name[opponent].version)
            stats[key]['scores'].append(game.players_by_name[profile].score)
            stats[key]['places'].append(game.players_by_name[profile].place)
    termtables.print(
        list(make_stats_rows(stats, sort_by)),
        header=['opponent', 'version', 'n', 'games', 'total_score', 'mean_score', 'median_score', 'mean_place', 'median_place'],
        style=termtables.styles.markdown,
    )
    print()


def show_stats_by_opponent_and_game_type(games, profile, sort_by):
    stats = defaultdict(lambda: dict(scores=list(), places=list()))
    for game in games:
        opponent = next((v for v in game.players_by_name.keys() if v != profile), None)
        if opponent is not None:
            key = (opponent, game.type)
            stats[key]['scores'].append(game.players_by_name[profile].score)
            stats[key]['places'].append(game.players_by_name[profile].place)
    termtables.print(
        list(make_stats_rows(stats, sort_by)),
        header=['opponent', 'game_type', 'n', 'games', 'total_score', 'mean_score', 'median_score', 'mean_place', 'median_place'],
        style=termtables.styles.markdown,
    )
    print()


def show_stats_by_opponent_and_version_and_game_type(games, profile, sort_by):
    stats = defaultdict(lambda: dict(scores=list(), places=list()))
    for game in games:
        opponent = next((v for v in game.players_by_name.keys() if v != profile), None)
        if opponent is not None:
            key = (opponent, game.players_by_name[opponent].version, game.type)
            stats[key]['scores'].append(game.players_by_name[profile].score)
            stats[key]['places'].append(game.players_by_name[profile].place)
    termtables.print(
        list(make_stats_rows(stats, sort_by)),
        header=['opponent', 'version', 'game_type', 'n', 'games', 'total_score', 'mean_score', 'median_score', 'mean_place', 'median_place'],
        style=termtables.styles.markdown,
    )
    print()


def make_stats_rows(stats, sort_by):
    for v in sorted((make_row(k, n, v) for n, (k, v) in enumerate(stats.items())), key=lambda w: w[sort_by]):
        yield make_stats_row(v)
    total = dict(scores=list(), places=list())
    for k, v in stats.items():
        total['scores'] += v['scores']
        total['places'] += v['places']
    if isinstance(k, str):
        yield make_stats_row(make_row('total', len(stats), total))
    elif isinstance(k, tuple):
        yield make_stats_row(make_row(['total'] + [''] * (len(k) - 1), len(stats), total))


def make_row(key, row, values):
    games = len(values['scores'])
    return dict(
        key=key,
        n=row,
        games=games,
        total_score=sum(values['scores']),
        mean_score=statistics.mean(values['scores']) if games > 0 else 0,
        median_score=statistics.median(values['scores']) if games > 0 else 0,
        mean_place=statistics.mean(values['places']) if games > 0 else 0,
        median_place=statistics.median(values['places']) if games > 0 else 0,
    )


def make_stats_row(values):
    key = [values['key']] if isinstance(values['key'], str) else values['key']
    del values['key']
    return [*key, *list(values.values())]


def fetch_games(profile, first_page, last_page, category):
    for page in range(first_page, last_page + 1):
        url = f'https://russianaicup.ru/profile/{profile}/{category}/page/{page}'
        response = requests.get(url=url)
        root = PyQuery(response.text)
        games_table = root('.gamesTable > tbody')
        rows = PyQuery(games_table.html())
        yield from (v for v in rows('tr').map(lambda k, v: parse_game(PyQuery(v))) if v)


def parse_game(query):
    if query('td:nth-child(7)').text() in ('Game is testing now', 'Game is in queue', ''):
        return None
    players = parse_players(query('td'))
    return Game(
        id=int(query('td:nth-child(1) > a:nth-child(1)').text()),
        type=query('td:nth-child(2)').text().replace('\u00d7', 'x'),
        creator=query('td:nth-child(4) > div:nth-child(1)').text(),
        players=players,
        players_by_name={v.name: v for v in players},
    )


def parse_players(query):
    names = [v.text() for v in query('td:nth-child(5) > a > span').items()]
    versions = [int(v) for v in query('td:nth-child(6)').text().split('\n')]
    scores = [int(v.text()) for v in query('td:nth-child(7) > div').items()]
    places = [int(v.text()) for v in query('td:nth-child(8) > div').items()]
    for n in range(len(names)):
        yield Player(name=names[n], version=versions[n], score=scores[n], place=places[n])


if __name__ == '__main__':
    main()
