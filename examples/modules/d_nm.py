#! /usr/bin/env python

import sys
from math import floor, sqrt

if __name__ == "__main__":
    for n,m in map(str.split, map(str.strip, sys.stdin)):
        m = int(m)
        n = int(n)

        sq_content = 17 + 8 * (m - n)
        # if sq_content < 0:
        #     print(n, m, -1, flush=True)
        # else:
        d_nm = floor((2* n + 1 - sqrt(sq_content)) /2 )
                    
        print(n, m, d_nm, flush=True)
