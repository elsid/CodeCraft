#!/usr/bin/env python3

import functools
import helpers
import json
import math
import numbers
import numpy
import operator
import os.path
import statistics
import sys
import termtables

from collections import defaultdict


def main():
    paths = sorted(sys.argv[2:], key=get_time)
    games = list(collect_data(paths))
    stats = get_stats(games)
    print_stats(stats)
    show_stats(stats, player=sys.argv[1])


def print_stats(stats):
    termtables.print(
        list(generate_stats_rows(stats)),
        header=('metric', 'value', '%'),
        style=termtables.styles.markdown,
    )
    termtables.print(
        list(generate_stats_per_player_rows(stats)),
        header=('metric', *stats['players'], *[f'{v}, %' for v in stats['players']], 'total'),
        style=termtables.styles.markdown,
    )


def generate_stats_rows(stats):
    for metric, value in stats.items():
        if isinstance(value, int):
            yield metric, value, value / stats['total_games'] * 100
        if isinstance(value, float):
            yield metric, value, ''


def generate_stats_per_player_rows(stats):
    for metric, values in stats.items():
        yield from generate_metric_rows(metric, values, stats['players'])


def generate_metric_rows(metric, values, players):
    if isinstance(values, dict):
        if values:
            if isinstance(tuple(values.values())[0], dict):
                for submetric, subvalues in values.items():
                    yield from generate_metric_rows(f'{metric} {submetric}', subvalues, players)
            elif isinstance(tuple(values.values())[0], numbers.Number):
                yield make_counter_row(metric, values, players)


def make_counter_row(metric, values, players):
    row_values = [values[v] for v in players]
    total = sum(row_values)
    fractions = [safe_div(v, total) * 100 for v in row_values]
    return tuple([metric, *row_values, *fractions, total])


def safe_div(a, b):
    return a / b if b else float('inf')


def show_stats(stats, player):
    import matplotlib.pyplot as pyplot

    show_metric_plot(pyplot, stats, 'scores_dynamic')
    show_metric_plot(pyplot, stats, 'scores_dynamic_cumsum')
    show_metric_plot(pyplot, stats, 'places_dynamic_cumsum')
    show_metric_plot(pyplot, stats, 'wins_dynamic_cumsum')
    show_metric_plot(pyplot, stats, 'losses_dynamic_cumsum')

    show_percentage_plots(pyplot, stats, player, 'scores_dynamic_cumsum')
    show_percentage_plots(pyplot, stats, player, 'places_dynamic_cumsum')
    show_percentage_plots(pyplot, stats, player, 'wins_dynamic_cumsum')
    show_percentage_plots(pyplot, stats, player, 'losses_dynamic_cumsum')

    show_score_distribution_plot(pyplot, stats)
    show_position_distribution_plot(pyplot, stats)
    show_place_distribution_plot(pyplot, stats)
    show_seed_distribution_plot(pyplot, stats)
    show_duration_distribution_plot(pyplot, stats)
    show_max_tick_duration_distribution_plot(pyplot, stats, player)

    show_places_positions_plot(pyplot, stats, player)

    pyplot.show()


def show_percentage_plots(pyplot, stats, player, metric):
    players = stats['players']
    for n in range(len(players)):
        if player == players[n]:
            show_ratio_plot(
                pyplot,
                name=f'{metric} {players[n]}, %',
                values=stats[metric][players[n]] / sum(stats[metric].values()) * 100,
            )


def show_ratio_plots(pyplot, stats, metric):
    players = stats['players']
    for n in range(len(players)):
        for m in range(n + 1, len(players)):
            show_ratio_plot(
                pyplot,
                name=f'{metric} {players[n]} / {players[m]}',
                values=stats[metric][players[n]] / stats[metric][players[m]],
            )
            show_ratio_plot(
                pyplot,
                name=f'{metric} {players[m]} / {players[n]}',
                values=stats[metric][players[m]] / stats[metric][players[n]],
            )


def show_ratio_plot(pyplot, name, values):
    fig, ax = pyplot.subplots()
    fig.canvas.set_window_title(name)
    ax.set_title(name)
    ax.plot(numpy.arange(0, len(values), 1), values, label=name)
    filtered = [v for v in values[len(values) // 2:] if not math.isinf(v)]
    if filtered:
        filtered = numpy.array(filtered)
        min_v = min(filtered)
        ax.plot([len(values) // 2, len(values) - 1], [min_v, min_v], '-.', label='last half max %s' % min_v)
        max_v = max(filtered)
        ax.plot([len(values) // 2, len(values) - 1], [max_v, max_v], '-.', label='last half max %s' % max_v)
        mean = statistics.mean(filtered)
        ax.plot([len(values) // 2, len(values) - 1], [mean, mean], '--', label='last half mean %s' % mean)
    ax.grid(True)
    ax.legend()


def show_metric_plot(pyplot, stats, metric):
    fig, ax = pyplot.subplots()
    fig.canvas.set_window_title(metric)
    ax.set_title(metric)
    for player, values in stats[metric].items():
        ax.plot(numpy.arange(0, len(values), 1), values, label=player)
    total = functools.reduce(operator.add, stats[metric].values())
    ax.plot(numpy.arange(0, len(total), 1), total, label='total')
    ax.grid(True)
    ax.legend()


def show_plot(pyplot, name, values):
    fig, ax = pyplot.subplots()
    fig.canvas.set_window_title(name)
    ax.set_title(name)
    ax.plot(numpy.arange(0, len(values), 1), values, label=name)
    ax.grid(True)
    ax.legend()


def show_score_distribution_plot(pyplot, stats):
    fig, ax = pyplot.subplots()
    fig.canvas.set_window_title('score_distribution')
    ax.set_title('score_distribution')
    max_value = max(max(v) for v in stats['scores_dynamic'].values())
    bins = numpy.linspace(0, max_value + max_value / 12, 13)
    for player, values in stats['scores_dynamic'].items():
        p, x = numpy.histogram(values, bins=bins)
        p = [0] + list(p) + [0]
        x = x[:-1] + (x[1] - x[0]) / 2
        x = [x[0] - (x[1] - x[0])] + list(x) + [x[-1] + (x[1] - x[0])]
        color = ax.plot(x, p, label=player)[0].get_color()
        ax.fill_between(x, p, color=color, alpha=1 / len(stats['scores_dynamic']))
    ax.set_xticks(bins)
    ax.grid(True)
    ax.legend()


def show_position_distribution_plot(pyplot, stats):
    fig, ax = pyplot.subplots()
    fig.canvas.set_window_title('position_distribution')
    ax.set_title('position_distribution')
    min_position = min(min(v) for v in stats['positions_dynamic'].values())
    max_position = max(max(v) for v in stats['positions_dynamic'].values())
    bins = list(range(min_position, max_position + 2))
    ax.hist(list(stats['positions_dynamic'].values()), bins=bins, label=list(stats['positions_dynamic'].keys()))
    ax.set_xticks(bins)
    ax.grid(True)
    ax.legend()


def show_place_distribution_plot(pyplot, stats):
    fig, ax = pyplot.subplots()
    fig.canvas.set_window_title('place_distribution')
    ax.set_title('place_distribution')
    min_place = min(min(v) for v in stats['places_dynamic'].values())
    max_place = max(max(v) for v in stats['places_dynamic'].values())
    bins = list(range(min_place, max_place + 2))
    ax.hist(list(stats['places_dynamic'].values()), bins=bins, label=list(stats['places_dynamic'].keys()))
    ax.set_xticks(bins)
    ax.grid(True)
    ax.legend()


def show_places_positions_plot(pyplot, stats, player):
    places = sorted(stats['places_positions'].keys())
    positions = sorted(v - 1 for v in places)
    counts = list()
    for place in places:
        place_counts = list()
        for position in positions:
            place_counts.append(0)
            for k, count in stats['places_positions'][place][position].items():
                if k == player:
                    place_counts[-1] = count
        counts.append(place_counts)
    fig, ax = pyplot.subplots()
    fig.canvas.set_window_title('places_positions')
    ax.set_title('places_positions')
    ax.imshow(counts)
    ax.set_xticks(numpy.arange(len(places)))
    ax.set_yticks(numpy.arange(len(positions)))
    ax.set_xticklabels(places)
    ax.set_yticklabels(positions)
    ax.set_xlabel('place')
    ax.set_ylabel('position')
    for i in range(len(places)):
        for j in range(len(positions)):
            ax.text(j, i, counts[i][j], ha="center", va="center", color="w")


def show_seed_distribution_plot(pyplot, stats):
    fig, ax = pyplot.subplots()
    fig.canvas.set_window_title('seeds')
    ax.set_title('seeds')
    bins = numpy.linspace(0, 2**64, 32)
    ax.hist(stats['seeds'], bins=32)
    ax.set_xticks(bins)
    ax.grid(True)


def show_duration_distribution_plot(pyplot, stats):
    fig, ax = pyplot.subplots()
    fig.canvas.set_window_title('duration_distribution')
    ax.set_title('duration_distribution')
    bins = numpy.linspace(0, max(stats['durations']), 50)
    ax.hist(stats['durations'], bins=bins)
    ax.axvline(stats['mean_duration'], label=f"mean = {stats['mean_duration']}", color='r', linestyle='--')
    ax.axvline(stats['median_duration'], label=f"median = {stats['median_duration']}", color='g')
    ax.set_xticks(bins)
    ax.grid(True)
    ax.legend()


def show_max_tick_duration_distribution_plot(pyplot, stats, player):
    if player not in stats['max_tick_duration']:
        return
    values = stats['max_tick_duration'][player]
    if not values:
        return
    fig, ax = pyplot.subplots()
    fig.canvas.set_window_title('max_tick_duration')
    ax.set_title('max_tick_duration')
    bins = numpy.linspace(0, max(values), 50)
    ax.hist(values, bins=bins)
    mean = statistics.mean(values)
    ax.axvline(mean, label=f"mean = {mean}", color='r', linestyle='--')
    median = statistics.median(values)
    ax.axvline(median, label=f"median = {median}", color='g')
    ax.set_xticks(bins)
    ax.grid(True)
    ax.legend()


def get_stats(games):
    players = set()
    for game in games:
        for player in game['results'].keys():
            players.add(player)
    draws = 0
    zero_draws = 0
    fails = 0
    durations = list()
    games_count = {v: 0 for v in players}
    wins = {v: 0 for v in players}
    losses = {v: 0 for v in players}
    places = defaultdict(lambda: {v: 0 for v in players})
    crashes = {v: 0 for v in players}
    positions = defaultdict(lambda: {v: 0 for v in players})
    places_positions = defaultdict(lambda: defaultdict(lambda: {v: 0 for v in players}))
    seeds = set()
    scores = {v: list() for v in players}
    places_dynamic = {v: list() for v in players}
    positions_dynamic = {v: list() for v in players}
    wins_dynamic = {v: list() for v in players}
    losses_dynamic = {v: list() for v in players}
    place_score = {v: 0 for v in players}
    bot_stats = defaultdict(lambda: {v: list() for v in players})
    for number, game in enumerate(games):
        for player in players:
            scores[player].append(0)
            positions_dynamic[player].append(0)
            places_dynamic[player].append(0)
            wins_dynamic[player].append(0)
            losses_dynamic[player].append(0)

        fails += game['code'] != 0
        durations.append(game['duration'])
        game_scores = numpy.array(sorted(v['score'] for v in game['results'].values()))
        unique_game_scores = numpy.array(sorted(frozenset(v['score'] for v in game['results'].values()), reverse=True))
        if len(unique_game_scores) == 1:
            draws += 1
            if unique_game_scores[0] == 0:
                zero_draws += 1
        max_score = max(unique_game_scores)
        min_score = min(unique_game_scores)
        if 1 == sum(1 for v in game_scores if v == max_score):
            winner = next(k for k, v in game['results'].items() if v['score'] == max_score)
            wins[winner] += 1
            wins_dynamic[winner][-1] = 1
        if 1 == sum(1 for v in game_scores if v == min_score):
            loser = next(k for k, v in game['results'].items() if v['score'] == min_score)
            losses[loser] += 1
            losses_dynamic[loser][-1] = 1
        for place, score in enumerate(unique_game_scores):
            for k, v in game['results'].items():
                if v['score'] == score:
                    places[place + 1][k] += 1
                    places_dynamic[k][-1] = place + 1
                    places_positions[place + 1][v['position']][k] += 1
                    place_score[k] += get_place_score(place + 1, len(game['results']))
        for k, v in game['results'].items():
            if v['crashed']:
                crashes[k] += 1
            scores[k][-1] = v['score']
            positions[v['position']][k] += 1
            positions_dynamic[k][-1] = v['position']
            games_count[k] += 1
        seeds.add(game['seed'])
        for player, data in game.get('stats', dict()).items():
            for k, v in data.items():
                if isinstance(v, dict):
                    bot_stats[k][player].append(v['secs'] + v['nanos'] / 10**9)
                else:
                    bot_stats[k][player].append(v)
    for k in scores.keys():
        scores[k] = numpy.array(scores[k])
        places_dynamic[k] = numpy.array(places_dynamic[k])
    return dict(
        total_games=len(games),
        draws=draws,
        zero_draws=zero_draws,
        fails=fails,
        unique_seeds=len(seeds),
        min_duration=min(durations),
        median_duration=statistics.median(durations),
        mean_duration=statistics.mean(durations),
        max_duration=max(durations),
        durations=durations,
        players=sorted(players),
        games=games_count,
        place_score=place_score,
        place_score_per_game={k: v / games_count[k] for k, v in place_score.items()},
        wins=wins,
        wins_per_game={k: v / games_count[k] for k, v in wins.items()},
        losses=losses,
        places=places,
        crashes=crashes,
        positions=positions,
        places_positions=places_positions,
        total_score={k: sum(v) for k, v in scores.items()},
        median_score={k: statistics.median(v) for k, v in scores.items()},
        mean_score={k: statistics.mean(v) for k, v in scores.items()},
        stdev_score={k: statistics.stdev(v) for k, v in scores.items()},
        min_score={k: min(v) for k, v in scores.items()},
        max_score={k: max(v) for k, v in scores.items()},
        q95_score={k: numpy.quantile(v, 0.95) for k, v in scores.items()},
        scores_dynamic=scores,
        scores_dynamic_cumsum=cumsums(scores),
        places_dynamic=places_dynamic,
        places_dynamic_cumsum=cumsums(places_dynamic),
        wins_dynamic=wins_dynamic,
        wins_dynamic_cumsum=cumsums(wins_dynamic),
        losses_dynamic=losses_dynamic,
        losses_dynamic_cumsum=cumsums(losses_dynamic),
        positions_dynamic=positions_dynamic,
        seeds=numpy.array(sorted(seeds)),
        **bot_stats,
    )


def get_place_score(place, sides):
    if sides == 2:
        return {1: 2, 2: 0}[place]
    else:
        return {1: 8, 2: 4, 3: 2, 4: 1}[place]


def cumsums(values):
    return {k: numpy.cumsum(v) for k, v in values.items()}


def get_time(path):
    return int(os.path.basename(path).split('.')[-1])


def collect_data(paths):
    for path in paths:
        players_path = os.path.join(path, 'players.json')
        if not os.path.exists(players_path):
            continue
        result_path = os.path.join(path, 'result.json')
        if not os.path.exists(result_path):
            continue
        task_path = os.path.join(path, 'task.json')
        if not os.path.exists(task_path):
            continue
        players_content = read_file(players_path)
        if not players_content:
            continue
        result_content = read_file(result_path)
        if not result_content:
            continue
        task_content = read_file(task_path)
        if not task_content:
            continue
        players = tuple(json.loads(players_content))
        result = parse_result(result_content, players)
        result.update(json.loads(task_content))
        stats_path = os.path.join(path, 'stats.json')
        if os.path.exists(stats_path):
            result['stats'] = helpers.read_json(stats_path)
        yield result


def read_file(path):
    with open(path) as f:
        return f.read()


def parse_result(content, players):
    data = json.loads(content)
    results = {name: get_record(data, index) for index, name in enumerate(players)}
    return dict(results=results, seed=data['seed'])


def get_record(data, index):
    return dict(
        crashed=data['players'][index]['crashed'],
        score=data['results'][index],
        position=index,
    )


if __name__ == '__main__':
    main()
