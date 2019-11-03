# Generated by the Protocol Buffers compiler. DO NOT EDIT!
# source: protos/header.proto
# plugin: grpclib.plugin.main
import abc
import typing

import grpclib.const
import grpclib.client
if typing.TYPE_CHECKING:
    import grpclib.server

import google.protobuf.empty_pb2
import protos.header_pb2


class HeaderBase(abc.ABC):

    @abc.abstractmethod
    async def Headers(self, stream: 'grpclib.server.Stream[protos.header_pb2.HeadersRequest, protos.header_pb2.HeadersResponse]') -> None:
        pass

    @abc.abstractmethod
    async def Subscribe(self, stream: 'grpclib.server.Stream[google.protobuf.empty_pb2.Empty, protos.header_pb2.SubscribeResponse]') -> None:
        pass

    def __mapping__(self) -> typing.Dict[str, grpclib.const.Handler]:
        return {
            '/header.Header/Headers': grpclib.const.Handler(
                self.Headers,
                grpclib.const.Cardinality.UNARY_UNARY,
                protos.header_pb2.HeadersRequest,
                protos.header_pb2.HeadersResponse,
            ),
            '/header.Header/Subscribe': grpclib.const.Handler(
                self.Subscribe,
                grpclib.const.Cardinality.UNARY_STREAM,
                google.protobuf.empty_pb2.Empty,
                protos.header_pb2.SubscribeResponse,
            ),
        }


class HeaderStub:

    def __init__(self, channel: grpclib.client.Channel) -> None:
        self.Headers = grpclib.client.UnaryUnaryMethod(
            channel,
            '/header.Header/Headers',
            protos.header_pb2.HeadersRequest,
            protos.header_pb2.HeadersResponse,
        )
        self.Subscribe = grpclib.client.UnaryStreamMethod(
            channel,
            '/header.Header/Subscribe',
            google.protobuf.empty_pb2.Empty,
            protos.header_pb2.SubscribeResponse,
        )
