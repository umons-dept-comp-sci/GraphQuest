#!/usr/bin/env python

import sys


if __name__ == "__main__":
    count = 1
    with open("/home/axel/GitProject/GraphQuest/gquest_core/tests/modules/demofile.txt", "a") as f:
        for sig in map(str.strip, sys.stdin):
            print(sig, flush=True)
            count += 1

        