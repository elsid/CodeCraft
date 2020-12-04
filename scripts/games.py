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
def main(**kwargs):
    run(**kwargs)


def run(players, game_type, runner_bin_path, start_port, workers, max_runs, prefix, output_path, verbose, timeout):
    players = tuple(parse_players(players, start_port, workers))
    session = f"{prefix}.{game_type}.{format_players(players)}.{start_port}.{datetime.datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}"
    games_path = os.path.join(output_path, format_players(players), game_type, session)
    scheduler = Scheduler(workers_number=workers, verbose=verbose, timeout=timeout)
    scheduler.start()
    for number in range(max_runs):
        scheduler.put_task(Task(
            runner=Runner(
                bin_path=runner_bin_path,
                game_type=game_type,
                seed=random.randint(0, 2**64 - 1),
                output_path=os.path.join(games_path, '%s.%s' % (number, int(time.time() * 1e6))),
            ),
            players=players,
        ))
    if verbose:
        print('No more new tasks')
    scheduler.join()


def parse_players(text, start_port, workers):
    for n, v in enumerate(text.split(' ')):
        params = v.split(':')
        player_type = None
        if len(params) >= 1:
            player_type = params[0]
        player_bin_path = None
        if len(params) >= 2:
            player_bin_path = params[1]
        assert player_type in {'Tcp', 'QuickStart', 'Empty'}, player_type
        if player_bin_path is not None:
            assert os.path.exists(player_bin_path), player_bin_path
            assert os.path.isfile(player_bin_path), player_bin_path
        yield Player(type=player_type, bin_path=player_bin_path, start_port=start_port + n)


def format_players(players):
    return '.'.join(v.type for v in players)


Player = collections.namedtuple('Player', (
    'type',
    'bin_path',
    'start_port',
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
))


Worker = collections.namedtuple('Worker', (
    'thread',
    'stop',
))


class Scheduler:
    def __init__(self, workers_number, verbose, timeout):
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
                            port_shift=n * workers_number,
                            stop=stop,
                            verbose=verbose,
                            timeout=timeout,
                        )
                    ),
                    stop=stop,
                )
            )
        self.__workers = tuple(workers)

    def start(self):
        for worker in self.__workers:
            worker.thread.start()

    def put_task(self, task):
        put = False
        while not put:
            try:
                for worker in self.__workers:
                    if worker.stop.is_set():
                        raise RuntimeError('Worker is stopped')
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
                if verbose:
                    print(f'{task_queue.qsize()} tasks is left')
        except queue.Empty:
            pass
    if verbose:
        print(f'Worker {port_shift} is finished')


def handle_task(task, port_shift, verbose, stop, timeout):
    if verbose:
        print(f'Handle {task} by worker {port_shift}')
    task_path = os.path.join(task.runner.output_path, 'task.json')
    start = time.time()
    runner_process = runner.run_game(
        player_ports=[(v.type, v.start_port + port_shift) for v in task.players],
        game_type=task.runner.game_type,
        bin_path=task.runner.bin_path,
        verbose=verbose,
        output_path=task.runner.output_path,
        seed=task.runner.seed,
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
                    ),
                ),
                stop=stop_worker,
            ))
    player_workers = tuple(player_workers)
    time.sleep(0.2)
    for worker in player_workers:
        worker.thread.start()
    wait_process(process=runner_process, stop=stop, timeout=timeout, verbose=verbose)
    duration = time.time() - start
    if verbose:
        print(f'Runner is finished with {runner_process.returncode} by worker {port_shift} in {duration}s')
    helpers.write_json(data=dict(duration=duration, code=runner_process.returncode), path=task_path)
    if runner_process.returncode != 0:
        stop.set()
    if stop.is_set():
        for worker in player_workers:
            worker.stop.set()
    for worker in player_workers:
        worker.thread.join()


def run_player(bin_path, port, verbose, stop, timeout):
    args = [os.path.abspath(bin_path), '127.0.0.1', str(port)]
    fails = 0
    while fails < 3 and not stop.is_set():
        try:
            if verbose:
                print('Run', *args)
            process = subprocess.Popen(
                args=args,
                stdout=None if verbose else subprocess.DEVNULL,
                stderr=None if verbose else subprocess.DEVNULL,
            )
            if not wait_process(process=process, stop=stop, timeout=timeout, verbose=verbose):
                break
            if process.returncode == 0 or process.returncode == -2:
                break
            fails += 1
            time.sleep(min(1.0, fails * 0.1))
        except subprocess.TimeoutExpired:
            break
        except:
            traceback.print_exc()
            break


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