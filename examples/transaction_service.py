from transaction_pb2 import *
from transaction_pb2_grpc import *
from google.protobuf import empty_pb2
import grpc

# Init client
channel = grpc.insecure_channel('localhost:50051')
stub = TransactionStub(channel)

# Get transaction
tx_hash = bytes.fromhex(
    "ca097e95155ca7ca1d625893e3070c3ca94ef4093beb97073cd68105b2c965c5")
request = TransactionRequest(tx_hash=tx_hash)
version = stub.Transaction(request).raw_tx
print(version)
