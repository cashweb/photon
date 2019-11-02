# Generated by the Protocol Buffers compiler. DO NOT EDIT!
# source: protos/script_hash.proto
# plugin: grpclib.plugin.main
import abc
import typing

import grpclib.const
import grpclib.client
if typing.TYPE_CHECKING:
    import grpclib.server

import protos.script_hash_pb2


class ScriptHashBase(abc.ABC):

    @abc.abstractmethod
    async def Balance(self, stream: 'grpclib.server.Stream[protos.script_hash_pb2.BalanceRequest, protos.script_hash_pb2.BalanceResponse]') -> None:
        pass

    @abc.abstractmethod
    async def History(self, stream: 'grpclib.server.Stream[protos.script_hash_pb2.HistoryRequest, protos.script_hash_pb2.HistoryResponse]') -> None:
        pass

    @abc.abstractmethod
    async def ListUnspent(self, stream: 'grpclib.server.Stream[protos.script_hash_pb2.ListUnspentRequest, protos.script_hash_pb2.ListUnspentResponse]') -> None:
        pass

    @abc.abstractmethod
    async def Subscribe(self, stream: 'grpclib.server.Stream[protos.script_hash_pb2.SubscribeRequest, protos.script_hash_pb2.SubscribeResponse]') -> None:
        pass

    def __mapping__(self) -> typing.Dict[str, grpclib.const.Handler]:
        return {
            '/script_hash.ScriptHash/Balance': grpclib.const.Handler(
                self.Balance,
                grpclib.const.Cardinality.UNARY_UNARY,
                protos.script_hash_pb2.BalanceRequest,
                protos.script_hash_pb2.BalanceResponse,
            ),
            '/script_hash.ScriptHash/History': grpclib.const.Handler(
                self.History,
                grpclib.const.Cardinality.UNARY_UNARY,
                protos.script_hash_pb2.HistoryRequest,
                protos.script_hash_pb2.HistoryResponse,
            ),
            '/script_hash.ScriptHash/ListUnspent': grpclib.const.Handler(
                self.ListUnspent,
                grpclib.const.Cardinality.UNARY_UNARY,
                protos.script_hash_pb2.ListUnspentRequest,
                protos.script_hash_pb2.ListUnspentResponse,
            ),
            '/script_hash.ScriptHash/Subscribe': grpclib.const.Handler(
                self.Subscribe,
                grpclib.const.Cardinality.UNARY_STREAM,
                protos.script_hash_pb2.SubscribeRequest,
                protos.script_hash_pb2.SubscribeResponse,
            ),
        }


class ScriptHashStub:

    def __init__(self, channel: grpclib.client.Channel) -> None:
        self.Balance = grpclib.client.UnaryUnaryMethod(
            channel,
            '/script_hash.ScriptHash/Balance',
            protos.script_hash_pb2.BalanceRequest,
            protos.script_hash_pb2.BalanceResponse,
        )
        self.History = grpclib.client.UnaryUnaryMethod(
            channel,
            '/script_hash.ScriptHash/History',
            protos.script_hash_pb2.HistoryRequest,
            protos.script_hash_pb2.HistoryResponse,
        )
        self.ListUnspent = grpclib.client.UnaryUnaryMethod(
            channel,
            '/script_hash.ScriptHash/ListUnspent',
            protos.script_hash_pb2.ListUnspentRequest,
            protos.script_hash_pb2.ListUnspentResponse,
        )
        self.Subscribe = grpclib.client.UnaryStreamMethod(
            channel,
            '/script_hash.ScriptHash/Subscribe',
            protos.script_hash_pb2.SubscribeRequest,
            protos.script_hash_pb2.SubscribeResponse,
        )
