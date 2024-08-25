#! /usr/bin/python3
import sys
import random



def get_res(sign):
    return random.randint(0,1000)



if __name__=="__main__":
    
    count = 0
    sys.stdout.flush()
    for line in sys.stdin:
        signature = line.split(" ")[0]
        signature = signature.split("\n")[0]
        
        res = "5"
        sys.stdout.write(signature + " " + res + "\n")
        count += 1
    
    
    sys.stdout.write("\n")