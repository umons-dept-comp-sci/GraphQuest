#! /usr/bin/env python

import sys
from math import floor, sqrt

def d_nm(n:int,m:int):
    sq_content = 17 + 8 * (m - n)
    return floor((2* n + 1 - sqrt(sq_content)) /2 )

if __name__ == "__main__":
    for n,m in map(str.split, map(str.strip, sys.stdin)):
        m = int(m)
        n = int(n)
        print(n, m, d_nm(n,m), flush=True)
