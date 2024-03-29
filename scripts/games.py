#!/usr/bin/env python3

import click
import collections
import datetime
import os
import os.path
import queue
import random
import runner
import subprocess
import threading
import time
import traceback
import helpers


@click.command()
@click.option('--players', required=True, type=str)
@click.option('--game_type', default='Round1', type=click.Choice((
    'Round1',
    'Round2',
    'Finals',
)))
@click.option('--runner_bin_path', required=True, type=click.Path(exists=True, dir_okay=False))
@click.option('--start_port', default=40010, type=int)
@click.option('--workers', default=1, type=int)
@click.option('--max_runs', default=2**64 - 1, type=int)
@click.option('--prefix', default='default')
@click.option('--output_path', default=os.path.join(os.getcwd(), 'results/new'), type=click.Path(dir_okay=True, file_okay=False))
@click.option('--verbose', is_flag=True)
@click.option('--timeout', default=120, type=int)
@click.option('--seed', default=None, type=int)
@click.option('--config_path', default=None, type=click.Path(exists=True, dir_okay=False))
@click.option('--visual', is_flag=False)
@click.option('--sides', default=4, type=int)
def main(**kwargs):
    run(**kwargs)


def run(players, game_type, runner_bin_path, start_port, workers, max_runs, prefix, output_path,
        verbose, timeout, seed, config_path, visual, sides):
    players = tuple(parse_players(text=players, start_port=start_port))
    assert len(players) == len({v.start_port if v.name is None else v.name for v in players}), f'{[v.name for v in players]}'
    session = f"{prefix}.{game_type}.{format_players(players)}.{start_port}.{datetime.datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}"
    games_path = os.path.join(output_path, game_type, session)
    scheduler = Scheduler(workers_number=workers, ports_per_worker=len(players), verbose=verbose, timeout=timeout)
    scheduler.start()
    while len(players) < sides:
        players += players
    for number in range(max_runs):
        if verbose:
            print(f'{max_runs - number - 1} tasks is left')
        scheduler.put_task(Task(
            runner=Runner(
                bin_path=runner_bin_path,
                game_type=game_type,
                seed=random.randint(0, 2**64 - 1) if seed is None else seed,
                output_path=os.path.join(games_path, '%s.%s' % (number, int(time.time() * 1e6))),
                config_path=config_path,
                visual=visual,
            ),
            players=random.sample(players, sides),
        ))
    if verbose:
        print('No more new tasks')
    scheduler.join()


def parse_players(text, start_port):
    for n, v in enumerate(text.split(' ')):
        params = v.split(':')
        player_type = None
        if len(params) >= 1:
            player_type = params[0]
        player_bin_path = None
        if player_type == 'Tcp':
            if len(params) >= 2:
                player_bin_path = params[1]
            player_name_index = 2
        else:
            player_name_index = 1
        player_name = None
        if len(params) >= player_name_index + 1:
            player_name = params[player_name_index]
        assert player_type in {'Tcp', 'QuickStart', 'Empty'}, player_type
        if player_bin_path is not None:
            assert os.path.exists(player_bin_path), player_bin_path
            assert os.path.isfile(player_bin_path), player_bin_path
        yield Player(type=player_type, bin_path=player_bin_path, start_port=start_port + n, name=player_name)


def format_players(players):
    return '.'.join(v.type for v in players)


Player = collections.namedtuple('Player', (
    'type',
    'bin_path',
    'start_port',
    'name',
))


Task = collections.namedtuple('Task', (
    'runner',
    'players',
))


Runner = collections.namedtuple('Runner', (
    'bin_path',
    'game_type',
    'seed',
    'output_path',
    'config_path',
    'visual',
))


Worker = collections.namedtuple('Worker', (
    'thread',
    'stop',
))


class Scheduler:
    def __init__(self, workers_number, ports_per_worker, verbose, timeout):
        self.__task_queue = queue.Queue(maxsize=workers_number)
        workers = list()
        for n in range(workers_number):
            stop = threading.Event()
            workers.append(
                Worker(
                    thread=threading.Thread(
                        target=run_worker,
                        kwargs=dict(
                            task_queue=self.__task_queue,
                            port_shift=n * ports_per_worker,
                            stop=stop,
                            verbose=verbose,
                            timeout=timeout,
                        )
                    ),
                    stop=stop,
                )
            )
        self.__workers = workers
        self.__ports_per_worker = ports_per_worker
        self.__verbose = verbose
        self.__timeout = timeout

    def start(self):
        for worker in self.__workers:
            worker.thread.start()

    def put_task(self, task):
        put = False
        while not put:
            try:
                for n, worker in enumerate(self.__workers):
                    if worker.stop.is_set():
                        port_shift = n * self.__ports_per_worker
                        if self.__verbose:
                            print(f'Worker {port_shift} is crashed, rerunning...')
                        stop = threading.Event()
                        worker = Worker(
                            thread=threading.Thread(
                                target=run_worker,
                                kwargs=dict(
                                    task_queue=self.__task_queue,
                                    port_shift=port_shift,
                                    stop=stop,
                                    verbose=self.__verbose,
                                    timeout=self.__timeout,
                                )
                            ),
                            stop=stop,
                        )
                        worker.thread.start()
                        self.__workers[n] = worker
                self.__task_queue.put(task, timeout=1)
                put = True
            except queue.Full:
                pass

    def join(self):
        joined = False
        self.__task_queue.join()
        while not joined:
            try:
                for worker in self.__workers:
                    worker.stop.set()
                for worker in self.__workers:
                    worker.thread.join()
                joined = True
            except:
                traceback.print_exc()


def run_worker(task_queue, port_shift, stop, verbose, timeout):
    if verbose:
        print(f'Worker {port_shift} is started')
    while not stop.is_set():
        try:
            task = task_queue.get(timeout=1)
            if task is None:
                continue
            try:
                handle_task(task=task, port_shift=port_shift, verbose=verbose, stop=stop, timeout=timeout)
            except:
                traceback.print_exc()
            finally:
                task_queue.task_done()
        except queue.Empty:
            pass
    if verbose:
        print(f'Worker {port_shift} is finished')


def handle_task(task, port_shift, verbose, stop, timeout):
    if verbose:
        print(f'Handle {task} by worker {port_shift}, ports: {[v.start_port + port_shift for v in task.players]}')
    task_path = os.path.join(task.runner.output_path, 'task.json')
    start = time.time()
    runner_process = runner.run_game(
        player_ports=[(v.type, v.start_port + port_shift) for v in task.players],
        player_names=[format_player_name(v) if v.name is None else v.name for v in task.players],
        game_type=task.runner.game_type,
        bin_path=task.runner.bin_path,
        verbose=verbose,
        output_path=task.runner.output_path,
        seed=task.runner.seed,
        visual=task.runner.visual,
    )
    player_workers = list()
    for player in task.players:
        if player.bin_path is not None:
            stop_worker = threading.Event()
            player_workers.append(Worker(
                thread=threading.Thread(
                    target=run_player,
                    kwargs=dict(
                        bin_path=player.bin_path,
                        port=player.start_port + port_shift,
                        verbose=verbose,
                        stop=stop_worker,
                        timeout=timeout,
                        config_path=task.runner.config_path,
                        stats_path=os.path.join(task.runner.output_path, f'{player.start_port}.stats.json'),
                    ),
                ),
                stop=stop_worker,
            ))
    player_workers = tuple(player_workers)
    time.sleep(0.2)
    for worker in player_workers:
        worker.thread.start()
    wait_process(process=runner_process, stop=stop, timeout=timeout + 1, verbose=verbose)
    duration = time.time() - start
    if verbose:
        print(f'Runner is finished with {runner_process.returncode} by worker {port_shift} in {duration}s')
    helpers.write_json(data=dict(duration=duration, code=runner_process.returncode), path=task_path)
    if runner_process.returncode is not None and runner_process.returncode != 0:
        stop.set()
    if stop.is_set():
        for worker in player_workers:
            worker.stop.set()
    for worker in player_workers:
        worker.thread.join()
    stats = dict()
    for player in task.players:
        stats_path = os.path.join(task.runner.output_path, f'{player.start_port}.stats.json')
        player_name = format_player_name(player) if player.name is None else player.name
        if os.path.exists(stats_path):
            stats[player_name] = helpers.read_json(stats_path)
        else:
            stats[player_name] = dict()
    helpers.write_json(stats, os.path.join(task.runner.output_path, 'stats.json'))


def format_player_name(player):
    if player.bin_path is None:
        return f'{player.type}:{player.start_port}'
    return f'{player.type}:{player.start_port}:{os.path.split(player.bin_path)[-1]}'


def run_player(bin_path, port, verbose, stop, timeout, config_path, stats_path):
    env = os.environ.copy()
    env['RUST_BACKTRACE'] = '1'
    if config_path is not None:
        env['CONFIG'] = str(config_path)
    env['STATS'] = str(stats_path)
    args = [os.path.abspath(bin_path), '127.0.0.1', str(port)]
    fails = 0
    while fails < 3 and not stop.is_set():
        try:
            if verbose:
                print('Run', *args)
            process = subprocess.Popen(
                env=env,
                args=args,
                stdout=None if verbose else subprocess.DEVNULL,
                stderr=None if verbose else subprocess.DEVNULL,
            )
            if not wait_process(process=process, stop=stop, timeout=timeout, verbose=verbose):
                break
            if process.returncode == 0:
                return
            if process.returncode == -2:
                break
            fails += 1
            time.sleep(min(1.0, fails * 0.1))
        except subprocess.TimeoutExpired:
            break
        except:
            traceback.print_exc()
            break
    if verbose:
        print(f'Worker {port} has failed')


def wait_process(process, stop, timeout, verbose):
    if verbose:
        print('Wait', *process.args)
    start = time.time()
    while time.time() - start < timeout and not stop.is_set():
        try:
            process.wait(timeout=0.1)
            if verbose:
                print('Success', *process.args)
            return True
        except subprocess.TimeoutExpired:
            pass
    process.terminate()
    if verbose:
        print('Fail', *process.args)
    return False


if __name__ == "__main__":
    main()
