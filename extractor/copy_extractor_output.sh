#!/usr/bin/env bash

cd "$(dirname "$0")" || return

rm ../extracted/*.json
cp run/valence_extractor_output/*.json ../extracted/
