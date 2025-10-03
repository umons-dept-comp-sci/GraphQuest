#!/usr/bin/env python

import sys


if __name__ == "__main__":
    count = 1
    with open("/home/axel/GitProject/GraphQuest/gquest_core/tests/modules/demofile.txt", "a") as f:
        for sig in map(str.strip, sys.stdin):
            print("i = ", count, " : ", sig, flush=True)
            # f.write(sig + "\n")
            # f.flush()
            # if count % 31 == 0:
            # if count % 1 == 0:
            #     0 / 0
            count += 1

        