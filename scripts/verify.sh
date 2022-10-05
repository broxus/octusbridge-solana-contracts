#!/usr/bin/env bash

function print_help() {
  echo 'Usage: verify.sh [OPTIONS]'
  echo ''
  echo 'Options:'
  echo '  -h,--help         Print this help message and exit'
  echo '  -a,--address      Deployed program address to verify'
  echo '  -b,--binary       Program to verify'
  echo '  -u,--url          Solana RPC url'
}

while [[ $# -gt 0 ]]; do
  key="$1"
  case $key in
      -h|--help)
        print_help
        exit 0
      ;;
      -a|--address)
        address="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected program address'
          echo ''
          print_help
          exit 1
        fi
      ;;
      -b|--binary)
        binary="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected binary'
          echo ''
          print_help
          exit 1
        fi
      ;;
      -u|--url)
        url="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected url'
          echo ''
          print_help
          exit 1
        fi
      ;;
      *) # unknown option
        echo 'ERROR: Unknown option'
        echo ''
        print_help
        exit 1
      ;;
  esac
done

if [ -z "$url" ]; then
  url=https://api.mainnet-beta.solana.com
fi

solana program dump "$address" to_verify.so --url $url
truncate --size="$(ls -nl "$binary" | awk '{print $5}')" to_verify.so

left=$(sha256sum to_verify.so | cut -d' ' -f1 | paste -s -)
right=$(sha256sum "$binary" | cut -d' ' -f1 | paste -s -)

rm to_verify.so

if [ "$left" == "$right" ]; then
  echo "Verification successfully passed"
else
  echo "Verification failed"
  exit 1
fi
