#!/bin/bash

echo "Running guest bash entrypoint script at $(date)"

echo "Executing Python script in guest"

python3 test.py

echo "Python script executed"
