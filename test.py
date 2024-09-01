import os
import socket
import random
import json
import zlib
import gzip

PORT = os.environ.get('PORT', 12201)
HOST = os.environ.get('HOST', '127.0.0.1')
ADDRESS = (HOST, PORT)
MAX_SIZE = 3000
SOCKET = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

def generate_gelf_chunk(message_id, message, seq_num, total_seq):
    return b''.join([
        b'\x1e\x0f',
        message_id,
        seq_num.to_bytes(1, 'big'),
        total_seq.to_bytes(1, 'big'),
        message
    ])

def generate_gelf_chunks(gelf_payload):
    message_id = os.urandom(8)
    chunks = [gelf_payload[i:i+MAX_SIZE] for i in range(0, len(gelf_payload), MAX_SIZE)]
    total_seq = len(chunks)
    chunks = [generate_gelf_chunk(message_id, chunk, i, total_seq) for i, chunk in enumerate(chunks)]
    # Randomly shuffle the chunks
    random.shuffle(chunks)
    return chunks

def generate_gelf_payload(message):
    return json.dumps({
      "version": "1.1",
      "host": "example.org",
      "short_message": message,
      "full_message": "There is no full message bro",
      "timestamp": 1385053862.3072,
      "level": 1,
      "_user_id": 9001,
      "_some_info": "foo",
      "_some_env_var": "bar"
    })


def generate_compressed_gelf_payload(message,compression):
    payload = generate_gelf_payload(message)
    if compression == "zlib":
        return zlib.compress(payload.encode())
    elif compression == "gzip":
        return gzip.compress(payload.encode())
    elif compression == None:
        return payload.encode()
    else:
        raise ValueError("Invalid compression type")


def send_message(message,compression):
    gelf_payload = generate_compressed_gelf_payload(message,compression)
    chunks = generate_gelf_chunks(gelf_payload)
    for chunk in chunks:
        SOCKET.sendto(chunk, ADDRESS)

    print(f"Sent {len(chunks)} chunks")

def main():
    small_message = "This is a small message" *100
    large_message = "This is a large message" * 500
    very_large_message = "This is a very large message" * 10000

    # send_message(small_message,None)
    # send_message(small_message,None)
    # send_message(large_message,None)
    # send_message(very_large_message,none)
    #
    zlib_message = "This is a zlib message" * 100
    gzip_message = "This is a gzip message" * 100

    send_message(zlib_message,"zlib")
    # send_message(gzip_message,"gzip")

if __name__ == '__main__':
    main()
