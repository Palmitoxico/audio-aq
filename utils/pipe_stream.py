#!/usr/bin/env python3

import serial
import sys
import string

base64_table = bytearray(string.ascii_uppercase + string.ascii_lowercase + string.digits + '+/', encoding = "UTF-8")
base64_dec_dic = dict(zip(base64_table, range(64)))

try:
    portname = sys.argv[1]
    lines_to_read = int(sys.argv[2])
    sample_rate = int(sys.argv[3])
except:
    print("Usage: " + sys.argv[0] + " serial_port_name lines_to_read sample_rate")
    exit(1)

ser = serial.Serial()

ser.port = portname
ser.open()

sample_rate_str = str(sample_rate) + "\n"
ser.write(sample_rate_str.encode(encoding='UTF-8'))
ser.write(sample_rate_str.encode(encoding='UTF-8'))

buffer_out = bytearray()

line = ser.readline()

if lines_to_read < 0:
    while (1):
        line = ser.readline().rstrip()
        index = 0
        while (index < len(line)):
            sample = base64_dec_dic[line[index]] | (base64_dec_dic[line[index + 1]] << 6) - 2048
            sample = sample * 4
            byte_low = sample & 0xFF
            byte_high = (sample >> 8) & 0xFF
            buffer_out.append(byte_low)
            buffer_out.append(byte_high)
            index += 2
            sys.stdout.buffer.write(buffer_out)
            buffer_out.clear()

for i in range(lines_to_read):
    line = ser.readline().rstrip()
    index = 0
    while (index < len(line)):
        sample = base64_dec_dic[line[index]] | (base64_dec_dic[line[index + 1]] << 6) - 2048
        byte_low = sample & 0xFF
        byte_high = (sample >> 8) & 0xFF
        buffer_out.append(byte_low)
        buffer_out.append(byte_high)
        index += 2
    sys.stdout.buffer.write(buffer_out)
    buffer_out.clear()
