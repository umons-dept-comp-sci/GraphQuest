#!/usr/bin/env python

import sys

def get_vertices(signature: str):
    byte_list = list(map(lambda val: ord(val), signature))
    if signature[0] != "~":
        return byte_list[0] - 63
    else:
        return ((byte_list[1] - 63) << 18) | ((byte_list[2] - 63) << 12) | ((byte_list[3] - 63) << 6) | ((byte_list[4] - 63) << 3) 

if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        print(sig, get_vertices(sig))