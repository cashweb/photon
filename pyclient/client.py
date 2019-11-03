#!/usr/bin/env python3
#
# Python Photon Client, WIP
#
# Requires grpclib:
#   $ pip3 install --user grpclib protobuf
# To compile protobuf files, you can use protoc from the grpcio-tools package:
#   $ pip3 install --user grpcio-tools
# To compile the protobuf files for this, copy or link the .proto files to protos/ subdir then:
#   $ python3 -m grpc_tools.protoc -I. --python_out=. --python_grpc_out=. protos/whatever.proto
#
import asyncio
import queue
import sys
import threading
import time

from typing import Any, Callable, Tuple, Union

from grpclib.client import Channel
from grpclib.exceptions import GRPCError, StreamTerminatedError, ProtocolError
from grpclib.const import Status

import google.protobuf.empty_pb2
Empty = google.protobuf.empty_pb2.Empty()

# generated by protoc
from protos import utility_pb2
from protos import utility_grpc
from protos import transaction_pb2
from protos import transaction_grpc

class MalformedResponse(Exception):
    ''' Thrown if the response from the server violates a guarantee
    or is otherwise nonsensical. '''

#### Utility functions (these are workalikes from Electron Cash codebase)
import hashlib
import itertools

def Sha256(x: bytes) -> bytes:
    return hashlib.sha256(x).digest()

def to_bytes(something: Union[str, bytes, bytearray], encoding='utf8') -> bytes:
    if isinstance(something, bytes):
        return something
    if isinstance(something, str):
        return something.encode(encoding)
    elif isinstance(something, bytearray):
        return bytes(something)
    else:
        raise TypeError("Not a string or bytes like object")

def Hash(x: Union[bytes, str, bytearray]) -> bytes:
    x = to_bytes(x, 'utf8')
    out = Sha256(Sha256(x))
    return out
def RHash(x: Union[bytes, str, bytearray]) -> bytes:
    ''' Reversed Sha256d (a-la bitcoin) '''
    return Hash(x)[::-1]

class Monotonic:
    ''' Returns a monotonically increasing int each time an instance is called
    as a function. Optionally thread-safe.'''
    __slots__ = ('__call__',)
    def __init__(self, locking=False, start=1):
        counter = itertools.count(start)
        self.__call__ = incr = lambda: next(counter)
        if locking:
            lock = threading.Lock()
            def incr_with_lock():
                with lock: return incr()
            self.__call__ = incr_with_lock

# /Utility functions


class Client:
    DEFAULT_HOST = '127.0.0.1'
    DEFAULT_PORT = 50051  # fixme
    trace = False
    id_next = Monotonic(locking=True)

    def __init__(self, host: str = DEFAULT_HOST, port: int = DEFAULT_PORT, ssl: bool = False,
                 *, logger: Callable = None, dont_raise_on_error=False, trace=None):
        self.logger = logger or (lambda *args, **kwargs: None)
        self.host_port = (host, port)
        self.ssl = ssl
        self.thr = None
        self.utility: utility_grpc.UtilityStub = None
        self.transaction: transaction_grpc.TransactionStub = None
        self.loop = None
        self.id = self.id_next()
        self.dont_raise_on_error = dont_raise_on_error
        if trace is not None: self.trace = trace  # specified instance-specific trace setting (otherwise we inherit class-level)

    def start(self):
        if (self.thr and self.thr.is_alive()) or (self.loop and self.loop.is_running()):
            raise RuntimeError('Already running')
        self.loop = asyncio.new_event_loop()
        self.thr = threading.Thread(target=self._thrdFunc, daemon=True,
                                    name=f"{__class__.__name__} {self.id} {':'.join([str(x) for x in self.host_port])} Async Thr")
        self.thr.q = queue.Queue()
        self.thr.start()
        exc = self.thr.q.get(timeout=10.0)  # wait for thread to start before we return
        del self.thr.q
        if isinstance(exc, Exception):
            raise exc
        return True

    def stop(self):
        if not self.thr:
            return False
        if self.loop.is_running():
            self.logger("Stopping loop...")
            for task in asyncio.Task.all_tasks(self.loop):
                task.cancel()
            self.loop.call_soon_threadsafe(self.loop.stop)
        if self.thr.is_alive():
            self.logger("Waiting for thread...")
            self.thr.join()
        self.loop = self.thr = None
        return True

    def is_running(self):
        return bool(self.loop and self.loop.is_running())

    def _thrdFunc(self):
        t0 = time.time()
        try:
            self.logger("thread started: ", threading.current_thread().name)
            asyncio.set_event_loop(self.loop)
            channel = Channel(*self.host_port, loop=self.loop, ssl=self.ssl)
            self.utility = utility_grpc.UtilityStub(channel)
            self.transaction = transaction_grpc.TransactionStub(channel)
            self.thr.q.put(True)
            self.loop.run_forever()
        except Exception as e:
            self.thr.q.put(e)
        finally:
            self.logger("Closing channel...")
            channel.close()
            self.logger("Shutting down asyncgens...")
            self.loop.run_until_complete(self.loop.shutdown_asyncgens())
            self.loop.close()
            if self.utility:
                self.utility = None
            if self.transaction:
                self.transaction = None
            self.logger(f"thread '{threading.current_thread().name}' exiting after running for {time.time()-t0:.3f} seconds")

    # CAUTION --
    #   The below methods should be run in this event loop in this class's thread.
    #   They aren't to be scheduled from code that isn't excuting in this class's thread
    #   and that isn't using this class's event loop.
    #
    async def Version(self, useragent: str, version: str) -> Tuple[str, str]:
        pb = utility_pb2
        if self.trace: self.logger(f"--> Sending utility.Version...")
        reply: pb.VersionResponse = await self.utility.Version(pb.VersionRequest(agent=useragent, version=version))
        if self.trace: self.logger(f"<-- Got utility.Version:", reply.agent, reply.version)
        return (reply.agent, reply.version)

    async def Banner(self) -> str:
        pb = utility_pb2
        if self.trace: self.logger(f"--> Sending utility.Banner...")
        reply: pb.BannerResponse = await self.utility.Banner(Empty)
        if self.trace: self.logger(f"<-- Got utility.Banner:", reply.banner[:80]+("..." if len(reply.banner) > 80 else ""))
        return reply.banner

    async def Ping(self) -> float:
        pb = utility_pb2
        if self.trace: self.logger(f"--> Sending utility.Ping...")
        t0 = time.time()
        await self.utility.Ping(Empty)
        elapsed = time.time()-t0
        if self.trace: self.logger(f"<-- Got utility.Ping reply in {1e3*elapsed:1.3f} msec")
        return elapsed

    async def DonationAddress(self) -> str:
        pb = utility_pb2
        if self.trace: self.logger(f"--> Sending utility.DonationAddress ...")
        reply: pb.DonationAddressResponse = await self.utility.DonationAddress(Empty)
        addr = reply.address or '' # is this needed?
        if self.trace: self.logger(f"<-- Got utility.DonationAddress:", addr[:80]+("..." if len(addr) > 80 else ""))
        return reply.address

    async def Transaction(self, tx_hash: bytes) -> dict:
        pb = transaction_pb2
        if self.trace: self.logger(f"--> Sending transaction.Transaction('{tx_hash[:4].hex()}..{tx_hash[-4:].hex()}') ...")
        request = pb.TransactionRequest(tx_hash=tx_hash)
        reply: pb.TransactionResponse = await self.transaction.Transaction(request)
        tx_hash_server = reply.raw_tx and RHash(reply.raw_tx)
        if tx_hash_server != tx_hash:
            raise MalformedResponse('raw_tx data from server does not match the requested txid',
                                    tx_hash_server.hex(), tx_hash.hex())
        if self.trace: self.logger(f"<-- Got transaction.Transaction: {len(reply.raw_tx)} bytes ...")
        merkle_dict = dict()
        if reply.merkle:
            merkle_dict['block_height'] = reply.merkle.block_height
            merkle_dict['merkle'] = reply.merkle.merkle
            merkle_dict['pos'] = reply.merkle.pos
        return { 'raw_tx': reply.raw_tx, 'merkle': merkle_dict }


    async def Sleeper(self, delay=5.0) -> int:
        await asyncio.sleep(delay)
        import random
        return random.randint(1, 1024)

    #
    # PRIVATE ---
    #
    def _do_sync(self, coro: asyncio.Future, *, timeout=10.0, dont_raise_on_error=None, trace=None) -> Any:
        ''' Helper: schedules a coroutine to run in this class's thread, on its event loop. Thread safe,
        intended to be called from outside code not running on this class's thread.  Will return the result
        syncrhonously or raise an exception on timeout or if the coroutine raised. Optionally
        `dont_raise_on_error` will suppress any exceptions and None will be returned (this is a debugging
        feature). '''
        future = asyncio.run_coroutine_threadsafe(coro, self.loop)
        dont_raise_on_error = self.dont_raise_on_error if dont_raise_on_error is None else dont_raise_on_error
        trace = trace if trace is not None else self.trace
        try:
            result = future.result(timeout=timeout)
        except asyncio.TimeoutError as e:
            self.logger(f'The "{coro.__name__}" coroutine took too long, cancelling the task...')
            future.cancel()
            if not dont_raise_on_error:
                raise e
        except Exception as e:
            self.logger(f'The "{coro.__name__}" coroutine raised an exception: {e!r}')
            if not dont_raise_on_error:
                raise e
        else:
            if trace:
                self.logger(f'The "{coro.__name__}" coroutine returned: {result!r}')
            return result

    def _do_cb(self, coro : asyncio.Future, callback, *, timeout=10.0) -> None:
        ''' Helper: schedules a coroutine to run in this class's thread, on its event loop. Thread safe.
        Can be called either from outside code not running on this class's thread, or from this class's thread.
        `callback` will be called later when the coroutine completes, passing the return value from the coroutine
        to `callback`. If the coroutine raised, then an exception will be passed to `callback`.
        If a timeout occurred, asyncio.TimeoutError will be passed to `callback`.  '''
        run_coro = (asyncio.ensure_future if threading.current_thread() is self.thr
                    else asyncio.run_coroutine_threadsafe)
        future = run_coro(coro, loop=self.loop)
        timer = None
        def done_cb(future):
            if timer:
                timer.cancel()
            if future.cancelled():
                self.logger(f'The "{coro.__name__}" coroutine was cancelled.')
                return
            callback(future.exception() or future.result())
        future.add_done_callback(done_cb)
        timer = run_coro(asyncio.sleep(timeout), loop=self.loop)
        def timeout_cb(timer):
            if timer.cancelled():
                return
            if not future.done():
                future.cancel()
                callback(asyncio.TimeoutError(f"{coro.__name__} timed out after {timeout} seconds"))
        timer.add_done_callback(timeout_cb)

    # PUBLIC -- Synchronous RPC methods
    #   The below methods are thread safe and can be called from any thread EXCEPT this class's thread.
    #   They will synchronously block until a result is ready.
    #   They will raise asyncio.TimeoutError if the timeout is exceeded
    #   May also raise whatever exception the corresponding async functions above raised (such as ConnectionRefusedError, etc).
    #
    # UTILITY service (Synchronous/Blocking-style methods)
    def Version_Sync(self, useragent: str, version: str, *, timeout=10.0, dont_raise_on_error=None, trace=None) -> Tuple[str, str]:
        return self._do_sync(self.Version(useragent, version), timeout=timeout, dont_raise_on_error=dont_raise_on_error, trace=trace)
    def Banner_Sync(self, *, timeout=10.0, dont_raise_on_error=None, trace=None) -> str:
        return self._do_sync(self.Banner(), timeout=timeout, dont_raise_on_error=dont_raise_on_error, trace=trace)
    def Ping_Sync(self, *, timeout=10.0, dont_raise_on_error=None, trace=None) -> float:
        return self._do_sync(self.Ping(), timeout=timeout, dont_raise_on_error=dont_raise_on_error, trace=trace)
    def DonationAddress_Sync(self, *, timeout=10.0, dont_raise_on_error=None, trace=None) -> str:
        return self._do_sync(self.DonationAddress(), timeout=timeout, dont_raise_on_error=dont_raise_on_error, trace=trace)
    # TRANSACTION service (Synchronous/Blocking-style methods)
    def Transaction_Sync(self, tx_hash: bytes, *, timeout=10.0, dont_raise_on_error=None, trace=None) -> dict:
        return self._do_sync(self.Transaction(tx_hash), timeout=timeout, dont_raise_on_error=dont_raise_on_error, trace=trace)
    # testing...
    def Sleeper_Sync(self, delay=5.0, *, timeout=10.0, trace=None) -> int:
        return self._do_sync(self.Sleeper(delay), timeout=timeout, trace=trace)

    # PUBLIC -- Callback-style RPC methods
    #   The below methods are thread safe and can be called from any thread including this class's thread.
    #   Callbacks will execute in the thread context of this class's thread, and will be passed the result
    #   of the corresponding _RpcXXX() function above. If the _RPC function raised, the callee will
    #   be passed the exception encountered.  If the _RPC function timed out, asyncio.TimeoutError will be passed.
    #   Otherwise the result of the RPC call will be passed to `callback`.
    #
    # UTILITY service (Callback-style methods)
    def Version_CB(self, callback: Callable[[Union[Tuple[str, str], Exception]], None],
                   useragent: str, version: str, *, timeout=10.0):
        self._do_cb(self.Version(useragent, version), callback, timeout=timeout)
    def Banner_CB(self, callback: Callable[[Union[str, Exception]], None], *, timeout=10.0):
        self._do_cb(self.Banner(), callback, timeout=timeout)
    def Ping_CB(self, callback: Callable[[Union[float, Exception]], None], *, timeout=10.0):
        self._do_cb(self.Ping(), callback, timeout=timeout)
    def DonationAddress_CB(self, callback: Callable[[Union[str, Exception]], None], *, timeout=10.0):
        self._do_cb(self.DonationAddress(), callback, timeout=timeout)
    # TRANSACTION service (Callback-style methods)
    def Transaction_CB(self, callback: Callable[[Union[dict, Exception]], None],
                       tx_hash: bytes, *, timeout=10.0):
        self._do_cb(self.Transaction(tx_hash), callback, timeout=timeout)
    # testing...
    def Sleeper_CB(self, callback: Callable[[Union[int, Exception]], None], delay=5.0, *, timeout=10.0):
        self._do_cb(self.Sleeper(delay), callback, timeout=timeout)

def main():
    import argparse

    host = Client.DEFAULT_HOST
    port = Client.DEFAULT_PORT

    parser = argparse.ArgumentParser(prog="client.py", description="Test Photon Python Client")
    parser.add_argument('host', nargs='?', help='host:port to connect to')
    parser.add_argument('-t', nargs=1, metavar='file', help="Transaction file to read for the transaction.Transaction test")

    args = parser.parse_args()

    try:
        _host, _port = args.host.rsplit(':', 1)
        _port = int(_port)
        host, port = _host, _port
        del _host, _port
    except:
        if args.host:
            print("Failed to parse host:port")
            parser.print_help()
            sys.exit(1)
        print(f"Note: Will connect to default {host}:{port}, specify a HOST:PORT on the command-line to override.")
    else:
        print(f"Command-line host:port specified: \"{host}:{port}\"")

    txns = []
    if args.t:
        fn = args.t[0]
        print(f"Reading transaction id's from '{fn}' ...")
        line_ctr = 0
        try:
            with open(fn, "rt") as f:
                for line in f:
                    txid = bytes.fromhex(line.strip()).hex()
                    line_ctr += 1
                    assert(len(txid) == 64), "TXID must be 32 bytes"
                    txns.append(txid)
        except OSError as e:
            print(f"File error on '{fn}': {e}")
            sys.exit(1)
        except Exception as e:
            print("Error parsing txid on line:", line_ctr, ",", e)
            sys.exit(1)
        else:
            print(f"Read {len(txns)} txid's from file")


    c = Client(host, port, logger=print, dont_raise_on_error=True, trace=True)
    print(repr(threading.current_thread()))
    c.start()
    c.Version_Sync("Photon PyClient", "0.1.0")
    c.Banner_Sync()
    c.Ping_Sync()
    c.DonationAddress_Sync()

    test_txns(c, txns=txns)

    # Testing interrupting an in-progress operation
    def got_result(x):
        print("CB Got result:", repr(x))
        print("Thread:", repr(threading.current_thread()))
        # test scheduling another callback from within the callback.. works!
        c.Sleeper_CB(print, 1.0, timeout=0.5)
    c.Sleeper_CB(got_result, 1.0, timeout=3.0)
    time.sleep(2.5)
    c.stop()

def test_txns(c: Client, *, txns=None):
    txns = txns or (
           [ 'a3e0b7558e67f5cadd4a3166912cbf6f930044124358ef3a9afd885ac391625d',  # <-- early txid block <200
             'f399cb6c5bed7bb36d44f361c29dd5ecf12deba163470d8835b7ba0c4ed8aebd',  # "
             '41b48c64cba68c21e0b7b37f589408823f112bb7cbccef4aece29df25347ffb4',  # "
             '71cbe112176d6dc40490dde588798bd75de80133438016a0c05754d74ee1925a',  # "
             '6a8226ad9980693ffbf41a15a1118fb73a9f68cda9c0b9951490bd03cb70d1d8',  # "
             '1abd5b2a5ef41b5636b18216518b77b854ac26b9923ec99c272dbd7236133176',  # <-- block 237000
             '414318b20e42ecd9816610f95ae926dc5d9a5afbaff934277b75572f35197843',  # ""
             '2ee90107999ad508097a8f9f804e78ada865c577851791fbdba469a58d4dabc9',  # <-- block 284784
             '4828db57fc85e46f322ef760a017e054dfef467374f1887e90ea9ea74f4b5a85',  # <-- block 391744
             '123a33e29879f7bef161b5059741af0ce6c594b7f04c08ef171612767950206c',  # <-- later txids height >500,000 (needs full synch)
             '1caa134689ced460a154aed0357483462dab324da66bc927fbd6a1312adebee5',  # "
             '06d29d7fbcbceed8bcb878d7a352a65123e515a7ad532a731a735ea040c7fdf4',  # "
             'dfdd9fce1ea51b061e60592d66ab57d14e8e6f94deb2235e719ac0d2b9d2dc7f',  # "
             'd8d170cb03bd901495b9c0a9cd689f3ab78f11a1151af4fb3f698099ad26826a',  # <-- reversed txid (should throw error NOT FOUND)
             'b33ff00db33f00d00000000000000000000000000000000000000000deadb33f',  # <-- invalid txid (should always throw NOT FOUND)
            ]
    )

    limit = 0  # play with this to control how much you spam the server
    if limit and len(txns) > limit:
        txns = txns[:limit]

    #tests = { 'synch', 'asynch' }
    tests = { 'asynch' }
    #tests = { 'synch' }

    print("Performing tests:", ', '.join(f"'{t}'" for t in tests), "on", len(txns), "txids ...")

    if 'synch' in tests:
        found = 0
        bad = 0
        notfound = 0
        for tx_hash_hex in txns:
            tx_hash = bytes.fromhex(tx_hash_hex)
            try:
                txd = c.Transaction_Sync(tx_hash, trace=False, dont_raise_on_error=False) # synch req
            except GRPCError as e:
                if e.status == Status.NOT_FOUND:
                    notfound += 1
                    print("NOT FOUND", e.message)
                    continue
                else:
                    raise e  # unknown/unexpected error status
            except (OSError, StreamTerminatedError, ProtocolError) as e:
                # example of what errrors grpclib can raise on comm./network error
                print("Low-level I/O Error:", repr(e))
                break
            except MalformedResponse as e:
                # exmple of potential checks we do within the coroutine and how to handle them when they fail
                bad += 1
                print("TxID mismatch:", *e.args[1:])
            else:
                found += 1
                print("TxID ok; bytes:", len(txd['raw_tx']), "height:", txd['merkle']['block_height'] or '??')
        print(f"Synchronous test complete on {len(txns)} txns (found: {found}, notfound: {notfound}, bad: {bad})")

    if 'asynch' in tests:
        timeout = len(txns)
        found = 0
        bad = 0
        notfound = 0
        # Now, do it async. with a callback
        q = queue.Queue()
        for tx_hash_hex in txns:
            tx_hash = bytes.fromhex(tx_hash_hex)
            def callback(res, *, tx_hash=tx_hash):
                q.put((tx_hash, res))
            c.Transaction_CB(callback, tx_hash, timeout=timeout) # asynch req
        print("Submitted", len(txns), "asynch. callbacks ...")
        ctr = 0
        while ctr < len(txns):
            try:
                tx_hash, res = q.get(timeout=timeout)
                tx_hash_hex = tx_hash.hex()
                ctr += 1
            except queue.Empty:
                print(f"Timeout waiting for asynch. txns, aborting early, got {ctr}/{len(txns)} txns")
                return
            if isinstance(res, Exception):
                e = res
                if isinstance(e, GRPCError):
                    if e.status == Status.NOT_FOUND:
                        print("NOT FOUND", tx_hash_hex[:8], e.message)
                        notfound += 1
                    else:
                        raise e  # unknown/unexpected error status
                elif isinstance(e, (OSError, StreamTerminatedError, ProtocolError)):
                    # example of what errrors grpclib can raise on comm./network error
                    print("Low-level I/O Error:", tx_hash_hex[:8], repr(e))
                    return
                elif isinstance(e, MalformedResponse):
                    # exmple of potential checks we do within the coroutine and how to handle them when they fail
                    print("TxID mismatch:", *e.args[1:])
                    bad += 1
                else:
                    raise e
            else:
                txd = res
                found += 1
                print("TxID", tx_hash_hex[:8], "ok; bytes:", len(txd['raw_tx']), "height:", txd['merkle']['block_height'] or '??')
        assert ctr == len(txns)
        print(f"Got {ctr} txn replies asynchronously (found: {found}, notfound: {notfound}, bad: {bad}), yay!")

if __name__ == "__main__":
    main()
