from utility_pb2 import *
from utility_pb2_grpc import *
from google.protobuf import empty_pb2
import grpc

# Init client
channel = grpc.insecure_channel('localhost:50051')
stub = UtilityStub(channel)

# Get version
version = stub.Version(empty_pb2.Empty()).version
print(version)

# Get banner
banner = stub.Banner(empty_pb2.Empty()).banner
print(banner)
