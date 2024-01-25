#!/usr/bin/env bash

export network=$4

if [ -z $network ]
then
    export network="testnet"
fi

echo "deploying $1 from $3 to $2 on $network"

ckb-cli --url $2 deploy gen-txs --from-address $3 --fee-rate 1000 --deployment-config ./toml/$1.toml \
    --info-file ./$1.json --migration-dir ./migration/$network/$1 --sign-now

echo "ckb transacion file '$1.json' has generated"

ckb-cli --url $2 deploy apply-txs --info-file ./$1.json --migration-dir ./migration/$network/$1

rm ./$1.json

echo "deployment finished"

# usage: ./deploy.sh spore https://testnet.ckbapp.dev/ ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq28phxutezqvjgfv5q38gn5kwek4m9km3cmajeqs
