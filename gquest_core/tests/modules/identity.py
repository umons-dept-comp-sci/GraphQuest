#!/usr/bin/env python

import sys

flush_count = 0
max_flush_count = 100

if __name__ == "__main__":
    count = 1
    for sig in map(str.strip, sys.stdin):
        if flush_count >= max_flush_count-1:
            print(sig, flush=True)
            flush_count = 0
        else:
            print(sig, flush=False)
            flush_count += 1