#!/usr/bin/env python

import sys

if __name__ == "__main__":
    count = 0
    for sig in map(str.strip, sys.stdin):
        # This program does not flush by itself
        # However since gquest will only send one batch of data then close the
        # stdin, it should not cause any issues
        print(sig, flush=False)