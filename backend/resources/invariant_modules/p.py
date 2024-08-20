#! /usr/bin/python3
import sys
import random

MAX_BUFFER = 10000

def get_res(sign):
    return random.randint(0,1000)



if __name__=="__main__":
    list1 = []
    for line in sys.stdin:
        list1.append(line.strip("\n"))
        if len(list1) != MAX_BUFFER:
            for args in list1:
                res = get_res(args)
                sys.stdout.write(str(res) + ";")
            list1.clear()
    sys.stdout.write("\n")

