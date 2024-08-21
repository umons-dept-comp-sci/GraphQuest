#! /usr/bin/python3
import sys
import random

MAX_BUFFER = 10000

def get_res(sign):
    return random.randint(0,1000)



if __name__=="__main__":
    f = open("tmp.txt", "w")
    list1 = []
    for line in sys.stdin:
        list1.append(line.strip("\n"))
        if len(list1) != MAX_BUFFER:
            f.write(str(list1))
            for args in list1:
                
                res = get_res(args)
                sys.stdout.write(str(res) + "\n")
            list1.clear()
    f.close()
    sys.stdout.write("\n")