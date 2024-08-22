#! /usr/bin/python3
import sys
import random



def get_res(sign):
    return random.randint(0,1000)



if __name__=="__main__":
    f = open("tmp.txt", "a")
    count = 0
    sys.stdout.flush()
    for line in sys.stdin:
        f.write(str(line))
        res = get_res(line)
        sys.stdout.write(str(res) + "\n")
        count += 1
    f.write("closing") 
    f.close()

    sys.stdout.write("\n")