#!/usr/bin/env python3

import click
import datetime
import helpers
import os
import os.path
import random
import subprocess
import time
import traceback


@click.command()
@click.option('--player_types', default='Tcp QuickStart', type=str)
@click.option('--game_type', default='Round1', type=click.Choice((
    'Round1',
    'Round2',
    'Finals',
)))
@click.option('--start_port', default=40010, type=int)
@click.option('--max_runs', default=2**64 - 1, type=int)
@click.option('--prefix', default='default')
@click.option('--bin_path', required=True, type=click.Path(exists=True, dir_okay=False))
@click.option('--output_path', default=os.path.join(os.getcwd(), 'results'), type=click.Path(dir_okay=True, file_okay=False))
@click.option('--verbose', is_flag=True)
@click.option('--timeout', default=120, type=int)
def main(**kwargs):
    run(**kwargs)


def run(player_types, game_type, start_port, max_runs, prefix, bin_path, output_path, verbose, timeout, should_stop=None):
    player_types = parse_player_types(player_types)
    session = f"{prefix}.{'.'.join(player_types)}.{game_type}.{start_port}.{datetime.datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}"
    session_path = os.path.join(output_path, game_type, session)
    os.makedirs(session_path, exist_ok=False)
    player_ports = list((v, n + start_port) for n, v in enumerate(player_types))
    seeds = set()
    seed = random.randint(0, 2**64 - 1)
    for number in range(max_runs):
        if should_stop is not None and should_stop.is_set():
            break
        game = f"{number}.{int(time.time() * 1e6)}"
        game_path = os.path.join(session_path, game)
        while seed in seeds:
            seed = random.randint(0, 2**64 - 1)
        seeds.add(seed)
        process = run_game(
            player_ports=player_ports,
            player_names=[f'{v[0]}:{v[1]}' for v in player_ports],
            game_type=game_type,
            bin_path=bin_path,
            verbose=verbose,
            output_path=game_path,
            seed=seed,
        )
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            traceback.print_exc()


def parse_player_types(value):
    result = value.split(' ')
    assert 2 <= len(result) <= 4, len(result)
    for v in result:
        assert v in {'Tcp', 'QuickStart', 'Empty'}, v
    return result


def run_game(player_ports, player_names, game_type, bin_path, verbose, output_path, seed, visual):
    os.makedirs(output_path, exist_ok=False)
    config_path = os.path.join(output_path, 'config.json')
    result_path = os.path.join(output_path, 'result.json')
    players_path = os.path.join(output_path, 'players.json')
    config = generate_config(
        player_ports=player_ports,
        game_type=game_type,
        seed=seed,
    )
    helpers.write_json(data=config, path=config_path)
    helpers.write_json(data=player_names, path=players_path)
    args = [
        os.path.abspath(bin_path),
        '--config', config_path,
        '--save-results', result_path,
        '--player-names', *player_names,
    ]
    if not visual:
        args.append('--batch-mode')
    if verbose:
        print('Run', *args)
    return subprocess.Popen(
        args=args,
        stdout=None if verbose else subprocess.DEVNULL,
        stderr=None if verbose else subprocess.DEVNULL,
    )


def generate_config(player_ports, game_type, seed):
    return {
        'seed': seed,
        'game': {
            'Create': game_type,
        },
        'players': [make_player(player_type=v[0], port=v[1]) for v in player_ports]
    }


def make_player(player_type, port):
    if player_type == 'Tcp':
        return {
            'Tcp': {
                'host': None,
                'port': port,
                'accept_timeout': None,
                'timeout': None,
                'token': None,
            }
        }
    if player_type == 'QuickStart':
        return 'QuickStart'
    if player_type == 'Empty':
        return {'Empty': None}
    raise RuntimeError('Invalid opponent_type: %s' % player_type)


if __name__ == "__main__":
    main()
