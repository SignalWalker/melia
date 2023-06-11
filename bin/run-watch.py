#! /usr/bin/env python3

import argparse
import sys
import tempfile
import subprocess
import os
from enum import IntEnum, auto
import atexit

# for pretty printing to stderr
class Color:
    RED = '\033[31m'
    YELLOW = '\033[33m'
    GREEN = '\033[32m'
    BLUE = '\033[34m'
    PURPLE = '\033[35m'
    GREY = '\033[37m'
    ENDC = '\033[m'

class LogLevel(IntEnum):
    OFF = 0
    ERROR = 1
    WARNING = 2
    INFO = 3
    DEBUG = 4
    TRACE = 5

def eprint(msg: str, level: LogLevel = LogLevel.INFO, **kwargs):
	color = Color.ENDC
	if level == LogLevel.ERROR: color = Color.RED
	elif level == LogLevel.WARNING: color = Color.YELLOW
	elif level == LogLevel.INFO: color = Color.GREEN
	elif level == LogLevel.DEBUG: color = Color.BLUE
	elif level == LogLevel.TRACE: color = Color.PURPLE
	print(f"{color}[run-watch]{Color.ENDC} {msg}", file=sys.stderr, **kwargs)

def main() -> int:
	parser = argparse.ArgumentParser()
	parser.add_argument('-s', '--sockets', type=str, nargs='*', default=[])
	args = parser.parse_args()

	with tempfile.NamedTemporaryFile() as watch_trigger:

		eprint(f"watch trigger: {watch_trigger.name}")

		check_ignores = ["bin", "*.nix", "flake.lock", "justfile", "*.md"]
		check_cmd = ["cargo-watch", "--why", "-x", "check", "-s", f"touch {watch_trigger.name}"]
		for ignore in check_ignores:
			check_cmd += ["-i", ignore]

		if "MELIA_CTL_SOCKET" in os.environ:
			sock = os.environ["MELIA_CTL_SOCKET"]
			eprint(f"found $MELIA_CTL_SOCKET: {sock}")
			args.sockets.append("unix::" + os.environ["MELIA_CTL_SOCKET"])

		systemfd_cmd = ["systemfd", "--no-pid"]
		for sock in args.sockets:
			systemfd_cmd += ["-s", sock]
		systemfd_cmd += ["--", "cargo-watch", "--postpone", "-w", watch_trigger.name, "-x", "run -- daemon"]

		eprint(f"check cmd: {check_cmd}", LogLevel.DEBUG)
		eprint(f"systemfd cmd: {systemfd_cmd}", LogLevel.DEBUG)

		eprint(f"running check...", LogLevel.TRACE)
		watch_check = subprocess.Popen(check_cmd)
		eprint(f"running systemfd...", LogLevel.TRACE)
		watch_run = subprocess.Popen(systemfd_cmd)

		eprint(f"waiting for systemfd...", LogLevel.TRACE)
		watch_run.wait()
		eprint(f"killing check...", LogLevel.TRACE)
		watch_check.kill()

	eprint("done")
	return 0

if __name__ == '__main__':
	sys.exit(main())
