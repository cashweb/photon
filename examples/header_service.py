from header_pb2 import *
from header_pb2_grpc import *
from google.protobuf import empty_pb2
import grpc

# Init client
channel = grpc.insecure_channel('[::1]:50051')
stub = HeaderStub(channel)

# Get header
headers_request = HeadersRequest(start_height=33, count=1)
headers = stub.Headers(headers_request)
print(headers)

header_stream = stub.Subscribe(empty_pb2.Empty())
for header in header_stream:
    print(header)
