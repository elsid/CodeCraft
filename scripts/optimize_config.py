#!/usr/bin/env python3

import click
import collections
import copy
import datetime
import functools
import games
import helpers
import itertools
import json
import numpy
import operator
import os
import os.path
import results_stats
import time


@click.command()
@click.option('--config_path', required=True, type=str)
@click.option('--options_index_path', required=True, type=str)
@click.option('--seeds_path', required=True, type=str)
@click.option('--players', required=True, type=str)
@click.option('--target_player', required=True, type=str)
@click.option('--game_type', default='Round1', type=click.Choice((
    'Round1',
    'Round2',
    'Finals',
)))
@click.option('--runner_bin_path', required=True, type=click.Path(exists=True, dir_okay=False))
@click.option('--start_port', default=40010, type=int)
@click.option('--workers', default=1, type=int)
@click.option(
    '--output_path',
    default=os.path.join(os.getcwd(), 'results/optimization'),
    type=click.Path(dir_okay=True, file_okay=False),
)
@click.option('--verbose', is_flag=True)
@click.option('--timeout', default=120, type=int)
@click.option('--repeats_per_seed', default=1, type=int)
@click.option('--max_iterations', default=1, type=int)
def main(**kwargs):
    run(**kwargs)


def run(config_path, options_index_path, seeds_path, players, target_player, game_type, runner_bin_path,
        start_port, workers, output_path, verbose, timeout, repeats_per_seed, max_iterations):
    config = helpers.read_json(config_path)
    options_index = helpers.read_json(options_index_path)
    seeds = [int(v) for v in helpers.read_lines(seeds_path) if v]
    options_index = {
        k: dict(
            convert=make_option_convert(v['type'], v),
            scale=make_option_scale(v['type'], v),
            type=v['type'],
            index=v['index'],
        )
        for k, v in options_index.items()
    }
    initial = make_initial(config, options_index)
    function_calls = [0]
    players = tuple(games.parse_players(players, start_port))
    session = f"{game_type}.{games.format_players(players)}.{start_port}.{datetime.datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}"
    games_path = os.path.join(output_path, session)

    def function(iteration, args):
        start = time.time()
        current_config = copy.deepcopy(config)
        for k, v in options_index.items():
            current_config[k] = v['convert'](args[v['index']])
        call = function_calls[0]
        function_calls[0] += 1
        score, places = run_games(
            players=players,
            target_player=target_player,
            game_type=game_type,
            runner_bin_path=runner_bin_path,
            workers_number=workers,
            output_path=games_path,
            verbose=verbose,
            seeds=seeds,
            timeout=timeout,
            repeats_per_seed=repeats_per_seed,
            config=current_config,
            iteration=call,
        )
        iteration_result = sum((5 - 2 * k) * v for k, v in places.items()) + score / 1000000
        duration = time.time() - start
        print(json.dumps(dict(
            iteration=iteration,
            call=call,
            score=score,
            places=places,
            result=iteration_result,
            duration=duration,
            config=current_config,
        )))
        return -iteration_result

    scale_f = [None] * len(options_index)
    for v in options_index.values():
        scale_f[v['index']] = v['scale']
    result = minimize_ga(
        function=function,
        initial=numpy.array(initial),
        max_iterations=max_iterations,
        scale_f=scale_f,
    )
    for k, v in options_index.items():
        config[k] = v['convert'](result[1][v['index']])
    print(json.dumps(config))


def make_option_convert(name, data):
    if name == 'float':
        return float
    if name == 'min_float':
        min_value = data['min']
        return lambda v: max(float(v), min_value)
    if name == 'max_float':
        max_value = data['max']
        return lambda v: min(float(v), max_value)
    if name == 'bounded_float':
        min_value = data['min']
        max_value = data['max']
        return lambda v: max(min(float(v), max_value), min_value)
    raise RuntimeError(f'Invalid type name: {name}')


def make_option_scale(name, data):
    if name == 'float':
        return lambda v: numpy.abs(v) / 2.0
    if name == 'min_float':
        min_value = data['min']
        return lambda v: numpy.abs((v - min_value) / 2.0)
    if name == 'max_float':
        max_value = data['max']
        return lambda v: numpy.abs((max_value - v) / 2.0)
    if name == 'bounded_float':
        min_value = data['min']
        max_value = data['max']
        return lambda v: min(numpy.abs((v - min_value) / 2.0), numpy.abs((max_value - v) / 2.0))
    raise RuntimeError(f'Invalid type name: {name}')


def minimize_ga(function, initial, max_iterations, scale_f):
    iteration = [0]
    minimum = (function(0, initial), initial)
    generator = generate_minimums(
        function=function,
        iteration=iteration,
        generation=[initial for _ in range(max(len(initial), 4))],
        minimum=minimum,
        max_generation_size=len(initial),
        scale_f=scale_f,
    )
    for n in range(1, max_iterations + 1):
        iteration[0] = n
        minimum, generation = next(generator)
        print(json.dumps(dict(
            iteration=n,
            minimum=dict(value=minimum[0], args=list(minimum[1])),
            generation_len=len(generation),
        )))
    return minimum


def generate_minimums(function, iteration, generation, minimum, max_generation_size, scale_f):
    while True:
        mutations = [mutate(v, scale_f) for v in generation]
        results = sorted(((function(iteration[0], v), v) for v in mutations), key=lambda v: v[0])
        if minimum[0] > results[0][0]:
            minimum = results[0]
        selected = [v for _, v in results[:get_size_of_selection(max_generation_size)]]
        generation = [crossover(pair[0], pair[1]) for pair in itertools.combinations(selected, 2)]
        yield minimum, generation


def mutate(args, scale_f):
    return numpy.random.normal(args, numpy.array([f(v) for f, v in zip(scale_f, args)]))


def crossover(left, right):
    result = numpy.copy(left)
    for n in range(len(right)):
        if numpy.random.randint(0, 1) == 1:
            result[n] = right[n]
    return result


def get_size_of_selection(max_combinations_number):
    result = 2
    combinations = 0
    while combinations < max_combinations_number:
        combinations = number_of_combinations(n=result, r=2)
        result += 1
    return result


def number_of_combinations(n, r):
    r = min(r, n - r)
    numer = functools.reduce(operator.mul, range(n, n - r, -1), 1)
    denom = functools.reduce(operator.mul, range(1, r + 1), 1)
    return numer // denom


def run_games(players, target_player, game_type, runner_bin_path, workers_number, output_path, verbose,
              seeds, timeout, repeats_per_seed, config, iteration):
    etc_path = os.path.join(output_path, 'etc')
    os.makedirs(etc_path, exist_ok=True)
    games_path = os.path.join(output_path, 'games', str(iteration))
    os.makedirs(games_path, exist_ok=True)
    config_path = os.path.abspath(os.path.join(etc_path, 'config.%s.json' % iteration))
    helpers.write_json(config, config_path)
    scheduler = games.Scheduler(
        workers_number=workers_number,
        ports_per_worker=len(players),
        verbose=verbose,
        timeout=timeout,
    )
    scheduler.start()
    number = 0
    permutations = tuple(itertools.permutations(players))
    total = len(seeds) * len(permutations) * repeats_per_seed
    for seed in seeds:
        for permutation in permutations:
            for _ in range(repeats_per_seed):
                if verbose:
                    print(f'Iteration {iteration} has {total - number} more tasks')
                scheduler.put_task(games.Task(
                    runner=games.Runner(
                        bin_path=runner_bin_path,
                        game_type=game_type,
                        seed=seed,
                        output_path=os.path.join(games_path, '%s.%s' % (number, int(time.time() * 1e6))),
                        config_path=config_path,
                        visual=False,
                    ),
                    players=permutation,
                ))
                number += 1
    scheduler.join()
    return get_games_result(games_path, target_player)


def get_games_result(games_path, target_player):
    sum_score = 0
    places = collections.Counter()
    for game in collect_games(games_path):
        player_score = game['results'][target_player]['score']
        sum_score += player_score
        scores = sorted((v['score'] for v in game['results'].values()), reverse=True)
        places[next(n for n, v in enumerate(scores) if v == player_score) + 1] += 1
    return sum_score, places


def collect_games(path):
    for dir_name in os.listdir(path):
        dir_path = os.path.join(path, dir_name)
        if os.path.exists(os.path.join(dir_path, 'config.json')):
            yield from results_stats.collect_data([os.path.join(path, v) for v in os.listdir(path)])
            return
        else:
            yield from collect_games(os.path.exists(os.path.join(path, dir_name)))


def make_initial(config, options_index):
    initial = [0] * len(options_index)
    for k, v in options_index.items():
        initial[v['index']] = config[k]
    return initial


if __name__ == '__main__':
    main()
