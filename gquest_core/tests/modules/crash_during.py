#!/usr/bin/env python

import sys

if __name__ == "__main__":
    count = 0
    for sig in map(str.strip, sys.stdin):
        print(sig, sig, flush=True)
        if count == 5: 
            raise ZeroDivisionError(f"Error raised after {count} signatures")
        count += 1