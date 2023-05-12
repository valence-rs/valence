#!/usr/bin/env bash

cd "$(dirname "$0")" || return

rm ../extracted/*
cp run/valence_extractor_output/* ../extracted/
