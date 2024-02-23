#!/bin/bash

trunk build --release

cargo clean

cp vercel.json dist/vercel.json

vercel deploy --prod
