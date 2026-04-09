#! /bin/bash

set -e
outDir="$1"
if [ -z "$outDir" ]; then
    outDir=`pwd`/dist
fi

rootDir=`dirname $(dirname $0)`
mkdir $outDir &>/dev/null || true
sourceDir=$pwd/schema/proto

executeGenCmd() {
  ./node_modules/.bin/protoc \
  --proto_path=$sourceDir \
		$sourceDir/devlog/models/*.proto \
		$sourceDir/devlog/devblog/rpc/*.proto \
		$sourceDir/devlog/devblog/models/*.proto \
		"$@"
}

echo "Generating schema for web grpc typescript... $rootDir"
executeGenCmd \
    --plugin=protoc-gen-ts=./node_modules/.bin/protoc-gen-ts \
    --ts_out=service=$outDir
