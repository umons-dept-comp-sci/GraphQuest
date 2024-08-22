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
        signature = line.split(" ")[0]
        signature = signature.split("\n")[0]
        f.write(str(signature))
        res = get_res(signature)
        sys.stdout.write(signature + " " + str(res) + "\n")
        count += 1
    f.write("closing") 
    f.close()

    sys.stdout.write("\n")