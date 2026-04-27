#!/bin/bash

outDir=$(pwd)/src/schema
rm -rf "$outDir" 2>/dev/null
mkdir -p $outDir || true
protoDir=$(pwd)/proto
sourceDir=$(pwd)/../proto

echo "Compiling protobuf grpc NodeJS Typescript definitions at $sourceDir -> $protoDir ..."
rm -rf "$protoDir" 2>/dev/null
cp -r "$sourceDir" "$protoDir"
pnpm exec buf generate
