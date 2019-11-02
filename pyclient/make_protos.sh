#!/bin/bash

set -e

(cd protos && ln -sf ../../proto/*.proto .)

for a in protos/*.proto; do 
	echo "Making $a ..."
	python3 -m grpc_tools.protoc -I. --python_out=. --python_grpc_out=. $a 
done

echo "Removing tmp .proto files from protos/ ..."
rm -vf protos/*.proto

