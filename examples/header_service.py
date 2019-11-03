from header_pb2 import *
from header_pb2_grpc import *
from google.protobuf import empty_pb2
import grpc

# Init client
channel = grpc.insecure_channel('[::1]:50051')
stub = HeaderStub(channel)

# Get header
headers_request = HeadersRequest(start_height=3, count=0)
headers = stub.Headers(headers_request).headers
print(headers)
