# Ecast

Quick and dirty broadcasting.

A weekend project to allow easy broadcasting of future projects. This tool takes
input from `stdin` and broadcasts it to multiple kinds of outputs.

Currently the following are supported:
- Output to `stdout`
- Log to any number of files
- Output to TCP clients
- Output to GRPC clients, with optional CSV splitting

# Why?

There are times when I'm hacking on a hardware project that I'll end up with a
console outputting CSV data. This tool allows me to broadcast the data to other
places on my network for live processing, logging, etc... I'm not just stuck on
whatever machine the hardware is communicating on.
