#!/usr/bin/env python

import sys


if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        print(sig)